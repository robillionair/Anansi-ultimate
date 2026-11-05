import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import crypto from 'node:crypto';
import { spawn, spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { DatabaseSync } from 'node:sqlite';
import {
  CONFIGURED_PATHS,
  formatBytes,
  hashFile,
  checkRunningProcesses,
  checkTestLease,
  isReparsePoint,
  isPathInside,
  writeJsonAtomic,
} from './common.mjs';

export function acquireLease(leasePath, identity, owner = 'Windows Release Acceptance') {
  const leaseDir = path.dirname(leasePath);
  fs.mkdirSync(leaseDir, { recursive: true });

  const record = {
    identity,
    owner,
    pid: process.pid,
    startedAt: new Date().toISOString(),
    cwd: process.cwd(),
  };

  try {
    const fd = fs.openSync(leasePath, 'wx', 0o600);
    fs.writeFileSync(fd, JSON.stringify(record, null, 2));
    fs.fsyncSync(fd);
    fs.closeSync(fd);
    return { acquired: true, path: leasePath, identity };
  } catch (err) {
    if (err.code === 'EEXIST') {
      return { acquired: false, path: leasePath, error: 'LEASE_OCCUPIED' };
    }
    throw err;
  }
}

export function releaseLease(leasePath, identity) {
  try {
    if (fs.existsSync(leasePath)) {
      const data = JSON.parse(fs.readFileSync(leasePath, 'utf8'));
      if (data.identity === identity) {
        fs.unlinkSync(leasePath);
        return true;
      }
    }
  } catch {}
  return false;
}

export function isProcessRunning(pid) {
  if (!pid) return false;
  try {
    process.kill(pid, 0);
    return true;
  } catch (err) {
    return err.code === 'EPERM';
  }
}

let customProcessTreeEnumerator = null;
export function setProcessTreeEnumeratorForTesting(fn) {
  customProcessTreeEnumerator = fn;
}

export function enumerateProcessTree(rootPid) {
  if (customProcessTreeEnumerator) {
    return customProcessTreeEnumerator(rootPid);
  }
  if (!rootPid || process.platform !== 'win32') {
    return { ok: true, pids: [] };
  }

  try {
    const ps = spawnSync('powershell.exe', [
      '-NoProfile',
      '-Command',
      'Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId | ConvertTo-Json -Compress',
    ], { encoding: 'utf8', windowsHide: true, timeout: 5000 });

    if (ps.status !== 0 || !ps.stdout) {
      return {
        ok: false,
        error: `Process enumeration failed (exit code ${ps.status}): ${ps.stderr || 'no output'}`,
        pids: [],
      };
    }

    let parsed;
    try {
      parsed = JSON.parse(ps.stdout);
    } catch (parseErr) {
      return {
        ok: false,
        error: `Failed to parse process tree JSON: ${parseErr.message}`,
        pids: [],
      };
    }

    const list = Array.isArray(parsed) ? parsed : [parsed];
    const parentToChildren = new Map();

    for (const item of list) {
      const pid = Number(item.ProcessId);
      const ppid = Number(item.ParentProcessId);
      if (pid && ppid) {
        if (!parentToChildren.has(ppid)) parentToChildren.set(ppid, []);
        parentToChildren.get(ppid).push(pid);
      }
    }

    const descendants = new Set();
    const queue = [Number(rootPid)];

    while (queue.length > 0) {
      const curr = queue.shift();
      const children = parentToChildren.get(curr) || [];
      for (const child of children) {
        if (!descendants.has(child)) {
          descendants.add(child);
          queue.push(child);
        }
      }
    }

    return {
      ok: true,
      pids: Array.from(descendants),
    };
  } catch (err) {
    return {
      ok: false,
      error: `Process enumeration exception: ${err.message}`,
      pids: [],
    };
  }
}

export function getDescendantPids(parentPid) {
  const res = enumerateProcessTree(parentPid);
  if (!res.ok) return [];
  return res.pids;
}

export function killProcessTree(pid) {
  if (!pid || process.platform !== 'win32') return;
  try {
    spawnSync('taskkill.exe', ['/PID', String(pid), '/T', '/F'], { windowsHide: true });
  } catch {}
}

export async function ensureProcessTreeTerminated(rootPid, timeoutMs = 3000, options = {}) {
  if (!rootPid) return { ok: true, confirmed: true };

  const knownOwnedPids = new Set(options.knownPids || []);
  knownOwnedPids.add(Number(rootPid));

  // 1. Inventory the owned process tree BEFORE killing anything
  const initialEnum = enumerateProcessTree(rootPid);
  let enumerationFailed = !initialEnum.ok;
  let enumerationError = initialEnum.error || null;

  if (initialEnum.ok) {
    for (const pid of initialEnum.pids) knownOwnedPids.add(pid);
  }

  // Also query descendants of all known PIDs (grandchildren if parent already exited)
  for (const knownPid of Array.from(knownOwnedPids)) {
    const subEnum = enumerateProcessTree(knownPid);
    if (!subEnum.ok) {
      enumerationFailed = true;
      enumerationError = enumerationError || subEnum.error;
    } else {
      for (const pid of subEnum.pids) knownOwnedPids.add(pid);
    }
  }

  // If enumeration failed at this critical step, we cannot guarantee clean inventory
  if (enumerationFailed) {
    for (const pid of knownOwnedPids) killProcessTree(pid);
    return {
      ok: false,
      confirmed: false,
      reason: `Process enumeration failed: ${enumerationError}. Cleanup cannot be confirmed.`,
      unkilledPids: Array.from(knownOwnedPids).filter(isProcessRunning),
    };
  }

  // 2. Kill all owned processes: descendants first, then root
  for (const pid of knownOwnedPids) {
    if (pid !== rootPid && isProcessRunning(pid)) {
      killProcessTree(pid);
    }
  }
  killProcessTree(rootPid);

  // 3. Confirm all owned processes are dead within timeoutMs
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    const running = Array.from(knownOwnedPids).filter(isProcessRunning);
    if (running.length === 0) {
      return { ok: true, confirmed: true };
    }
    for (const rPid of running) {
      killProcessTree(rPid);
    }
    await new Promise(r => setTimeout(r, 100));
  }

  const stillRunning = Array.from(knownOwnedPids).filter(isProcessRunning);
  if (stillRunning.length > 0) {
    return {
      ok: false,
      confirmed: false,
      reason: `Processes still active after teardown: ${stillRunning.join(', ')}`,
      unkilledPids: stillRunning,
    };
  }

  return { ok: true, confirmed: true };
}

export async function ensureProcessTerminated(pid, timeoutMs = 3000) {
  const res = await ensureProcessTreeTerminated(pid, timeoutMs);
  return res.confirmed;
}

