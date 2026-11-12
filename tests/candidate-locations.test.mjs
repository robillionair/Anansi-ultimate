import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { createServer } from 'node:net';
import {
  CANDIDATE_ENV,
  LEASE_IDENTITY_ENV,
  DEFAULT_CDP_PORT,
  assertIsolatedDir,
  assertQualificationHostFree,
  defaultStorageRoot,
  resolveCandidateLocations,
} from '../scripts/windows-release/candidate-locations.mjs';

const BINARY_BYTES = Buffer.from('synthetic candidate binary');
const BINARY_HASH = createHash('sha256').update(BINARY_BYTES).digest('hex');

function fakeFs(entries) {
  const map = new Map(entries);
  return {
    existsSync: p => map.has(p),
    statSync: p => {
      const e = map.get(p);
      if (!e) throw Error('ENOENT ' + p);
      return { isFile: () => e.type === 'file', isDirectory: () => e.type === 'dir' };
    },
    lstatSync: p => {
      const e = map.get(p);
      if (!e) throw Error('ENOENT ' + p);
      return { isSymbolicLink: () => e.type === 'link' };
    },
    readdirSync: p => {
      const e = map.get(p);
      if (!e || e.type !== 'dir') throw Error('ENOTDIR ' + p);
      return e.children ?? [];
    },
    readFileSync: p => {
      const e = map.get(p);
      if (!e || e.type !== 'file') throw Error('ENOENT ' + p);
      return e.content ?? Buffer.alloc(0);
    },
  };
}

const WIN = {
  path: path.win32,
  platform: 'win32',
  cwd: 'E:\\Anansi\\worktrees\\v1-pilot-rc',
  homedir: 'C:\\Users\\robin',
  env: { LOCALAPPDATA: 'C:\\Users\\robin\\AppData\\Local' },
};
const WIN_ENV = {
  [CANDIDATE_ENV.binary]: 'E:\\Anansi\\previews\\v1-pilot-rc\\anansi-desktop.exe',
  [CANDIDATE_ENV.storage]: 'E:\\Anansi\\previews\\v1-pilot-rc\\data',
  [CANDIDATE_ENV.udf]: 'E:\\Anansi\\previews\\v1-pilot-rc\\webview-udf',
};
function winFs(extra = []) {
  return fakeFs([
    ['E:\\Anansi\\previews\\v1-pilot-rc\\anansi-desktop.exe', { type: 'file', content: BINARY_BYTES }],
    ...extra,
  ]);
}
function winDeps(extra = [], overrides = {}) {
  return { ...WIN, fs: winFs(extra), ...overrides };
}

test('explicit Windows candidate locations resolve with binary identity', () => {
  const resolved = resolveCandidateLocations(WIN_ENV, winDeps());
  assert.equal(resolved.binaryPath, 'E:\\Anansi\\previews\\v1-pilot-rc\\anansi-desktop.exe');
  assert.equal(resolved.storageRoot, 'E:\\Anansi\\previews\\v1-pilot-rc\\data');
  assert.equal(resolved.udfPath, 'E:\\Anansi\\previews\\v1-pilot-rc\\webview-udf');
  assert.equal(resolved.port, DEFAULT_CDP_PORT);
  assert.equal(resolved.binarySha256, BINARY_HASH);
  assert.equal(resolved.binaryBytes, BINARY_BYTES.length);
});

test('missing candidate locations are refused with every missing name', () => {
  assert.throws(
    () => resolveCandidateLocations({}, winDeps()),
    /ANANSI_CANDIDATE_BINARY.*ANANSI_CANDIDATE_STORAGE_ROOT.*ANANSI_CANDIDATE_WEBVIEW_UDF/,
  );
  assert.throws(
    () => resolveCandidateLocations({ [CANDIDATE_ENV.binary]: WIN_ENV[CANDIDATE_ENV.binary] }, winDeps()),
    /ANANSI_CANDIDATE_STORAGE_ROOT/,
  );
});

