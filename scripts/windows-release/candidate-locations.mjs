// Shared candidate-location resolution for Windows qualification scripts.
//
// The qualification candidate (desktop executable, storage root, WebView2
// user-data folder) must be explicit: no defaults, no hardcoded paths, and
// no owner-data roots. The guard vocabulary mirrors
// launch-acceptance.mjs: E:-only data roots on Windows, preview-path and
// repository-overlap refusal, reparse-point refusal, running-process and
// shared-lease refusal.
//
// The functions are pure and dependency-injected so the Windows-only rules
// are unit-testable on Linux. Host checks reuse common.mjs
// (checkRunningProcesses, checkTestLease); path math is local and injected
// because common.mjs binds node:path directly.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { checkRunningProcesses, checkTestLease } from './common.mjs';
import { assertPortsFree } from '../verify-ports.mjs';

export const CANDIDATE_ENV = Object.freeze({
  binary: 'ANANSI_CANDIDATE_BINARY',
  storage: 'ANANSI_CANDIDATE_STORAGE_ROOT',
  udf: 'ANANSI_CANDIDATE_WEBVIEW_UDF',
  port: 'ANANSI_CANDIDATE_CDP_PORT',
  expectedHash: 'ANANSI_CANDIDATE_EXPECTED_SHA256',
});
export const LEASE_IDENTITY_ENV = 'ANANSI_APPLICATION_TEST_LEASE';
export const DEFAULT_CDP_PORT = 19323;
export const LEASE_FILE_NAME = path.join('.codex', 'anansi-application-test.lease.json');

// The host default storage root, mirroring Core::for_host: LOCALAPPDATA
// ANANSI/workspaces on Windows, ~/.local/share/anansi/workspaces elsewhere.
// When LOCALAPPDATA is unset on Windows the conventional location under the
// user profile is used, so the refusal stays conservative.
export function defaultStorageRoot(env, deps = {}) {
  const p = deps.path ?? path;
  const platform = deps.platform ?? process.platform;
  const homedir = deps.homedir ?? os.homedir();
  if (platform === 'win32') {
    const localAppData = env.LOCALAPPDATA || p.join(homedir, 'AppData', 'Local');
    return p.resolve(p.join(localAppData, 'ANANSI', 'workspaces'));
  }
  const home = env.HOME || homedir;
  return p.resolve(p.join(home, '.local', 'share', 'anansi', 'workspaces'));
}

function normalize(p, resolved, platform) {
  return platform === 'win32' ? resolved.toLowerCase() : resolved;
}

export function isInside(childPath, parentPath, deps = {}) {
  const p = deps.path ?? path;
  const platform = deps.platform ?? process.platform;
  const rel = p.relative(
    normalize(p, p.resolve(parentPath), platform),
    normalize(p, p.resolve(childPath), platform),
  );
  return rel === '' || (!rel.startsWith('..') && !p.isAbsolute(rel));
}

function equalPath(a, b, deps) {
  const p = deps.path ?? path;
  const platform = deps.platform ?? process.platform;
  return normalize(p, p.resolve(a), platform) === normalize(p, p.resolve(b), platform);
}

// Refuse a directory that must hold only isolated synthetic data: it must be
// absolute, on E: on Windows, outside the repository and the host default
// storage root, free of the preview marker, free of reparse points in its
// existing ancestors, and either absent or an empty directory.
export function assertIsolatedDir(dir, label, deps = {}) {
  const f = deps.fs ?? fs;
  const p = deps.path ?? path;
  const platform = deps.platform ?? process.platform;
  const cwd = deps.cwd ?? process.cwd();
  const env = deps.env ?? process.env;
  const resolved = p.resolve(dir);
  if (!p.isAbsolute(dir)) {
    throw Error(`${label} must be an absolute path, got: ${dir}`);
  }
  if (platform === 'win32' && !resolved.toUpperCase().startsWith('E:\\')) {
    throw Error(`${label} refused: must reside on E:, got ${resolved}`);
  }
  if (resolved.toLowerCase().includes('anansi-preview')) {
    throw Error(`${label} refused: cannot use a preview directory: ${resolved}`);
  }
  if (isInside(resolved, cwd, deps) || isInside(cwd, resolved, deps)) {
    throw Error(`${label} refused: overlaps with repository root (${cwd}): ${resolved}`);
  }
  const ownerRoot = defaultStorageRoot(env, deps);
  if (isInside(resolved, ownerRoot, deps)) {
    throw Error(`${label} refused: inside the host default storage root (${ownerRoot}): ${resolved}`);
  }
  let cursor = resolved;
  while (cursor && p.dirname(cursor) !== cursor) {
    if (f.existsSync(cursor) && f.lstatSync(cursor).isSymbolicLink()) {
      throw Error(`${label} refused: path component is a reparse point/junction (${cursor})`);
    }
    cursor = p.dirname(cursor);
  }
  if (f.existsSync(resolved)) {
    if (!f.statSync(resolved).isDirectory()) {
      throw Error(`${label} refused: exists and is not a directory: ${resolved}`);
    }
    const entries = f.readdirSync(resolved);
    if (entries.length > 0) {
      throw Error(
        `${label} refused: directory is not empty (${entries.length} entries); ` +
          'use a fresh isolated directory so no existing data is touched.',
      );
    }
  }
  return resolved;
}

