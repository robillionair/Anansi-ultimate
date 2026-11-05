import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import crypto from 'node:crypto';
import { spawn, spawnSync, execFileSync } from 'node:child_process';

export const CONFIGURED_PATHS = Object.freeze({
  buildCache: 'E:\\Anansi\\build-cache\\antigravity-release',
  tmp: 'E:\\Anansi\\tmp\\antigravity-release',
  artifacts: 'E:\\Anansi\\artifacts\\antigravity-release',
});

export const PREFLIGHT_MIN_REQUIREMENTS = Object.freeze({
  minRamBytes: 4 * 1024 * 1024 * 1024, // 4 GiB
  minFreeCDriveBytes: 5 * 1024 * 1024 * 1024, // 5 GiB
  minFreeEDriveBytes: 20 * 1024 * 1024 * 1024, // 20 GiB
  pinnedPnpmVersion: '11.25.0',
});

export function formatBytes(bytes) {
  if (bytes === undefined || bytes === null || Number.isNaN(bytes)) return '0 B';
  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB'];
  let val = bytes;
  let idx = 0;
  while (val >= 1024 && idx < units.length - 1) {
    val /= 1024;
    idx++;
  }
  return `${val.toFixed(2)} ${units[idx]}`;
}

export function hashBuffer(buffer) {
  return crypto.createHash('sha256').update(buffer).digest('hex');
}

export function hashFile(filePath) {
  const content = fs.readFileSync(filePath);
  return crypto.createHash('sha256').update(content).digest('hex');
}

export function getDiskSpace(drive) {
  try {
    const root = path.parse(path.resolve(drive)).root || drive;
    const stats = fs.statfsSync(root);
    const totalBytes = Number(stats.blocks) * Number(stats.bsize);
    const freeBytes = Number(stats.bavail) * Number(stats.bsize);
    return {
      drive: root,
      totalBytes,
      freeBytes,
      usedBytes: totalBytes - freeBytes,
      totalFormatted: formatBytes(totalBytes),
      freeFormatted: formatBytes(freeBytes),
      usedFormatted: formatBytes(totalBytes - freeBytes),
      percentFree: totalBytes > 0 ? ((freeBytes / totalBytes) * 100).toFixed(1) : '0',
    };
  } catch (error) {
    return { drive, error: error.message, totalBytes: 0, freeBytes: 0 };
  }
}

export function getRamInfo() {
  const total = os.totalmem();
  const free = os.freemem();
  return {
    totalBytes: total,
    freeBytes: free,
    usedBytes: total - free,
    totalFormatted: formatBytes(total),
    freeFormatted: formatBytes(free),
    percentFree: ((free / total) * 100).toFixed(1),
  };
}

export function findPnpm(customSearchRoots = []) {
  // 1. Check if 'pnpm' or 'pnpm.cmd' is in PATH
  try {
    const whereResult = spawnSync('where.exe', ['pnpm.cmd'], { encoding: 'utf8', windowsHide: true });
    if (whereResult.status === 0 && whereResult.stdout?.trim()) {
      const first = whereResult.stdout.trim().split(/\r?\n/)[0];
      if (fs.existsSync(first)) {
        return { path: first, dir: path.dirname(first), source: 'PATH' };
      }
    }
  } catch {}

  // 2. Check PNPM_HOME
  if (process.env.PNPM_HOME) {
    const candidate = path.join(process.env.PNPM_HOME, 'pnpm.cmd');
    if (fs.existsSync(candidate)) {
      return { path: candidate, dir: process.env.PNPM_HOME, source: 'PNPM_HOME' };
    }
  }

  // 3. Check AppData Local pnpm store package-manager-store links
  const localAppData = process.env.LOCALAPPDATA || path.join(os.homedir(), 'AppData', 'Local');
  const storeLinkBase = path.join(localAppData, 'pnpm', 'package-manager-store', 'v11', 'links', '@pnpm', 'exe', '11.25.0');
  if (fs.existsSync(storeLinkBase)) {
    try {
      const hashes = fs.readdirSync(storeLinkBase, { withFileTypes: true });
      for (const hashDir of hashes.filter(h => h.isDirectory())) {
        const binCandidate = path.join(storeLinkBase, hashDir.name, 'bin', 'pnpm.cmd');
        if (fs.existsSync(binCandidate)) {
          return { path: binCandidate, dir: path.dirname(binCandidate), source: 'local-store-link' };
        }
      }
    } catch {}
  }

  // 4. Check custom search roots
  for (const root of customSearchRoots) {
    const candidate = path.join(root, 'pnpm.cmd');
    if (fs.existsSync(candidate)) {
      return { path: candidate, dir: root, source: 'custom-root' };
    }
  }

  return null;
}