test('relative and non-E binary paths are refused', () => {
  assert.throws(
    () => resolveCandidateLocations({ ...WIN_ENV, [CANDIDATE_ENV.binary]: 'anansi-desktop.exe' }, winDeps()),
    /must be an absolute path/,
  );
  assert.throws(
    () => resolveCandidateLocations({ ...WIN_ENV, [CANDIDATE_ENV.binary]: 'C:\\temp\\anansi-desktop.exe' }, winDeps()),
    /must reside on E:/,
  );
});

test('missing binary is refused', () => {
  assert.throws(
    () => resolveCandidateLocations(WIN_ENV, winDeps([], { fs: fakeFs([]) })),
    /does not name an existing file/,
  );
});

test('non-empty storage, equal storage/UDF, and preview paths are refused', () => {
  assert.throws(
    () => resolveCandidateLocations(WIN_ENV, winDeps([['E:\\Anansi\\previews\\v1-pilot-rc\\data', { type: 'dir', children: ['workspaces.sqlite3'] }]])),
    /not empty/,
  );
  assert.throws(
    () => resolveCandidateLocations({ ...WIN_ENV, [CANDIDATE_ENV.udf]: WIN_ENV[CANDIDATE_ENV.storage] }, winDeps()),
    /must be different directories/,
  );
  assert.throws(
    () => resolveCandidateLocations({ ...WIN_ENV, [CANDIDATE_ENV.storage]: 'E:\\Anansi\\ANANSI-Preview\\data' }, winDeps()),
    /preview directory/,
  );
});

test('repository overlap is refused in both directions', () => {
  assert.throws(
    () => resolveCandidateLocations({ ...WIN_ENV, [CANDIDATE_ENV.storage]: 'E:\\Anansi\\worktrees\\v1-pilot-rc\\data' }, winDeps()),
    /overlaps with repository root/,
  );
  assert.throws(
    () => resolveCandidateLocations({ ...WIN_ENV, [CANDIDATE_ENV.storage]: 'E:\\Anansi' }, winDeps()),
    /overlaps with repository root/,
  );
});

test('reparse-point ancestors are refused', () => {
  assert.throws(
    () => resolveCandidateLocations(WIN_ENV, winDeps([['E:\\Anansi\\previews', { type: 'link' }]])),
    /reparse point/,
  );
});

test('host default storage root is refused (Linux rules, no E: gate)', () => {
  const deps = {
    fs: fakeFs([['/tmp/cand/anansi-desktop', { type: 'file', content: BINARY_BYTES }]]),
    path: path.posix,
    platform: 'linux',
    cwd: '/tmp/repo',
    homedir: '/home/robin',
    env: { HOME: '/home/robin' },
  };
  assert.throws(
    () => resolveCandidateLocations({
      [CANDIDATE_ENV.binary]: '/tmp/cand/anansi-desktop',
      [CANDIDATE_ENV.storage]: '/home/robin/.local/share/anansi/workspaces/evil',
      [CANDIDATE_ENV.udf]: '/tmp/cand/udf',
    }, deps),
    /inside the host default storage root/,
  );
});

test('default storage root mirrors Core::for_host', () => {
  assert.equal(
    defaultStorageRoot({ LOCALAPPDATA: 'C:\\Users\\robin\\AppData\\Local' }, WIN),
    'C:\\Users\\robin\\AppData\\Local\\ANANSI\\workspaces',
  );
  assert.equal(
    defaultStorageRoot({}, WIN),
    'C:\\Users\\robin\\AppData\\Local\\ANANSI\\workspaces',
  );
  assert.equal(
    defaultStorageRoot({ HOME: '/home/robin' }, { path: path.posix, platform: 'linux', homedir: '/home/other' }),
    '/home/robin/.local/share/anansi/workspaces',
  );
});

test('expected binary hash is enforced when set', () => {
  const ok = resolveCandidateLocations({ ...WIN_ENV, [CANDIDATE_ENV.expectedHash]: BINARY_HASH.toUpperCase() }, winDeps());
  assert.equal(ok.binarySha256, BINARY_HASH);
  assert.throws(
    () => resolveCandidateLocations({ ...WIN_ENV, [CANDIDATE_ENV.expectedHash]: '0'.repeat(64) }, winDeps()),
    /hash mismatch/,
  );
});