export function resolveCandidateLocations(env, deps = {}) {
  const f = deps.fs ?? fs;
  const p = deps.path ?? path;
  const platform = deps.platform ?? process.platform;
  const missing = [CANDIDATE_ENV.binary, CANDIDATE_ENV.storage, CANDIDATE_ENV.udf]
    .filter(name => !env[name]);
  if (missing.length > 0) {
    throw Error(
      `Missing required candidate location(s): ${missing.join(', ')}. ` +
        'Set all three explicitly; the qualification candidate has no default paths.',
    );
  }
  const binary = p.resolve(env[CANDIDATE_ENV.binary]);
  if (!p.isAbsolute(env[CANDIDATE_ENV.binary])) {
    throw Error(`${CANDIDATE_ENV.binary} must be an absolute path, got: ${env[CANDIDATE_ENV.binary]}`);
  }
  if (platform === 'win32' && !binary.toUpperCase().startsWith('E:\\')) {
    throw Error(`${CANDIDATE_ENV.binary} refused: must reside on E:, got ${binary}`);
  }
  if (!f.existsSync(binary) || !f.statSync(binary).isFile()) {
    throw Error(`${CANDIDATE_ENV.binary} does not name an existing file: ${binary}`);
  }
  const childDeps = { ...deps, env: { ...(deps.env ?? {}), ...env } };
  const storageRoot = assertIsolatedDir(env[CANDIDATE_ENV.storage], CANDIDATE_ENV.storage, childDeps);
  const udfPath = assertIsolatedDir(env[CANDIDATE_ENV.udf], CANDIDATE_ENV.udf, childDeps);
  if (equalPath(storageRoot, udfPath, deps)) {
    throw Error('Storage root and WebView2 user-data folder must be different directories.');
  }
  const binaryBytes = f.readFileSync(binary);
  const binarySha256 = createHash('sha256').update(binaryBytes).digest('hex');
  const expected = env[CANDIDATE_ENV.expectedHash];
  if (expected && binarySha256.toLowerCase() !== expected.toLowerCase()) {
    throw Error(
      `Candidate binary hash mismatch: expected ${expected}, calculated ${binarySha256}.`,
    );
  }
  let port = DEFAULT_CDP_PORT;
  if (env[CANDIDATE_ENV.port]) {
    port = Number(env[CANDIDATE_ENV.port]);
    if (!Number.isInteger(port) || port < 1 || port > 65535) {
      throw Error(`${CANDIDATE_ENV.port} must be an integer in 1..65535, got: ${env[CANDIDATE_ENV.port]}`);
    }
  }
  return { binaryPath: binary, storageRoot, udfPath, port, binarySha256, binaryBytes: binaryBytes.length };
}

// Refuse a host that cannot run an interfering qualification check: running
// target executables, an occupied CDP port, or a shared application-test
// lease held by another identity. A lease held by this run itself (via the
// lease wrapper, which exports the identity) is allowed. Never steals.
export async function assertQualificationHostFree(options = {}, deps = {}) {
  const env = options.env ?? process.env;
  const port = options.port ?? DEFAULT_CDP_PORT;
  const running = (deps.checkRunningProcesses ?? checkRunningProcesses)(['anansi-desktop.exe', 'anansi-api.exe']);
  if (running.length > 0) {
    throw Error(
      `Conflicting processes running: ${running.map(r => `${r.imageName} (PID ${r.pid})`).join(', ')}. ` +
        'Stop them before qualification; the running preview and owner storage must stay untouched.',
    );
  }
  const leasePath = options.leasePath
    ?? (deps.path ?? path).join(deps.homedir ?? os.homedir(), LEASE_FILE_NAME);
  const lease = (deps.checkTestLease ?? checkTestLease)(leasePath);
  if (lease.occupied && lease.details?.identity !== env[LEASE_IDENTITY_ENV]) {
    throw Error(
      `Shared application-test lease is occupied by "${lease.details?.owner || 'unknown'}" ` +
        `(PID ${lease.details?.pid || 'unknown'}) at ${lease.path}. Do not steal it; wait or reconcile with its owner.`,
    );
  }
  await (deps.assertPortsFree ?? assertPortsFree)([port]);
  return { lease: lease.occupied ? 'own' : 'free', port };
}