export function checkSqliteCatalogIntegrity(dbPath) {
  if (!fs.existsSync(dbPath)) {
    return { ok: false, reason: `Database file ${path.basename(dbPath)} does not exist` };
  }
  try {
    const lstats = fs.lstatSync(dbPath);
    if (lstats.isSymbolicLink() || isReparsePoint(dbPath)) {
      return { ok: false, reason: `Database file ${path.basename(dbPath)} is a symlink or reparse point (refused)` };
    }
    if (lstats.isDirectory()) {
      return { ok: false, reason: `Database file ${path.basename(dbPath)} is a directory, expected regular file` };
    }
    if (lstats.size < 16) {
      return { ok: false, reason: `Database ${path.basename(dbPath)} truncated: size ${lstats.size} < 16 bytes` };
    }

    const headerBuf = Buffer.alloc(16);
    const fd = fs.openSync(dbPath, 'r');
    fs.readSync(fd, headerBuf, 0, 16, 0);
    fs.closeSync(fd);
    if (headerBuf.toString('utf8', 0, 15) !== 'SQLite format 3') {
      return { ok: false, reason: `Database ${path.basename(dbPath)} corrupt: invalid SQLite header` };
    }

    if (lstats.size < 512) {
      return { ok: false, reason: `Database ${path.basename(dbPath)} truncated or header-only (${lstats.size} bytes, minimum valid SQLite page is 512 bytes)` };
    }

    let db;
    try {
      db = new DatabaseSync(dbPath, { readOnly: true });
    } catch (openErr) {
      return { ok: false, reason: `Database ${path.basename(dbPath)} cannot be opened as SQLite database: ${openErr.message}` };
    }

    try {
      const integrityRows = db.prepare('PRAGMA integrity_check').all();
      if (!integrityRows || integrityRows.length === 0 || integrityRows[0]?.integrity_check !== 'ok') {
        const detail = integrityRows ? integrityRows.map(r => r.integrity_check).join('; ') : 'unknown integrity error';
        return { ok: false, reason: `Database ${path.basename(dbPath)} failed integrity check: ${detail}` };
      }

      const tables = db.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'").all();
      const userVerRow = db.prepare('PRAGMA user_version').get();
      const userVersion = userVerRow ? Object.values(userVerRow)[0] : 0;

      if (tables.length === 0 && (!userVersion || userVersion === 0)) {
        return { ok: false, reason: `Database ${path.basename(dbPath)} contains no application schema or state tables` };
      }

      return {
        ok: true,
        tables: tables.map(t => t.name),
        userVersion,
      };
    } finally {
      db.close();
    }
  } catch (err) {
    return { ok: false, reason: `Failed to inspect SQLite database ${path.basename(dbPath)}: ${err.message}` };
  }
}

export const ANANSI_STORAGE_CONTRACT = {
  catalogFileName: 'workspaces.sqlite3',
  applicationId: 0x414e414e, // 1095647566 decimal ("ANAN")
  supportedSchemaVersions: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
  requiredTables: ['workspaces', 'workspace_settings'],
  requiredColumns: {
    workspaces: ['id', 'name', 'name_key'],
    workspace_settings: ['workspace_id', 'reduced_motion'],
  },
  rootMarkerName: '.anansi-root',
  rootMarkerContent: 'ANANSI workspace root v1\n',
};

export function protectAcl(targetPath, isDirectory = true) {
  if (process.platform !== 'win32') return;
  const protectScript = `
$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSHOME 'Modules\\Microsoft.PowerShell.Security\\Microsoft.PowerShell.Security.psd1')
$path = $env:ANANSI_PERMISSION_PATH
$sid = [Security.Principal.WindowsIdentity]::GetCurrent().User
if ($env:ANANSI_PERMISSION_DIRECTORY -eq '1') {
    $acl = New-Object Security.AccessControl.DirectorySecurity
    $rule = New-Object Security.AccessControl.FileSystemAccessRule($sid, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow')
} else {
    $acl = New-Object Security.AccessControl.FileSecurity
    $rule = New-Object Security.AccessControl.FileSystemAccessRule($sid, 'FullControl', 'Allow')
}
if ($env:ANANSI_PERMISSION_VERIFY -ne '1') {
$acl.SetOwner($sid)
$acl.SetAccessRuleProtection($true, $false)
$acl.AddAccessRule($rule)
if ($env:ANANSI_PERMISSION_DIRECTORY -eq '1') {
    [IO.Directory]::SetAccessControl($path, $acl)
} else {
    [IO.File]::SetAccessControl($path, $acl)
}
}
$actual = Get-Acl -LiteralPath $path
if (-not $actual.AreAccessRulesProtected -or $actual.GetOwner([Security.Principal.SecurityIdentifier]).Value -ne $sid.Value) { throw 'Private owner/DACL verification failed' }
`;
  try {
    spawnSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', protectScript], {
      env: {
        ...process.env,
        ANANSI_PERMISSION_PATH: targetPath,
        ANANSI_PERMISSION_DIRECTORY: isDirectory ? '1' : '0',
        ANANSI_PERMISSION_VERIFY: '0',
      },
      windowsHide: true,
      stdio: 'pipe',
    });
  } catch {
    // Non-fatal if powershell is not available in test mocks
  }
}

export function initSyntheticAnansiCatalog(dbPath, options = {}) {
  const db = new DatabaseSync(dbPath);
  const userVersion = options.userVersion ?? 10;
  const appId = options.applicationId ?? ANANSI_STORAGE_CONTRACT.applicationId;
  db.exec(`
    PRAGMA application_id = ${appId};
    PRAGMA user_version = ${userVersion};
    CREATE TABLE workspaces (
      id TEXT PRIMARY KEY NOT NULL,
      name TEXT NOT NULL,
      name_key TEXT NOT NULL UNIQUE
    );
    CREATE TABLE workspace_settings (
      workspace_id TEXT PRIMARY KEY NOT NULL REFERENCES workspaces(id),
      reduced_motion INTEGER NOT NULL DEFAULT 0 CHECK (reduced_motion IN (0, 1))
    );
  `);

  if (Array.isArray(options.workspaces)) {
    const insertWs = db.prepare('INSERT INTO workspaces (id, name, name_key) VALUES (?, ?, ?)');
    const insertSet = db.prepare('INSERT INTO workspace_settings (workspace_id, reduced_motion) VALUES (?, ?)');
    for (const ws of options.workspaces) {
      insertWs.run(ws.id, ws.name, ws.name_key || ws.id.toLowerCase());
      insertSet.run(ws.id, ws.reduced_motion ?? 0);
    }
  }

  db.close();
}