test('CDP port override is validated', () => {
  assert.equal(resolveCandidateLocations({ ...WIN_ENV, [CANDIDATE_ENV.port]: '19324' }, winDeps()).port, 19324);
  for (const bad of ['0', '65536', 'abc', '19.5']) {
    assert.throws(
      () => resolveCandidateLocations({ ...WIN_ENV, [CANDIDATE_ENV.port]: bad }, winDeps()),
      /must be an integer in 1\.\.65535/,
    );
  }
});

test('isolated-directory guard rejects files and accepts empty or absent dirs', () => {
  const deps = {
    fs: fakeFs([
      ['/tmp/a-file', { type: 'file' }],
      ['/tmp/empty', { type: 'dir', children: [] }],
    ]),
    path: path.posix, platform: 'linux', cwd: '/tmp/repo', homedir: '/home/robin', env: { HOME: '/home/robin' },
  };
  assert.throws(() => assertIsolatedDir('/tmp/a-file', 'TARGET', deps), /not a directory/);
  assert.equal(assertIsolatedDir('/tmp/empty', 'TARGET', deps), '/tmp/empty');
  assert.equal(assertIsolatedDir('/tmp/absent', 'TARGET', deps), '/tmp/absent');
  assert.throws(() => assertIsolatedDir('relative/path', 'TARGET', deps), /absolute path/);
});

test('real filesystem happy path on Linux', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-candidate-locations-'));
  try {
    const binary = path.join(root, 'anansi-desktop');
    fs.writeFileSync(binary, BINARY_BYTES);
    const resolved = resolveCandidateLocations({
      [CANDIDATE_ENV.binary]: binary,
      [CANDIDATE_ENV.storage]: path.join(root, 'data'),
      [CANDIDATE_ENV.udf]: path.join(root, 'udf'),
    }, { cwd: path.join(root, 'repo'), homedir: path.join(root, 'home'), env: { HOME: path.join(root, 'home') } });
    assert.equal(resolved.binarySha256, BINARY_HASH);
    assert.equal(resolved.storageRoot, path.join(root, 'data'));
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test('host-free guard refuses running processes and foreign leases, allows own lease', async () => {
  const free = { checkRunningProcesses: () => [], checkTestLease: () => ({ occupied: false, path: '/x', details: null }), assertPortsFree: async () => {} };
  assert.deepEqual(await assertQualificationHostFree({ port: 19323, env: {} }, free), { lease: 'free', port: 19323 });

  await assert.rejects(
    assertQualificationHostFree({ env: {} }, { ...free, checkRunningProcesses: () => [{ imageName: 'anansi-desktop.exe', pid: 4242 }] }),
    /anansi-desktop\.exe \(PID 4242\)/,
  );
  await assert.rejects(
    assertQualificationHostFree({ env: {} }, { ...free, checkTestLease: () => ({ occupied: true, path: '/x', details: { owner: 'Other', pid: 7, identity: 'other-id' } }) }),
    /occupied by "Other"/,
  );
  const own = await assertQualificationHostFree(
    { env: { [LEASE_IDENTITY_ENV]: 'own-id' } },
    { ...free, checkTestLease: () => ({ occupied: true, path: '/x', details: { owner: 'M2-M4 integration', pid: 7, identity: 'own-id' } }) },
  );
  assert.equal(own.lease, 'own');
  await assert.rejects(
    assertQualificationHostFree(
      { env: { [LEASE_IDENTITY_ENV]: 'own-id' } },
      { ...free, checkTestLease: () => ({ occupied: true, path: '/x', details: { owner: 'Legacy', pid: 7 } }) },
    ),
    /occupied by "Legacy"/,
  );
});

test('host-free guard refuses an occupied CDP port against the real binder', async () => {
  const server = createServer();
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const port = server.address().port;
  try {
    await assert.rejects(
      assertQualificationHostFree({ port, env: {} }, {
        checkRunningProcesses: () => [],
        checkTestLease: () => ({ occupied: false, path: '/x', details: null }),
      }),
      /already in use/,
    );
  } finally {
    await new Promise(resolve => server.close(resolve));
  }
  await assertQualificationHostFree({ port, env: {} }, {
    checkRunningProcesses: () => [],
    checkTestLease: () => ({ occupied: false, path: '/x', details: null }),
  });
});