export function findMakensis() {
  // 1. Check PATH
  try {
    const whereResult = spawnSync('where.exe', ['makensis.exe'], { encoding: 'utf8', windowsHide: true });
    if (whereResult.status === 0 && whereResult.stdout?.trim()) {
      const first = whereResult.stdout.trim().split(/\r?\n/)[0];
      if (fs.existsSync(first)) return { path: first, source: 'PATH' };
    }
  } catch {}

  // 2. Check Tauri's local NSIS download location
  const localAppData = process.env.LOCALAPPDATA || path.join(os.homedir(), 'AppData', 'Local');
  const tauriNsis = path.join(localAppData, 'tauri', 'NSIS', 'makensis.exe');
  if (fs.existsSync(tauriNsis)) {
    return { path: tauriNsis, source: 'tauri-local-nsis' };
  }
  const tauriNsisBin = path.join(localAppData, 'tauri', 'NSIS', 'Bin', 'makensis.exe');
  if (fs.existsSync(tauriNsisBin)) {
    return { path: tauriNsisBin, source: 'tauri-local-nsis-bin' };
  }

  // 3. Program Files
  for (const pf of [process.env['ProgramFiles(x86)'], process.env.ProgramFiles].filter(Boolean)) {
    const candidate = path.join(pf, 'NSIS', 'makensis.exe');
    if (fs.existsSync(candidate)) return { path: candidate, source: 'program-files' };
  }

  return null;
}

export function checkRunningProcesses(targetNames = ['anansi-desktop.exe', 'anansi-api.exe']) {
  if (process.platform !== 'win32') return [];
  const found = [];
  try {
    const result = spawnSync('tasklist.exe', ['/FO', 'CSV', '/NH'], { encoding: 'utf8', windowsHide: true });
    if (result.status === 0 && result.stdout) {
      const lines = result.stdout.split(/\r?\n/);
      for (const line of lines) {
        if (!line.trim()) continue;
        const match = line.match(/^"([^"]+)","([^"]+)"/);
        if (match) {
          const imageName = match[1].toLowerCase();
          const pid = Number(match[2]);
          for (const target of targetNames) {
            if (imageName === target.toLowerCase()) {
              found.push({ imageName: match[1], pid });
            }
          }
        }
      }
    }
  } catch (err) {
    // Return empty on error
  }
  return found;
}

export function checkTestLease(customLeasePath = null) {
  const leasePath = customLeasePath || path.join(os.homedir(), '.codex', 'anansi-application-test.lease.json');
  if (fs.existsSync(leasePath)) {
    try {
      const raw = fs.readFileSync(leasePath, 'utf8');
      const data = JSON.parse(raw);
      return { occupied: true, path: leasePath, details: data };
    } catch {
      return { occupied: true, path: leasePath, details: { error: 'Invalid or unparseable lease file' } };
    }
  }
  return { occupied: false, path: leasePath, details: null };
}

export function isReparsePoint(targetPath) {
  try {
    if (!fs.existsSync(targetPath)) return false;
    const stats = fs.lstatSync(targetPath);
    return stats.isSymbolicLink();
  } catch {
    return false;
  }
}