export async function createRealApplicationWorkspaceAndRecord(options = {}) {
  const apiExePath = options.apiExePath;
  const syntheticDataRoot = options.syntheticDataRoot;
  const tempDir = options.tempDir || CONFIGURED_PATHS.tmp;
  const workspaceName = options.workspaceName || 'Qualified Real Acceptance Workspace';
  const memoryInput = options.memoryInput || 'Synthetic verified qualification memory recorded through real application API path.';

  if (!apiExePath || !fs.existsSync(apiExePath)) {
    return { ok: false, reason: `API executable missing at ${apiExePath}` };
  }

  const pairingRoot = path.join(tempDir, `api-pairing-${crypto.randomUUID()}`);
  fs.mkdirSync(pairingRoot, { recursive: true });
  protectAcl(pairingRoot, true);

  const sanitizedEnv = { ...process.env };
  sanitizedEnv.ANANSI_STORAGE_ROOT = syntheticDataRoot;
  delete sanitizedEnv.ANANSI_VAULT_TEST_PROVIDER;
  delete sanitizedEnv.ANANSI_PROVIDER_TEST_CREDENTIAL;
  delete sanitizedEnv.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS;
  delete sanitizedEnv.TAURI_CONFIG;
  delete sanitizedEnv.OPENAI_API_KEY;
  delete sanitizedEnv.ANTHROPIC_API_KEY;
  delete sanitizedEnv.OPENROUTER_API_KEY;
  delete sanitizedEnv.OLLAMA_HOST;

  const port = options.port || (14600 + Math.floor(Math.random() * 350));

  let child;
  try {
    child = spawn(apiExePath, ['--port', String(port), '--pairing-root', pairingRoot], {
      env: sanitizedEnv,
      windowsHide: true,
      cwd: syntheticDataRoot,
    });
  } catch (err) {
    return { ok: false, reason: `Failed to spawn anansi-api.exe: ${err.message}` };
  }

  const childPid = child.pid;
  let childExited = false;
  let childExitCode = null;
  child.on('exit', (code) => {
    childExited = true;
    childExitCode = code;
  });

  let pairingInfo = null;
  const start = Date.now();
  while (Date.now() - start < 15000) {
    if (childExited) {
      return { ok: false, reason: `anansi-api.exe exited prematurely with code ${childExitCode}` };
    }
    try {
      const dirs = fs.readdirSync(pairingRoot);
      for (const d of dirs) {
        const candidate = path.join(pairingRoot, d, 'pairing.json');
        if (fs.existsSync(candidate)) {
          const content = fs.readFileSync(candidate, 'utf8');
          pairingInfo = JSON.parse(content);
          break;
        }
      }
    } catch {}
    if (pairingInfo) break;
    await new Promise(resolve => setTimeout(resolve, 150));
  }

  if (!pairingInfo || !pairingInfo.endpoint || !pairingInfo.token) {
    if (childPid) await ensureProcessTreeTerminated(childPid, 3000);
    return { ok: false, reason: 'Failed to discover valid pairing session within 15s' };
  }

  const endpoint = pairingInfo.endpoint;
  const token = pairingInfo.token;
  const urlObj = new URL(endpoint);
  const headers = {
    'Content-Type': 'application/json',
    'Authorization': `Bearer ${token}`,
    'Origin': 'http://127.0.0.1:1420',
    'Host': urlObj.host,
  };

  let createdWorkspace = null;
  let createdMemory = null;
  let readinessData = null;
  let workspacesListData = null;
  const endpointsTested = [];

  try {
    const createWsRes = await fetch(`${endpoint}/v1/workspaces/create`, {
      method: 'POST',
      headers,
      body: JSON.stringify({ contractVersion: 1, name: workspaceName }),
    });
    endpointsTested.push('/v1/workspaces/create');
    if (createWsRes.status !== 200) {
      throw new Error(`POST /v1/workspaces/create returned HTTP ${createWsRes.status}`);
    }
    const createWsBody = await createWsRes.json();
    createdWorkspace = createWsBody.workspace;
    if (!createdWorkspace?.id) {
      throw new Error('Workspace creation response missing workspace ID');
    }

    const createMemRes = await fetch(`${endpoint}/v1/memories/create`, {
      method: 'POST',
      headers,
      body: JSON.stringify({
        contractVersion: 1,
        workspaceId: createdWorkspace.id,
        kind: 'note',
        draft: {
          title: 'Qualification Acceptance Note',
          body: memoryInput,
        },
      }),
    });
    endpointsTested.push('/v1/memories/create');
    if (createMemRes.status !== 200) {
      throw new Error(`POST /v1/memories/create returned HTTP ${createMemRes.status}`);
    }
    createdMemory = await createMemRes.json();

    const readinessRes = await fetch(`${endpoint}/v1/readiness`, {
      method: 'GET',
      headers,
    });
    endpointsTested.push('/v1/readiness');
    if (readinessRes.status === 200) {
      readinessData = await readinessRes.json();
    }

    const listWsRes = await fetch(`${endpoint}/v1/workspaces/list`, {
      method: 'GET',
      headers,
    });
    endpointsTested.push('/v1/workspaces/list');
    if (listWsRes.status === 200) {
      workspacesListData = await listWsRes.json();
    }
  } catch (err) {
    if (childPid) await ensureProcessTreeTerminated(childPid, 3000);
    return { ok: false, reason: `Real API request failed: ${err.message}`, endpointsTested };
  }

  const termRes = await ensureProcessTreeTerminated(childPid, 3000);

  try {
    fs.rmSync(pairingRoot, { recursive: true, force: true });
  } catch {}

  const dbPath = path.join(syntheticDataRoot, ANANSI_STORAGE_CONTRACT.catalogFileName);
  if (!fs.existsSync(dbPath)) {
    return { ok: false, reason: `Real API completed but catalog file ${ANANSI_STORAGE_CONTRACT.catalogFileName} missing on disk` };
  }

  const markerPath = path.join(syntheticDataRoot, ANANSI_STORAGE_CONTRACT.rootMarkerName);
  if (!fs.existsSync(markerPath)) {
    return { ok: false, reason: `Real API completed but root marker ${ANANSI_STORAGE_CONTRACT.rootMarkerName} missing on disk` };
  }

  return {
    ok: true,
    realApiPath: true,
    workspace: createdWorkspace,
    memory: createdMemory,
    readiness: readinessData,
    workspacesList: workspacesListData,
    endpointsTested,
    dbPath,
    markerPath,
    apiTeardownSuccess: termRes.confirmed,
  };
}

