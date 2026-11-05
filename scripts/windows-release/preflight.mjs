import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import {
  CONFIGURED_PATHS,
  PREFLIGHT_MIN_REQUIREMENTS,
  formatBytes,
  getDiskSpace,
  getRamInfo,
  findPnpm,
  findMakensis,
  checkRunningProcesses,
  checkTestLease,
} from './common.mjs';
import { collect as collectDoctor, evaluate as evaluateDoctor } from '../doctor.mjs';

function runCmd(cmd, args, env = process.env) {
  try {
    let executable = cmd;
    let finalArgs = args;
    if (process.platform === 'win32' && (cmd.toLowerCase().endsWith('.cmd') || cmd.toLowerCase().endsWith('.bat'))) {
      executable = 'cmd.exe';
      finalArgs = ['/d', '/c', cmd, ...args];
    }
    const res = spawnSync(executable, finalArgs, { encoding: 'utf8', windowsHide: true, timeout: 15000, env });
    return { ok: res.status === 0, status: res.status, output: res.stdout?.trim() || res.stderr?.trim() || null };
  } catch (err) {
    return { ok: false, status: -1, output: err.message };
  }
}

export function isNodeVersionSupported(versionStr) {
  if (!versionStr) return false;
  const match = versionStr.match(/^v?(\d+)\.(\d+)\.(\d+)/);
  if (!match) return false;
  const major = parseInt(match[1], 10);
  const minor = parseInt(match[2], 10);
  return major > 22 || (major === 22 && minor >= 13);
}

export function evaluatePreflight(report, options = {}) {
  const minRam = options.minRamBytes ?? PREFLIGHT_MIN_REQUIREMENTS.minRamBytes;
  const minFreeC = options.minFreeCDriveBytes ?? PREFLIGHT_MIN_REQUIREMENTS.minFreeCDriveBytes;
  const minFreeE = options.minFreeEDriveBytes ?? PREFLIGHT_MIN_REQUIREMENTS.minFreeEDriveBytes;

  const failures = [];
  const warnings = [];

  // 1. RAM check
  if (report.system.ram.freeBytes < minRam) {
    failures.push(`Insufficient free RAM: available ${report.system.ram.freeFormatted}, required >= ${formatBytes(minRam)}.`);
  }

  // 2. Disk C: check
  if (report.system.diskC.freeBytes < minFreeC) {
    failures.push(`Insufficient free space on C: drive: available ${report.system.diskC.freeFormatted}, required >= ${formatBytes(minFreeC)}.`);
  }

  // 3. Disk E: check
  if (report.system.diskE.freeBytes < minFreeE) {
    failures.push(`Insufficient free space on E: drive: available ${report.system.diskE.freeFormatted}, required >= ${formatBytes(minFreeE)}.`);
  }

  // 4. Toolchain checks
  if (!report.toolchain.node?.ok || !isNodeVersionSupported(report.toolchain.node?.version)) {
    failures.push(`Node.js version ${report.toolchain.node?.version || 'unknown'} is unsupported: Node >= 22.13.0 is required.`);
  }
  if (!report.toolchain.pnpm?.ok || report.toolchain.pnpm?.version !== PREFLIGHT_MIN_REQUIREMENTS.pinnedPnpmVersion) {
    failures.push(`pnpm version mismatch: expected ${PREFLIGHT_MIN_REQUIREMENTS.pinnedPnpmVersion}, found ${report.toolchain.pnpm?.version || 'none'}.`);
  }
  if (!report.toolchain.git?.ok) failures.push('Git executable is missing or not functional.');
  if (!report.toolchain.rustc?.ok || !/host: .*windows-msvc/.test(report.toolchain.rustc.output ?? '')) {
    failures.push('Rust MSVC host toolchain (rustc compiler is not available or x86_64-pc-windows-msvc target missing).');
  }
  if (!report.toolchain.cargo?.ok) failures.push('Cargo executable is missing.');
  if (!report.toolchain.native?.cpp) failures.push('Visual Studio C++ build tools are missing.');
  if (!report.toolchain.native?.sdk) failures.push('Windows 10/11 SDK headers and x64 libraries are missing.');
  if (!report.toolchain.native?.webview) failures.push('WebView2 installation was not detected.');
  if (!report.toolchain.makensis?.ok) {
    warnings.push('NSIS (makensis.exe) was not found; standalone desktop builds will work, but installer bundling will fail.');
  }

  // 5. Active processes
  if (report.processSafety.runningTargetProcesses.length > 0) {
    const procs = report.processSafety.runningTargetProcesses.map(p => `${p.imageName} (PID ${p.pid})`).join(', ');
    failures.push(`Active target executables detected that would prevent clean rebuilding: ${procs}.`);
  }

  // 6. Test lease
  if (report.processSafety.applicationTestLease.occupied) {
    const owner = report.processSafety.applicationTestLease.details?.owner || 'unknown';
    const pid = report.processSafety.applicationTestLease.details?.pid || 'unknown';
    warnings.push(`Shared application-test lease is currently occupied by "${owner}" (PID ${pid}). Do not steal it; wait or run non-interfering checks.`);
  }

  return {
    ready: failures.length === 0,
    failures,
    warnings,
  };
}