export function isPathInside(childPath, parentPath) {
  const rel = path.relative(path.resolve(parentPath), path.resolve(childPath));
  return !rel.startsWith('..') && !path.isAbsolute(rel);
}

export function writeJsonAtomic(filePath, data) {
  const tempPath = `${filePath}.${crypto.randomUUID()}.tmp`;
  const json = JSON.stringify(data, null, 2);
  fs.writeFileSync(tempPath, json, 'utf8');
  fs.renameSync(tempPath, filePath);
}

export function redactSecrets(text, extraCanaries = []) {
  if (typeof text !== 'string') return text;
  let redacted = text;

  // Sanitize user profile paths: C:\Users\<username> -> <USERPROFILE>
  const userProfile = os.homedir();
  if (userProfile && userProfile.length > 5) {
    const escaped = userProfile.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    redacted = redacted.replace(new RegExp(escaped, 'gi'), '<USERPROFILE>');
  }

  // Redact extra canaries passed in
  for (const canary of extraCanaries) {
    if (typeof canary === 'string' && canary.length >= 3) {
      const escaped = canary.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      redacted = redacted.replace(new RegExp(escaped, 'g'), '[REDACTED_SECRET_CANARY]');
    }
  }

  // 1. Authorization headers & Bearer tokens
  redacted = redacted.replace(/Authorization:\s*Bearer\s+[a-zA-Z0-9._~+/-]+=*/gi, 'Authorization: Bearer [REDACTED_TOKEN]');
  redacted = redacted.replace(/Bearer\s+[a-zA-Z0-9._~+/-]+=*/gi, 'Bearer [REDACTED_TOKEN]');

  // 2. API keys patterns (sk-..., ghp_..., AKIA...)
  redacted = redacted.replace(/sk-[a-zA-Z0-9_-]{16,}/g, '[REDACTED_API_KEY]');
  redacted = redacted.replace(/(?:ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9_]{36,}/g, '[REDACTED_GITHUB_TOKEN]');
  redacted = redacted.replace(/AKIA[0-9A-Z]{16}/g, '[REDACTED_AWS_KEY]');
  redacted = redacted.replace(/aws_secret_access_key\s*[:=]\s*["']?([^"'\s\r\n]{8,})["']?/gi, 'aws_secret_access_key=[REDACTED_SECRET]');

  // 3. Pairing codes and session tokens
  redacted = redacted.replace(/\b(?:pairing_code|session_token|auth_token|refresh_token|session_secret)\b\s*[:=]\s*(?:(["'])(.*?)\1|((?!\b(?:pairing_code|session_token)\b\s*[:=])\S+))/gi, (m, quote, qval, bareVal) => {
    const val = qval !== undefined ? qval : bareVal;
    return val ? m.replace(val, '[REDACTED_SESSION_TOKEN]') : m;
  });

  // 4. Passwords and generic credentials
  redacted = redacted.replace(/\b(?:password|passwd|secret|client_secret|token|api[_-]?key)\b\s*[:=]\s*(?:(["'])(.*?)\1|((?!\b(?:password|passwd|secret|token|api[_-]?key)\b\s*[:=])[^\s"'\r\n;,]+))/gi, (m, quote, qval, bareVal) => {
    const val = qval !== undefined ? qval : bareVal;
    if (val && !val.toUpperCase().startsWith('[REDACTED_')) {
      return m.replace(val, '[REDACTED_CREDENTIAL]');
    }
    return m;
  });

  // 5. Memory / prompt / database-like bodies
  redacted = redacted.replace(/(?:prompt|memory_body|concept_body)\s*[:=]\s*["']([^"'\r\n]{5,})["']/gi, (m, val) => {
    return m.replace(val, '[REDACTED_PROMPT_BODY]');
  });
  redacted = redacted.replace(/(?:INSERT\s+INTO|CREATE\s+TABLE|SELECT\s+.*?\s+FROM)\s+[a-zA-Z0-9_]+/gi, '[REDACTED_SQL_QUERY]');

  return redacted;
}