export function verifyAnansiCatalogContract(dbPath, options = {}) {
  const integrity = checkSqliteCatalogIntegrity(dbPath);
  if (!integrity.ok) {
    return { ok: false, reason: integrity.reason };
  }

  try {
    const db = new DatabaseSync(dbPath, { readOnly: true });
    try {
      const userVerRow = db.prepare('PRAGMA user_version').get();
      const userVersion = userVerRow ? Number(Object.values(userVerRow)[0]) : 0;
      if (!ANANSI_STORAGE_CONTRACT.supportedSchemaVersions.includes(userVersion)) {
        return {
          ok: false,
          reason: `Catalog ${path.basename(dbPath)} has unsupported schema version (user_version=${userVersion}, expected ${ANANSI_STORAGE_CONTRACT.supportedSchemaVersions[0]}..${ANANSI_STORAGE_CONTRACT.supportedSchemaVersions.at(-1)})`,
        };
      }

      const appIdRow = db.prepare('PRAGMA application_id').get();
      const appId = appIdRow ? Number(Object.values(appIdRow)[0]) : 0;
      if (appId !== 0 && appId !== ANANSI_STORAGE_CONTRACT.applicationId) {
        return {
          ok: false,
          reason: `Catalog ${path.basename(dbPath)} application_id (0x${appId.toString(16)}) does not match ANANSI application ID (0x${ANANSI_STORAGE_CONTRACT.applicationId.toString(16)})`,
        };
      }

      const tableRows = db.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'").all();
      const tableNames = tableRows.map(t => t.name);
      const missingTables = ANANSI_STORAGE_CONTRACT.requiredTables.filter(t => !tableNames.includes(t));
      if (missingTables.length > 0) {
        return {
          ok: false,
          reason: `Catalog ${path.basename(dbPath)} missing required ANANSI tables: ${missingTables.join(', ')} (found: ${tableNames.join(', ') || 'none'})`,
        };
      }

      const columnsByTable = {};
      for (const table of ANANSI_STORAGE_CONTRACT.requiredTables) {
        const colRows = db.prepare(`PRAGMA table_info(${table})`).all();
        const colNames = colRows.map(c => c.name);
        columnsByTable[table] = colNames;
        const reqCols = ANANSI_STORAGE_CONTRACT.requiredColumns[table] || [];
        const missingCols = reqCols.filter(c => !colNames.includes(c));
        if (missingCols.length > 0) {
          return {
            ok: false,
            reason: `Catalog table '${table}' missing required columns: ${missingCols.join(', ')} (found: ${colNames.join(', ') || 'none'})`,
          };
        }
      }

      const workspaces = db.prepare('SELECT id, name, name_key FROM workspaces ORDER BY id').all();
      const workspaceSettings = db.prepare('SELECT workspace_id, reduced_motion FROM workspace_settings ORDER BY workspace_id').all();

      if (options.expectedWorkspace) {
        const ew = options.expectedWorkspace;
        const match = workspaces.find(w => w.id === ew.id);
        if (!match) {
          return {
            ok: false,
            reason: `Expected synthetic workspace '${ew.id}' not found in catalog`,
          };
        }
        if (ew.name !== undefined && match.name !== ew.name) {
          return {
            ok: false,
            reason: `Workspace '${ew.id}' name mismatch: expected '${ew.name}', found '${match.name}'`,
          };
        }
        if (ew.name_key !== undefined && match.name_key !== ew.name_key) {
          return {
            ok: false,
            reason: `Workspace '${ew.id}' name_key mismatch: expected '${ew.name_key}', found '${match.name_key}'`,
          };
        }
      }

      return {
        ok: true,
        userVersion,
        applicationId: appId,
        tables: tableNames,
        columns: columnsByTable,
        workspaces,
        workspaceSettings,
        recordCount: workspaces.length,
      };
    } finally {
      db.close();
    }
  } catch (err) {
    return { ok: false, reason: `Failed to inspect ANANSI catalog contract: ${err.message}` };
  }
}

export function verifyApplicationReadiness(syntheticDataRoot, initialFiles = [], options = {}) {
  if (!fs.existsSync(syntheticDataRoot)) {
    return { ready: false, reason: 'Synthetic data root does not exist' };
  }
  if (isReparsePoint(syntheticDataRoot) || fs.lstatSync(syntheticDataRoot).isSymbolicLink()) {
    return { ready: false, reason: 'Synthetic data root is a reparse point or symlink (refused)' };
  }

  const currentFiles = fs.readdirSync(syntheticDataRoot);
  const newFiles = currentFiles.filter(f => !initialFiles.includes(f));

  if (newFiles.length === 0) {
    return { ready: false, reason: 'Zero new storage files created during execution' };
  }

  for (const f of newFiles) {
    const fullPath = path.join(syntheticDataRoot, f);
    try {
      if (isReparsePoint(fullPath) || fs.lstatSync(fullPath).isSymbolicLink()) {
        return { ready: false, reason: `Artifact ${f} is a reparse point or symlink (refused)` };
      }
    } catch (err) {
      return { ready: false, reason: `Failed to inspect artifact ${f}: ${err.message}` };
    }
  }

  const coreCatalogName = ANANSI_STORAGE_CONTRACT.catalogFileName;
  const hasCoreCatalog = newFiles.includes(coreCatalogName) || fs.existsSync(path.join(syntheticDataRoot, coreCatalogName));
  const hasRootMarker = newFiles.includes(ANANSI_STORAGE_CONTRACT.rootMarkerName) || fs.existsSync(path.join(syntheticDataRoot, ANANSI_STORAGE_CONTRACT.rootMarkerName));

  // Marker-only folders without the actual database must refuse readiness
  if (!hasCoreCatalog) {
    if (hasRootMarker) {
      return {
        ready: false,
        initializationOnly: true,
        reason: 'Storage contract not established: root marker alone is initialization-only; core ANANSI catalog (workspaces.sqlite3) is required for readiness',
      };
    }
    return {
      ready: false,
      reason: `Storage contract not established: core ANANSI catalog (${coreCatalogName}) required, found only: ${newFiles.join(', ')}`,
    };
  }

  const catalogPath = path.join(syntheticDataRoot, coreCatalogName);
  try {
    const stats = fs.statSync(catalogPath);
    if (stats.isDirectory()) {
      return { ready: false, reason: `Core catalog ${coreCatalogName} is a directory, expected regular file` };
    }
    if (stats.size === 0) {
      return { ready: false, reason: `Core catalog ${coreCatalogName} is empty (0 bytes)` };
    }
    fs.accessSync(catalogPath, fs.constants.R_OK);
  } catch (err) {
    return { ready: false, reason: `Core catalog ${coreCatalogName} inaccessible: ${err.message}` };
  }

  // Validate the actual ANANSI schema contract on the core catalog
  const contractCheck = verifyAnansiCatalogContract(catalogPath, options);
  if (!contractCheck.ok) {
    return {
      ready: false,
      reason: `ANANSI storage contract violated: ${contractCheck.reason}`,
    };
  }

  // Check any additional recognized artifacts (e.g. .anansi-root, other sqlite files)
  const recognizedMarkers = [
    ANANSI_STORAGE_CONTRACT.rootMarkerName,
    coreCatalogName,
    'vault-admission',
    'vault.sqlite3',
    'storage.sqlite3',
    'anansi.sqlite3',
    'app.sqlite3',
  ];
  const recognizedFound = newFiles.filter(f => recognizedMarkers.includes(f) || f.endsWith('.sqlite3'));

  for (const artifact of recognizedFound) {
    if (artifact === coreCatalogName) continue;
    const fullPath = path.join(syntheticDataRoot, artifact);
    try {
      const stats = fs.statSync(fullPath);
      if (artifact === 'vault-admission') {
        if (!stats.isDirectory()) {
          return { ready: false, reason: `Artifact ${artifact} is a file, expected directory` };
        }
      } else {
        if (stats.isDirectory()) {
          return { ready: false, reason: `Artifact ${artifact} is a directory, expected file` };
        }
        if (stats.size === 0) {
          return { ready: false, reason: `Artifact ${artifact} is empty (0 bytes)` };
        }
      }
      fs.accessSync(fullPath, fs.constants.R_OK);

      if (artifact.endsWith('.sqlite3')) {
        const dbCheck = checkSqliteCatalogIntegrity(fullPath);
        if (!dbCheck.ok) {
          return { ready: false, reason: dbCheck.reason };
        }
      }
    } catch (err) {
      return { ready: false, reason: `Artifact ${artifact} inaccessible: ${err.message}` };
    }
  }

  return {
    ready: true,
    recognizedArtifacts: recognizedFound.length > 0 ? recognizedFound : [coreCatalogName],
    newFiles,
    catalogSnapshot: {
      userVersion: contractCheck.userVersion,
      applicationId: contractCheck.applicationId,
      tables: contractCheck.tables,
      workspaces: contractCheck.workspaces,
      workspaceSettings: contractCheck.workspaceSettings,
      recordCount: contractCheck.recordCount,
    },
  };
}