export function collectPreflight(options = {}) {
  // Doctor checks
  const doctorReport = collectDoctor();
  const doctorReadiness = evaluateDoctor(doctorReport);

  // System resources
  const diskC = getDiskSpace('C:');
  const diskE = getDiskSpace('E:');
  const ram = getRamInfo();

  // Toolchain resolution
  const pnpmInfo = findPnpm(options.customPnpmRoots);
  let pnpmVersion = null;
  if (pnpmInfo?.path) {
    const pnpmRes = runCmd(pnpmInfo.path, ['--version']);
    if (pnpmRes.ok) pnpmVersion = pnpmRes.output;
  }

  const makensisInfo = findMakensis();
  let makensisVersion = null;
  if (makensisInfo?.path) {
    const nsisRes = runCmd(makensisInfo.path, ['/VERSION']);
    if (nsisRes.ok) makensisVersion = nsisRes.output;
  }

  const gitCommit = runCmd('git', ['rev-parse', 'HEAD']);
  const gitStatus = runCmd('git', ['status', '--porcelain']);

  const report = {
    timestamp: new Date().toISOString(),
    system: {
      platform: process.platform,
      arch: process.arch,
      ram,
      diskC,
      diskE,
    },
    configuredLocations: {
      buildCache: CONFIGURED_PATHS.buildCache,
      tmp: CONFIGURED_PATHS.tmp,
      artifacts: CONFIGURED_PATHS.artifacts,
    },
    toolchain: {
      node: {
        ok: isNodeVersionSupported(process.version),
        version: process.version,
        path: process.execPath,
      },
      pnpm: {
        ok: pnpmVersion === PREFLIGHT_MIN_REQUIREMENTS.pinnedPnpmVersion,
        version: pnpmVersion,
        path: pnpmInfo?.path ?? null,
        source: pnpmInfo?.source ?? null,
      },
      git: {
        ok: doctorReport.git?.ok ?? false,
        version: doctorReport.git?.output ?? null,
        commit: gitCommit.output,
        dirty: !!gitStatus.output,
      },
      rustc: {
        ok: doctorReport.rust?.ok ?? false,
        output: doctorReport.rust?.output ?? null,
        path: doctorReport.rust?.command ?? null,
      },
      cargo: {
        ok: doctorReport.cargo?.ok ?? false,
        output: doctorReport.cargo?.output ?? null,
        path: doctorReport.cargo?.command ?? null,
      },
      native: doctorReport.native ?? {},
      makensis: {
        ok: !!makensisInfo?.path,
        version: makensisVersion,
        path: makensisInfo?.path ?? null,
        source: makensisInfo?.source ?? null,
      },
    },
    processSafety: {
      runningTargetProcesses: checkRunningProcesses(['anansi-desktop.exe', 'anansi-api.exe']),
      applicationTestLease: checkTestLease(options.customLeasePath),
    },
  };

  const readiness = evaluatePreflight(report, options);

  return { report, readiness };
}

export function measureDirectory(dirPath) {
  let totalBytes = 0;
  let fileCount = 0;
  try {
    if (!fs.existsSync(dirPath)) return { totalBytes: 0, fileCount: 0, exists: false };
    const entries = fs.readdirSync(dirPath, { recursive: true, withFileTypes: true });
    for (const entry of entries) {
      if (entry.isFile()) {
        try {
          const s = fs.statSync(path.join(entry.parentPath || dirPath, entry.name));
          totalBytes += s.size;
          fileCount++;
        } catch {}
      }
    }
    return { totalBytes, fileCount, exists: true };
  } catch (err) {
    return { totalBytes: 0, fileCount: 0, exists: false, error: err.message };
  }
}

export function collectDiskInventory() {
  const entries = [];

  // 1. C: ChatGPT checkouts
  const chatGptBase = 'C:\\Users\\robin\\Documents\\ChatGPT';
  if (fs.existsSync(chatGptBase)) {
    try {
      const folders = fs.readdirSync(chatGptBase, { withFileTypes: true });
      for (const f of folders.filter(d => d.isDirectory() && d.name.toLowerCase().includes('anansi'))) {
        const fullDir = path.join(chatGptBase, f.name);

        // target/
        const targetDir = path.join(fullDir, 'target');
        if (fs.existsSync(targetDir)) {
          const m = measureDirectory(targetDir);
          const isPrimary = f.name === 'Anansi ultimate';
          const isRunningPreview = f.name === 'Anansi-m5-review';
          entries.push({
            path: targetDir,
            sizeBytes: m.totalBytes,
            sizeFormatted: formatBytes(m.totalBytes),
            fileCount: m.fileCount,
            category: 'buildCache',
            rebuildable: true,
            recoveryData: false,
            safeToPurge: !isPrimary && !isRunningPreview,
            recommendedAction: isRunningPreview
              ? 'PRESERVE_RUNNING_PREVIEW (Used by active preview)'
              : isPrimary
              ? 'REBUILD_ON_DEMAND (Primary checkout; cargo clean if needed)'
              : 'SAFE_TO_PURGE (Inactive historical checkout target)',
            reason: isRunningPreview
              ? 'Target for currently active M5 preview binary.'
              : isPrimary
              ? 'Target directory for primary ultimate workspace.'
              : `Historical review target cache from ${f.name}; safe to purge to reclaim space.`,
          });
        }

        // node_modules/
        const nmDir = path.join(fullDir, 'node_modules');
        if (fs.existsSync(nmDir)) {
          const m = measureDirectory(nmDir);
          entries.push({
            path: nmDir,
            sizeBytes: m.totalBytes,
            sizeFormatted: formatBytes(m.totalBytes),
            fileCount: m.fileCount,
            category: 'dependencies',
            rebuildable: true,
            recoveryData: false,
            safeToPurge: f.name !== 'Anansi ultimate' && f.name !== 'Anansi-m5-review',
            recommendedAction: f.name === 'Anansi ultimate' ? 'PRESERVE_PRIMARY' : 'SAFE_TO_PURGE_REINSTALLABLE',
            reason: 'NPM/pnpm dependencies; rebuildable via pnpm install.',
          });
        }

        // .git
        const gitDir = path.join(fullDir, '.git');
        if (fs.existsSync(gitDir)) {
          const isDir = fs.statSync(gitDir).isDirectory();
          if (isDir) {
            const m = measureDirectory(gitDir);
            entries.push({
              path: gitDir,
              sizeBytes: m.totalBytes,
              sizeFormatted: formatBytes(m.totalBytes),
              fileCount: m.fileCount,
              category: 'gitHistory',
              rebuildable: false,
              recoveryData: true,
              safeToPurge: false,
              recommendedAction: 'PRESERVE_MANDATORY_GIT_DATABASE',
              reason: 'Git database and object store.',
            });
          }
        }
      }
    } catch {}
  }

  // 2. E:\Anansi\build-cache
  const eBuildCache = 'E:\\Anansi\\build-cache';
  if (fs.existsSync(eBuildCache)) {
    try {
      const caches = fs.readdirSync(eBuildCache, { withFileTypes: true });
      for (const c of caches.filter(d => d.isDirectory())) {
        const fullDir = path.join(eBuildCache, c.name);
        const m = measureDirectory(fullDir);
        const isCurrent = c.name === 'antigravity-release';
        const isR17 = c.name === 'preview-r17';
        entries.push({
          path: fullDir,
          sizeBytes: m.totalBytes,
          sizeFormatted: formatBytes(m.totalBytes),
          fileCount: m.fileCount,
          category: 'buildCache',
          rebuildable: true,
          recoveryData: false,
          safeToPurge: !isCurrent && !isR17,
          recommendedAction: isR17
            ? 'PRESERVE_PREVIEW_BINARY (R17 preview fallback)'
            : isCurrent
            ? 'ACTIVE_RELEASE_BUILD_CACHE'
            : 'SAFE_TO_PURGE (Historical preview/test build cache on E:)',
          reason: isR17
            ? 'Hosts binary for Start ANANSI Preview R17 launcher.'
            : isCurrent
            ? 'Active cache for current release engineering worktree.'
            : `Completed past milestone build cache (${c.name}); rebuildable.`,
        });
      }
    } catch {}
  }

  // 3. User preview & app data
  const localAppData = process.env.LOCALAPPDATA || path.join(os.homedir(), 'AppData', 'Local');
  const previewDir = path.join(localAppData, 'ANANSI-Preview');
  if (fs.existsSync(previewDir)) {
    const m = measureDirectory(previewDir);
    entries.push({
      path: previewDir,
      sizeBytes: m.totalBytes,
      sizeFormatted: formatBytes(m.totalBytes),
      fileCount: m.fileCount,
      category: 'appData',
      rebuildable: false,
      recoveryData: true,
      safeToPurge: false,
      recommendedAction: 'PRESERVE_USER_PREVIEW_DATA',
      reason: 'User preview database, memories, and personal settings (r11, r16, r17).',
    });
  }

  // Calculate totals
  let totalReclaimableBytes = 0;
  let totalPreservedBytes = 0;
  for (const entry of entries) {
    if (entry.safeToPurge) {
      totalReclaimableBytes += entry.sizeBytes;
    } else {
      totalPreservedBytes += entry.sizeBytes;
    }
  }

  return {
    timestamp: new Date().toISOString(),
    totalReclaimableBytes,
    totalReclaimableFormatted: formatBytes(totalReclaimableBytes),
    totalPreservedBytes,
    totalPreservedFormatted: formatBytes(totalPreservedBytes),
    entries,
  };
}