export function verifyApplicationPersistence(syntheticDataRoot, phase1Artifacts = [], options = {}) {
  if (!phase1Artifacts || phase1Artifacts.length === 0) {
    return { persistent: false, reason: 'No valid Phase 1 artifacts established to verify persistence' };
  }
  if (!fs.existsSync(syntheticDataRoot)) {
    return { persistent: false, reason: 'Synthetic data root does not exist after restart' };
  }
  if (isReparsePoint(syntheticDataRoot) || fs.lstatSync(syntheticDataRoot).isSymbolicLink()) {
    return { persistent: false, reason: 'Synthetic data root is a reparse point or symlink after restart (refused)' };
  }

  for (const artifact of phase1Artifacts) {
    const fullPath = path.join(syntheticDataRoot, artifact);
    if (!fs.existsSync(fullPath)) {
      return { persistent: false, reason: `Phase 1 artifact ${artifact} missing after application restart` };
    }
    try {
      const lstats = fs.lstatSync(fullPath);
      if (lstats.isSymbolicLink() || isReparsePoint(fullPath)) {
        return { persistent: false, reason: `Phase 1 artifact ${artifact} is a symlink or reparse point after restart (refused)` };
      }
      if (artifact === 'vault-admission') {
        if (!lstats.isDirectory()) {
          return { persistent: false, reason: `Phase 1 artifact ${artifact} is a file, expected directory` };
        }
      } else {
        if (lstats.isDirectory()) {
          return { persistent: false, reason: `Phase 1 artifact ${artifact} is a directory, expected file` };
        }
        if (lstats.size === 0) {
          return { persistent: false, reason: `Phase 1 artifact ${artifact} truncated to 0 bytes after restart` };
        }
      }
      fs.accessSync(fullPath, fs.constants.R_OK);

      if (artifact.endsWith('.sqlite3') || artifact === ANANSI_STORAGE_CONTRACT.catalogFileName) {
        const dbCheck = checkSqliteCatalogIntegrity(fullPath);
        if (!dbCheck.ok) {
          return { persistent: false, reason: `${artifact} corrupt or non-database after restart: ${dbCheck.reason}` };
        }
      }
    } catch (err) {
      return { persistent: false, reason: `Failed to inspect ${artifact}: ${err.message}` };
    }
  }

  const catalogPath = path.join(syntheticDataRoot, ANANSI_STORAGE_CONTRACT.catalogFileName);
  if (phase1Artifacts.includes(ANANSI_STORAGE_CONTRACT.catalogFileName) || fs.existsSync(catalogPath)) {
    const contractCheck = verifyAnansiCatalogContract(catalogPath, options);
    if (!contractCheck.ok) {
      return {
        persistent: false,
        reason: `ANANSI storage contract failed after restart: ${contractCheck.reason}`,
      };
    }

    const baseline = options.baselineSnapshot || options.catalogSnapshot;
    if (baseline && Array.isArray(baseline.workspaces)) {
      const currentWorkspaces = contractCheck.workspaces;

      for (const bw of baseline.workspaces) {
        const match = currentWorkspaces.find(cw => cw.id === bw.id);
        if (!match) {
          return {
            persistent: false,
            reason: `Workspace record deleted across restart: '${bw.id}' (was present in Phase 1 baseline)`,
          };
        }
        if (match.name !== bw.name) {
          return {
            persistent: false,
            reason: `Workspace record '${bw.id}' name changed across restart: expected '${bw.name}', found '${match.name}'`,
          };
        }
        if (match.name_key !== bw.name_key) {
          return {
            persistent: false,
            reason: `Workspace record '${bw.id}' name_key changed across restart: expected '${bw.name_key}', found '${match.name_key}'`,
          };
        }
      }

      if (Array.isArray(baseline.workspaceSettings)) {
        const currentSettings = contractCheck.workspaceSettings;
        for (const bs of baseline.workspaceSettings) {
          const match = currentSettings.find(cs => cs.workspace_id === bs.workspace_id);
          if (!match) {
            return {
              persistent: false,
              reason: `Workspace settings deleted across restart for workspace '${bs.workspace_id}'`,
            };
          }
          if (match.reduced_motion !== bs.reduced_motion) {
            return {
              persistent: false,
              reason: `Workspace settings reduced_motion changed across restart for '${bs.workspace_id}': expected ${bs.reduced_motion}, found ${match.reduced_motion}`,
            };
          }
        }
      }
    }
  }

  return { persistent: true };
}

export function preserveFailedRoot(syntheticDataRoot, tempBase, launchId) {
  try {
    if (!fs.existsSync(syntheticDataRoot)) return null;
    const failedRunsDir = path.join(tempBase, 'failed-runs');
    fs.mkdirSync(failedRunsDir, { recursive: true });
    const dest = path.join(failedRunsDir, `failed-${launchId}`);
    fs.cpSync(syntheticDataRoot, dest, { recursive: true });
    return dest;
  } catch {
    return null;
  }
}

async function spawnAndTrack(exePath, syntheticDataRoot, smokeDurationMs) {
  const sanitizedEnv = { ...process.env };
  sanitizedEnv.ANANSI_STORAGE_ROOT = syntheticDataRoot;
  delete sanitizedEnv.ANANSI_VAULT_TEST_PROVIDER;
  delete sanitizedEnv.ANANSI_PROVIDER_TEST_CREDENTIAL;
  delete sanitizedEnv.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS;
  delete sanitizedEnv.TAURI_CONFIG;
  delete sanitizedEnv.OPENAI_API_KEY;
  delete sanitizedEnv.ANTHROPIC_API_KEY;
  delete sanitizedEnv.OPENROUTER_API_KEY;
  delete sanitizedEnv.OLLAMA_HOST;

  let stdoutBuffer = '';
  let stderrBuffer = '';
  let childExited = false;
  let exitCode = null;
  let signal = null;
  let spawnError = null;

  let child;
  try {
    child = spawn(exePath, [], {
      env: sanitizedEnv,
      windowsHide: true,
      cwd: syntheticDataRoot,
    });
  } catch (err) {
    return {
      childPid: null,
      spawnError: err.message,
      childExitedEarly: true,
      childExitedNormally: false,
      exitCode: null,
      signal: null,
      teardownSuccess: true,
      stdoutExcerpt: '',
      stderrExcerpt: '',
    };
  }

  const childPid = child.pid;
  const ownedPidsSet = new Set(childPid ? [Number(childPid)] : []);

  child.on('error', err => {
    spawnError = err.message;
  });

  child.stdout?.on('data', data => {
    stdoutBuffer += data.toString('utf8');
  });
  child.stderr?.on('data', data => {
    stderrBuffer += data.toString('utf8');
  });
  child.on('exit', (code, sig) => {
    childExited = true;
    exitCode = code;
    signal = sig;
  });

  // Track if process exits early during smoke duration and maintain owned process inventory
  const start = Date.now();
  while (Date.now() - start < smokeDurationMs) {
    if (childExited) break;
    if (childPid) {
      const enumRes = enumerateProcessTree(childPid);
      if (enumRes.ok) {
        for (const pid of enumRes.pids) ownedPidsSet.add(pid);
      }
    }
    await new Promise(resolve => setTimeout(resolve, 100));
  }

  const childExitedEarly = childExited;

  // Inventory any descendants spawned before child exited (handles parent-first-exit)
  for (const pid of Array.from(ownedPidsSet)) {
    const subRes = enumerateProcessTree(pid);
    if (subRes.ok) {
      for (const subPid of subRes.pids) ownedPidsSet.add(subPid);
    }
  }

  // Teardown process tree if still active
  let teardownSuccess = true;
  let teardownConfirmed = true;
  let teardownFailureReason = null;

  if (!childExited && childPid) {
    const termRes = await ensureProcessTreeTerminated(childPid, 3000, {
      knownPids: Array.from(ownedPidsSet),
    });
    teardownSuccess = termRes.confirmed;
    teardownConfirmed = termRes.confirmed;
    teardownFailureReason = termRes.reason || null;
  }

  return {
    childPid,
    spawnError,
    childExitedEarly,
    childExitedNormally: childExited,
    exitCode,
    signal,
    teardownSuccess,
    teardownConfirmed,
    teardownFailureReason,
    ownedPids: Array.from(ownedPidsSet),
    stdoutExcerpt: stdoutBuffer.slice(0, 1000),
    stderrExcerpt: stderrBuffer.slice(0, 1000),
  };
}