export function main() {
  const isInventory = process.argv.includes('--inventory');
  const isJson = process.argv.includes('--json');
  const isCheck = process.argv.includes('--check');

  if (isInventory) {
    const inventory = collectDiskInventory();
    if (isJson) {
      console.log(JSON.stringify(inventory, null, 2));
    } else {
      console.log('================================================================');
      console.log('ANANSI WINDOWS RELEASE PREFLIGHT — READ-ONLY DISK INVENTORY');
      console.log('================================================================');
      console.log(`Timestamp: ${inventory.timestamp}`);
      console.log(`Reclaimable Space (Safe to Purge): ${inventory.totalReclaimableFormatted}`);
      console.log(`Preserved Data (User/Git/Preview): ${inventory.totalPreservedFormatted}`);
      console.log('----------------------------------------------------------------');
      for (const e of inventory.entries) {
        const flag = e.safeToPurge ? '[PURGEABLE]' : '[PRESERVE] ';
        console.log(`${flag} ${e.sizeFormatted.padStart(10)} | ${e.category.padEnd(12)} | ${e.path}`);
        console.log(`             Action: ${e.recommendedAction}`);
      }
      console.log('================================================================');
    }
    process.exitCode = 0;
    return;
  }

  const { report, readiness } = collectPreflight();

  if (isJson) {
    console.log(JSON.stringify({ report, readiness }, null, 2));
  } else {
    console.log('================================================================');
    console.log('ANANSI WINDOWS RELEASE PREFLIGHT — SYSTEM & PREREQUISITES REPORT');
    console.log('================================================================');
    console.log(`Timestamp: ${report.timestamp}`);
    console.log(`Host Platform: ${report.system.platform} (${report.system.arch})`);
    console.log(`RAM: ${report.system.ram.freeFormatted} free / ${report.system.ram.totalFormatted} total (${report.system.ram.percentFree}% free)`);
    console.log(`Disk C: ${report.system.diskC.freeFormatted} free / ${report.system.diskC.totalFormatted} total (${report.system.diskC.percentFree}% free)`);
    console.log(`Disk E: ${report.system.diskE.freeFormatted} free / ${report.system.diskE.totalFormatted} total (${report.system.diskE.percentFree}% free)`);
    console.log('----------------------------------------------------------------');
    console.log('Configured Locations:');
    console.log(`  Build Cache: ${report.configuredLocations.buildCache}`);
    console.log(`  Temp Root:   ${report.configuredLocations.tmp}`);
    console.log(`  Artifacts:   ${report.configuredLocations.artifacts}`);
    console.log('----------------------------------------------------------------');
    console.log('Toolchains & Runtime:');
    console.log(`  Node.js:   ${report.toolchain.node.version} (${report.toolchain.node.path})`);
    console.log(`  pnpm:      ${report.toolchain.pnpm.version} (${report.toolchain.pnpm.path || 'NOT FOUND'}) [${report.toolchain.pnpm.source || 'N/A'}]`);
    console.log(`  Git:       ${report.toolchain.git.version} (HEAD: ${report.toolchain.git.commit?.slice(0, 9)}, dirty: ${report.toolchain.git.dirty})`);
    console.log(`  Rustc:     ${report.toolchain.rustc.output?.split('\n')[0] || 'NOT FOUND'}`);
    console.log(`  Cargo:     ${report.toolchain.cargo.output || 'NOT FOUND'}`);
    console.log(`  MSVC C++:  ${report.toolchain.native.cpp ? 'Installed' : 'NOT FOUND'}`);
    console.log(`  Win SDK:   ${report.toolchain.native.sdkEvidence ? report.toolchain.native.sdkEvidence.version : 'NOT FOUND'}`);
    console.log(`  WebView2:  ${report.toolchain.native.webview ? report.toolchain.native.webviewVersions?.join(', ') : 'NOT FOUND'}`);
    console.log(`  makensis:  ${report.toolchain.makensis.version || (report.toolchain.makensis.ok ? 'Available' : 'NOT FOUND')} (${report.toolchain.makensis.path || 'N/A'})`);
    console.log('----------------------------------------------------------------');
    console.log('Process Safety & Lease:');
    console.log(`  Active Target Exes: ${report.processSafety.runningTargetProcesses.length === 0 ? 'None (Clean)' : report.processSafety.runningTargetProcesses.map(p => `${p.imageName} (PID ${p.pid})`).join(', ')}`);
    console.log(`  App-Test Lease:     ${report.processSafety.applicationTestLease.occupied ? `OCCUPIED by ${report.processSafety.applicationTestLease.details?.owner}` : 'Available (Free)'}`);
    console.log('================================================================');

    if (readiness.failures.length > 0) {
      console.log('PREFLIGHT BLOCKED:');
      for (const f of readiness.failures) console.log(`  [FAIL] ${f}`);
    }
    if (readiness.warnings.length > 0) {
      console.log('WARNINGS:');
      for (const w of readiness.warnings) console.log(`  [WARN] ${w}`);
    }
    if (readiness.ready) {
      console.log('VERDICT: PASS — Host and prerequisites are READY for Windows build and verification.');
    } else {
      console.log('VERDICT: FAIL — Address preflight failures before running expensive build steps.');
    }
    console.log('================================================================');
  }

  process.exitCode = readiness.ready ? 0 : 1;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main();
}