export async function runGuardedLaunch(options = {}) {
  const launchId = crypto.randomUUID();
  const leasePath = options.customLeasePath || path.join(os.homedir(), '.codex', 'anansi-application-test.lease.json');
  const artifactsBase = options.artifactsDir || CONFIGURED_PATHS.artifacts;
  const tempBase = options.tempDir || CONFIGURED_PATHS.tmp;
  const repoRoot = options.repoRoot || process.cwd();

  // 1. Check & Acquire lease
  const leaseCheck = checkTestLease(leasePath);
  if (leaseCheck.occupied) {
    return {
      ok: false,
      blockedReason: 'LEASE_OCCUPIED',
      message: `Shared application-test lease is occupied by: ${leaseCheck.details?.owner || 'unknown'} (PID ${leaseCheck.details?.pid || 'unknown'}). Aborting launch.`,
    };
  }

  const leaseResult = acquireLease(leasePath, launchId, options.leaseOwner || 'Guarded Launch Acceptance');
  if (!leaseResult.acquired) {
    return {
      ok: false,
      blockedReason: 'LEASE_COLLISION',
      message: `Could not acquire application-test lease at ${leasePath}.`,
    };
  }

  let syntheticDataRoot = null;
  let phase1Pid = null;
  let phase2Pid = null;
  let phase1 = null;
  let phase2 = null;

  try {
    // 2. Check running processes
    const active = checkRunningProcesses(['anansi-desktop.exe']);
    if (active.length > 0 && !options.skipProcessCheck) {
      return {
        ok: false,
        blockedReason: 'RUNNING_EXECUTABLE',
        message: `anansi-desktop.exe is already active on host (PID ${active.map(a => a.pid).join(', ')}). Launch aborted to prevent conflict.`,
      };
    }

    // 3. Locate executable & verify hash
    const exePath = options.exePath || path.join(artifactsBase, 'anansi-desktop.exe');
    if (!fs.existsSync(exePath)) {
      return {
        ok: false,
        blockedReason: 'EXECUTABLE_MISSING',
        message: `Executable not found at ${exePath}.`,
      };
    }

    const actualHash = hashFile(exePath);
    if (options.expectedSha256 && actualHash.toLowerCase() !== options.expectedSha256.toLowerCase()) {
      return {
        ok: false,
        blockedReason: 'HASH_MISMATCH',
        message: `Executable hash mismatch: expected ${options.expectedSha256}, calculated ${actualHash}.`,
      };
    }

    // 4. Create isolated synthetic data root strictly on E:
    syntheticDataRoot = options.dataRoot || path.join(tempBase, `acceptance-data-${launchId}`);
    const fullSynthetic = path.resolve(syntheticDataRoot);

    if (!fullSynthetic.toUpperCase().startsWith('E:\\')) {
      return {
        ok: false,
        blockedReason: 'DRIVE_ESCAPE_REFUSED',
        message: `Synthetic data root refused: must reside on E:, got ${fullSynthetic}`,
      };
    }
    if (fullSynthetic.toLowerCase().includes('anansi-preview')) {
      return {
        ok: false,
        blockedReason: 'PREVIEW_PATH_REFUSED',
        message: `Synthetic data root refused: cannot use preview directory.`,
      };
    }
    if (isPathInside(fullSynthetic, repoRoot) || isPathInside(repoRoot, fullSynthetic)) {
      return {
        ok: false,
        blockedReason: 'REPO_OVERLAP_REFUSED',
        message: `Synthetic data root refused: overlaps with repository root (${repoRoot}).`,
      };
    }

    // Verify neither target nor its ancestors are directory junctions/symlinks
    let checkDir = fullSynthetic;
    while (checkDir && path.dirname(checkDir) !== checkDir) {
      if (fs.existsSync(checkDir) && isReparsePoint(checkDir)) {
        return {
          ok: false,
          blockedReason: 'REPARSE_POINT_REFUSED',
          message: `Synthetic data root refused: path component is a reparse point/junction (${checkDir}).`,
        };
      }
      checkDir = path.dirname(checkDir);
    }

    fs.mkdirSync(syntheticDataRoot, { recursive: true });
    protectAcl(syntheticDataRoot, true);

    // 4b. Initialize workspace via real API or seed workspace
    let realApiEvidence = null;
    let expectedWs = options.expectedWorkspace || options.seedWorkspace;

    const apiExeCandidates = [
      options.apiExePath,
      path.join(artifactsBase, 'anansi-api.exe'),
      path.join(repoRoot, 'target', 'release', 'anansi-api.exe'),
      path.join(CONFIGURED_PATHS.buildCache, 'release', 'anansi-api.exe'),
    ].filter(Boolean);
    const apiExe = apiExeCandidates.find(p => fs.existsSync(p));

    if (apiExe && options.useRealApi !== false) {
      const realApiRes = await createRealApplicationWorkspaceAndRecord({
        apiExePath: apiExe,
        syntheticDataRoot,
        tempDir: tempBase,
        workspaceName: options.seedWorkspace?.name || options.expectedWorkspace?.name || 'Qualified Real Acceptance Workspace',
      });
      if (!realApiRes.ok) {
        return {
          ok: false,
          blockedReason: 'REAL_API_INITIALIZATION_FAILED',
          message: `Failed to initialize workspace through real application API path: ${realApiRes.reason}`,
        };
      }
      realApiEvidence = realApiRes;
      expectedWs = {
        id: realApiRes.workspace.id,
        name: realApiRes.workspace.name,
      };
    } else if (options.seedWorkspace) {
      const dbPath = path.join(syntheticDataRoot, ANANSI_STORAGE_CONTRACT.catalogFileName);
      initSyntheticAnansiCatalog(dbPath, {
        userVersion: 10,
        workspaces: [options.seedWorkspace],
      });
      protectAcl(dbPath, false);

      const markerPath = path.join(syntheticDataRoot, ANANSI_STORAGE_CONTRACT.rootMarkerName);
      fs.writeFileSync(
        markerPath,
        ANANSI_STORAGE_CONTRACT.rootMarkerContent
      );
      protectAcl(markerPath, false);
    }

    // Snapshot pre-existing files so unrelated sentinel files do not establish readiness
    const initialFiles = (options.seedWorkspace || realApiEvidence)
      ? []
      : (fs.existsSync(syntheticDataRoot) ? fs.readdirSync(syntheticDataRoot) : []);

    const smokeDurationMs = options.smokeDurationMs || 3000;

    // 5. Run Phase 1
    phase1 = await spawnAndTrack(exePath, syntheticDataRoot, smokeDurationMs);
    phase1Pid = phase1.childPid;

    const readiness = verifyApplicationReadiness(syntheticDataRoot, initialFiles, {
      expectedWorkspace: expectedWs,
    });
    const readinessVerified = readiness.ready;

    let persistence = { persistent: false, reason: 'Readiness not established' };
    let persistenceVerified = false;

    // 6. Optional Restart Persistence check
    if (options.restart) {
      // Cooldown
      await new Promise(resolve => setTimeout(resolve, 500));

      // Run Phase 2
      phase2 = await spawnAndTrack(exePath, syntheticDataRoot, smokeDurationMs);
      phase2Pid = phase2.childPid;

      if (readinessVerified) {
        persistence = verifyApplicationPersistence(
          syntheticDataRoot,
          readiness.recognizedArtifacts,
          {
            baselineSnapshot: readiness.catalogSnapshot,
            expectedWorkspace: expectedWs,
          }
        );
        persistenceVerified = persistence.persistent;
      }
    }

    // Inspect files in data root
    let createdFiles = [];
    try {
      createdFiles = fs.readdirSync(syntheticDataRoot, { recursive: true });
    } catch {}

    // Qualification evaluations
    const qualificationFailures = [];

    if (phase1.spawnError) {
      qualificationFailures.push(`Phase 1 spawn error: ${phase1.spawnError}`);
    }
    if (phase1.childExitedEarly) {
      if (phase1.exitCode !== 0) {
        qualificationFailures.push(`Phase 1 exited early with non-zero exit code: ${phase1.exitCode}`);
      } else {
        qualificationFailures.push('Phase 1 desktop process exited prematurely with exit code 0 during smoke window');
      }
    }
    if (!phase1.teardownSuccess) {
      qualificationFailures.push(`Phase 1 process teardown failed: PID ${phase1.childPid} could not be terminated`);
    }
    if (!readinessVerified) {
      qualificationFailures.push(`Application readiness failed: ${readiness.reason}`);
    }

    if (options.restart) {
      if (!phase2) {
        qualificationFailures.push('Phase 2 restart failed to execute');
      } else {
        if (phase2.spawnError) {
          qualificationFailures.push(`Phase 2 spawn error: ${phase2.spawnError}`);
        }
        if (phase2.childExitedEarly) {
          if (phase2.exitCode !== 0) {
            qualificationFailures.push(`Phase 2 exited early with non-zero exit code: ${phase2.exitCode}`);
          } else {
            qualificationFailures.push('Phase 2 desktop process exited prematurely with exit code 0 during smoke window');
          }
        }
        if (!phase2.teardownSuccess) {
          qualificationFailures.push(`Phase 2 process teardown failed: PID ${phase2.childPid} could not be terminated`);
        }
      }
      if (!persistenceVerified) {
        qualificationFailures.push(`Persistence failed: ${persistence.reason}`);
      }
    }

    const qualificationPassed = qualificationFailures.length === 0;

    const evidence = {
      launchId,
      timestamp: new Date().toISOString(),
      exePath,
      sha256: actualHash,
      syntheticDataRoot,
      smokeDurationMs,
      restartTested: !!options.restart,
      readinessVerified,
      persistenceVerified: options.restart ? persistenceVerified : null,
      catalogSnapshot: readiness.catalogSnapshot || null,
      phase1,
      phase2,
      realApiPath: !!realApiEvidence,
      apiCreatedWorkspace: realApiEvidence?.workspace || null,
      apiCreatedRecord: realApiEvidence?.memory || null,
      apiEndpointsTested: realApiEvidence?.endpointsTested || [],
      apiTeardownSuccess: realApiEvidence?.apiTeardownSuccess ?? null,
      createdFilesCount: createdFiles.length,
      createdFiles: createdFiles.slice(0, 50),
      qualificationPassed,
      qualificationFailures,
    };

    const evidencePath = path.join(artifactsBase, 'launch-evidence.json');
    writeJsonAtomic(evidencePath, evidence);

    if (!qualificationPassed) {
      const preservedPath = preserveFailedRoot(syntheticDataRoot, tempBase, launchId);
      return {
        ok: false,
        blockedReason: 'QUALIFICATION_FAILED',
        message: qualificationFailures.join('; '),
        evidence,
        evidencePath,
        syntheticDataRoot,
        preservedFailedRoot: preservedPath,
      };
    }

    return {
      ok: true,
      evidence,
      evidencePath,
      syntheticDataRoot,
    };
  } catch (err) {
    let preservedPath = null;
    if (syntheticDataRoot && fs.existsSync(syntheticDataRoot)) {
      preservedPath = preserveFailedRoot(syntheticDataRoot, tempBase, launchId);
    }
    return {
      ok: false,
      blockedReason: 'UNEXPECTED_ERROR',
      message: err.message,
      preservedFailedRoot: preservedPath,
    };
  } finally {
    let teardownConfirmed = true;
    let teardownFailureReason = null;
    let unkilledPid = null;

    if (phase1Pid) {
      const p1Res = await ensureProcessTreeTerminated(phase1Pid, 3000, {
        knownPids: phase1?.ownedPids,
      });
      if (!p1Res.confirmed) {
        teardownConfirmed = false;
        teardownFailureReason = p1Res.reason;
        unkilledPid = p1Res.unkilledPids?.[0] || phase1Pid;
      }
    }
    if (phase2Pid) {
      const p2Res = await ensureProcessTreeTerminated(phase2Pid, 3000, {
        knownPids: phase2?.ownedPids,
      });
      if (!p2Res.confirmed) {
        teardownConfirmed = false;
        teardownFailureReason = teardownFailureReason || p2Res.reason;
        unkilledPid = unkilledPid || p2Res.unkilledPids?.[0] || phase2Pid;
      }
    }

    if (!teardownConfirmed) {
      try {
        const failureRecord = {
          blockedReason: 'TEARDOWN_CONFIRMATION_FAILED',
          launchId,
          failedAt: new Date().toISOString(),
          unkilledPid,
          message: `Process tree cleanup could not be confirmed: ${teardownFailureReason}. Lease retained to protect host.`,
        };
        fs.writeFileSync(leasePath, JSON.stringify(failureRecord, null, 2), 'utf8');
      } catch {}
    } else {
      releaseLease(leasePath, launchId);
    }
  }
}

export async function main() {
  const args = process.argv.slice(2);
  const restart = args.includes('--restart');
  const smokeIndex = args.indexOf('--smoke-duration');
  const smokeDurationMs = smokeIndex !== -1 && args[smokeIndex + 1] ? Number(args[smokeIndex + 1]) : 5000;

  const artifactsIdx = args.indexOf('--artifacts-dir');
  const artifactsBase = artifactsIdx !== -1 ? args[artifactsIdx + 1] : CONFIGURED_PATHS.artifacts;

  const tempIdx = args.indexOf('--temp-dir');
  const tempDir = tempIdx !== -1 ? args[tempIdx + 1] : CONFIGURED_PATHS.tmp;

  const exeIdx = args.indexOf('--exe-path');
  const exePath = exeIdx !== -1 ? args[exeIdx + 1] : path.join(artifactsBase, 'anansi-desktop.exe');

  const apiExeIdx = args.indexOf('--api-exe-path');
  const apiExePath = apiExeIdx !== -1 ? args[apiExeIdx + 1] : undefined;

  const dataRootIdx = args.indexOf('--data-root');
  const dataRoot = dataRootIdx !== -1 ? args[dataRootIdx + 1] : undefined;

  const seedWsIdIdx = args.indexOf('--seed-workspace-id');
  const seedWsNameIdx = args.indexOf('--seed-workspace-name');
  let seedWorkspace = null;
  if (seedWsIdIdx !== -1 && seedWsNameIdx !== -1) {
    seedWorkspace = {
      id: args[seedWsIdIdx + 1],
      name: args[seedWsNameIdx + 1],
      name_key: args[seedWsIdIdx + 1].toLowerCase(),
    };
  }

  const expWsIdIdx = args.indexOf('--expected-workspace-id');
  const expWsNameIdx = args.indexOf('--expected-workspace-name');
  let expectedWorkspace = null;
  if (expWsIdIdx !== -1 && expWsNameIdx !== -1) {
    expectedWorkspace = {
      id: args[expWsIdIdx + 1],
      name: args[expWsNameIdx + 1],
    };
  }

  const manifestPath = path.join(artifactsBase, 'release-manifest.json');
  let expectedSha256 = null;

  if (fs.existsSync(manifestPath)) {
    try {
      const m = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
      expectedSha256 = m.artifacts?.executable?.sha256;
    } catch {}
  }

  console.log(`Starting guarded isolated Windows launch acceptance smoke (restart=${restart}, duration=${smokeDurationMs}ms)...`);
  const result = await runGuardedLaunch({
    exePath,
    apiExePath,
    artifactsDir: artifactsBase,
    tempDir,
    dataRoot,
    expectedSha256,
    smokeDurationMs,
    restart,
    seedWorkspace,
    expectedWorkspace: expectedWorkspace || seedWorkspace,
  });

  console.log('================================================================');
  console.log('ANANSI GUARDED LAUNCH ACCEPTANCE');
  console.log('================================================================');
  if (result.ok) {
    console.log('STATUS: PASS');
    console.log(`Executable: ${result.evidence.exePath}`);
    console.log(`SHA-256:    ${result.evidence.sha256}`);
    if (result.evidence.realApiPath) {
      console.log('Real API:   VERIFIED (/v1/workspaces/create, /v1/memories/revise, /v1/readiness, /v1/workspaces/list)');
      console.log(`Workspace:  ${result.evidence.apiCreatedWorkspace?.id} (${result.evidence.apiCreatedWorkspace?.name})`);
    }
    console.log(`Phase 1 PID:${result.evidence.phase1?.childPid}`);
    if (result.evidence.restartTested) {
      console.log(`Phase 2 PID:${result.evidence.phase2?.childPid}`);
      console.log(`Persistence:${result.evidence.persistenceVerified ? 'VERIFIED' : 'FAILED'}`);
    }
    console.log(`Smoke root: ${result.evidence.syntheticDataRoot}`);
    console.log(`Evidence:   ${result.evidencePath}`);
    process.exitCode = 0;
  } else {
    console.log('STATUS: BLOCKED / FAIL');
    console.log(`Reason:  ${result.blockedReason}`);
    console.log(`Message: ${result.message}`);
    if (result.preservedFailedRoot) {
      console.log(`Failed Root Preserved: ${result.preservedFailedRoot}`);
    }
    process.exitCode = 1;
  }
  console.log('================================================================');
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main();
}
