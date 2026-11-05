import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import {
  CONFIGURED_PATHS,
  formatBytes,
  hashFile,
  findPnpm,
  findMakensis,
} from './common.mjs';
import { nsisPayloadHash } from '../installer-artifact.mjs';

function getGitMetadata() {
  try {
    const commit = spawnSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8', windowsHide: true }).stdout?.trim() || 'unknown';
    const status = spawnSync('git', ['status', '--porcelain'], { encoding: 'utf8', windowsHide: true }).stdout?.trim() || '';
    return { commit, dirty: status.length > 0, statusOutput: status };
  } catch (err) {
    return { commit: 'unknown', dirty: false, error: err.message };
  }
}

function runStep(command, args, options = {}) {
  const startTime = Date.now();
  let executable = command;
  let finalArgs = args;

  if (process.platform === 'win32' && (command.toLowerCase().endsWith('.cmd') || command.toLowerCase().endsWith('.bat'))) {
    executable = 'cmd.exe';
    finalArgs = ['/d', '/c', command, ...args];
  }

  const result = spawnSync(executable, finalArgs, {
    encoding: 'utf8',
    windowsHide: true,
    cwd: options.cwd || process.cwd(),
    env: options.env || process.env,
    stdio: options.stdio || 'pipe',
    maxBuffer: 50 * 1024 * 1024,
  });

  const durationMs = Date.now() - startTime;
  const stdout = result.stdout || '';
  const stderr = result.stderr || '';

  return {
    command: `${command} ${args.join(' ')}`,
    exitCode: result.status ?? (result.error ? -1 : 1),
    durationMs,
    durationFormatted: `${(durationMs / 1000).toFixed(2)}s`,
    stdout,
    stderr,
    error: result.error?.message || null,
  };
}

export function buildRelease(options = {}) {
  const buildStartTime = Date.now();
  const repoRoot = options.repoRoot || process.cwd();
  const artifactsBase = options.artifactsDir || CONFIGURED_PATHS.artifacts;
  const buildCache = options.cargoTargetDir || CONFIGURED_PATHS.buildCache;
  const tempDir = options.tempDir || CONFIGURED_PATHS.tmp;

  // Ensure directories exist on E:
  fs.mkdirSync(buildCache, { recursive: true });
  fs.mkdirSync(tempDir, { recursive: true });
  fs.mkdirSync(artifactsBase, { recursive: true });

  const pnpmInfo = findPnpm(options.customPnpmRoots);
  if (!pnpmInfo?.path) {
    throw new Error('pnpm executable was not discovered. Build aborted.');
  }

  // Configure environment strictly isolated to E:
  const cargoBin = path.join(process.env.CARGO_HOME || path.join(os.homedir(), '.cargo'), 'bin');
  const isolatedEnv = {
    ...process.env,
    PATH: `${pnpmInfo.dir};${cargoBin};${process.env.PATH}`,
    CARGO_TARGET_DIR: buildCache,
    TEMP: tempDir,
    TMP: tempDir,
    npm_execpath: pnpmInfo.path,
  };

  const gitMeta = getGitMetadata();
  const steps = [];
  const logFilePath = path.join(artifactsBase, 'build-execution.log');
  const logStream = fs.createWriteStream(logFilePath, { flags: 'w', encoding: 'utf8' });

  function log(msg) {
    logStream.write(`[${new Date().toISOString()}] ${msg}\n`);
    if (options.verbose) console.log(msg);
  }

  log(`ANANSI RELEASE BUILD START — Commit: ${gitMeta.commit} (Dirty: ${gitMeta.dirty})`);
  log(`Isolated Build Cache: ${buildCache}`);
  log(`Isolated Temp Dir:    ${tempDir}`);
  log(`Artifacts Output:     ${artifactsBase}`);

  // Step 1: Web build
  log('Starting Step 1: Web frontend production build (Vite)...');
  const webStep = runStep(pnpmInfo.path, ['--filter', '@anansi/web', 'build'], { env: isolatedEnv, cwd: repoRoot });
  steps.push({ name: 'web-build', ...webStep });
  log(`Web build finished with exit code ${webStep.exitCode} in ${webStep.durationFormatted}.`);
  if (webStep.exitCode !== 0) {
    log(`Web build failed:\n${webStep.stderr || webStep.stdout}`);
    logStream.end();
    return { ok: false, failedStep: 'web-build', exitCode: webStep.exitCode, steps, logFilePath };
  }

  // Step 2: API executable compilation (cargo build --release -p anansi-api)
  log('Starting Step 2: API executable compilation (cargo build --release -p anansi-api)...');
  const cargoExe = fs.existsSync(path.join(cargoBin, 'cargo.exe')) ? path.join(cargoBin, 'cargo.exe') : 'cargo';
  const apiStep = runStep(cargoExe, ['build', '--release', '-p', 'anansi-api', '--locked'], { env: isolatedEnv, cwd: repoRoot });
  steps.push({ name: 'api-build', ...apiStep });
  log(`API build finished with exit code ${apiStep.exitCode} in ${apiStep.durationFormatted}.`);
  if (apiStep.exitCode !== 0) {
    log(`API build failed:\n${apiStep.stderr || apiStep.stdout}`);
    logStream.end();
    return { ok: false, failedStep: 'api-build', exitCode: apiStep.exitCode, steps, logFilePath };
  }

  // Step 3: Desktop standalone build
  log('Starting Step 3: Desktop executable compilation (Tauri --no-bundle)...');
  const desktopStep = runStep(pnpmInfo.path, ['--filter', '@anansi/desktop', 'build'], { env: isolatedEnv, cwd: repoRoot });
  steps.push({ name: 'desktop-build', ...desktopStep });
  log(`Desktop build finished with exit code ${desktopStep.exitCode} in ${desktopStep.durationFormatted}.`);
  if (desktopStep.exitCode !== 0) {
    log(`Desktop build failed:\n${desktopStep.stderr || desktopStep.stdout}`);
    logStream.end();
    return { ok: false, failedStep: 'desktop-build', exitCode: desktopStep.exitCode, steps, logFilePath };
  }

  // Step 4: Desktop installer build (if requested)
  let installerStep = null;
  const shouldBuildInstaller = options.buildInstaller ?? (findMakensis() !== null && !options.standaloneOnly);
  if (shouldBuildInstaller) {
    log('Starting Step 4: NSIS Installer bundling (Tauri --bundles nsis --ci --no-sign)...');
    installerStep = runStep(pnpmInfo.path, ['--filter', '@anansi/desktop', 'installer'], { env: isolatedEnv, cwd: repoRoot });
    steps.push({ name: 'installer-build', ...installerStep });
    log(`Installer build finished with exit code ${installerStep.exitCode} in ${installerStep.durationFormatted}.`);
    if (installerStep.exitCode !== 0) {
      log(`Installer build failed:\n${installerStep.stderr || installerStep.stdout}`);
      logStream.end();
      return { ok: false, failedStep: 'installer-build', exitCode: installerStep.exitCode, steps, logFilePath };
    }
  }

  // Locate built artifacts
  // In Tauri 2 with CARGO_TARGET_DIR, the executable is at CARGO_TARGET_DIR\release\anansi-desktop.exe
  const builtExeCandidates = [
    path.join(buildCache, 'release', 'anansi-desktop.exe'),
    path.join(repoRoot, 'target', 'release', 'anansi-desktop.exe'),
  ];
  const builtExe = builtExeCandidates.find(p => fs.existsSync(p));
  if (!builtExe) {
    log('Error: Could not locate built anansi-desktop.exe.');
    logStream.end();
    return { ok: false, failedStep: 'artifact-location', error: 'anansi-desktop.exe missing', steps, logFilePath };
  }

  // Locate built API executable
  const builtApiCandidates = [
    path.join(buildCache, 'release', 'anansi-api.exe'),
    path.join(repoRoot, 'target', 'release', 'anansi-api.exe'),
  ];
  const builtApiExe = builtApiCandidates.find(p => fs.existsSync(p));
  if (!builtApiExe) {
    log('Error: Could not locate built anansi-api.exe.');
    logStream.end();
    return { ok: false, failedStep: 'artifact-location', error: 'anansi-api.exe missing', steps, logFilePath };
  }

  // Copy standalone executables to artifacts
  const destExe = path.join(artifactsBase, 'anansi-desktop.exe');
  fs.copyFileSync(builtExe, destExe);
  const exeStats = fs.statSync(destExe);
  const exeHash = hashFile(destExe);

  const destApiExe = path.join(artifactsBase, 'anansi-api.exe');
  fs.copyFileSync(builtApiExe, destApiExe);
  const apiStats = fs.statSync(destApiExe);
  const apiHash = hashFile(destApiExe);

  // Collect Web output files
  const webDistDir = path.join(repoRoot, 'apps', 'web', 'dist');
  const webArtifacts = [];
  if (fs.existsSync(webDistDir)) {
    const webFiles = fs.readdirSync(webDistDir, { recursive: true, withFileTypes: true });
    for (const file of webFiles) {
      if (file.isFile()) {
        const fullPath = path.join(file.parentPath || webDistDir, file.name);
        const relPath = path.relative(webDistDir, fullPath).replace(/\\/g, '/');
        const s = fs.statSync(fullPath);
        webArtifacts.push({
          relativePath: `web/${relPath}`,
          sourcePath: fullPath,
          sizeBytes: s.size,
          sizeFormatted: formatBytes(s.size),
          sha256: hashFile(fullPath),
        });
      }
    }
  }

  // Copy web assets to artifacts/web
  const destWebDir = path.join(artifactsBase, 'web');
  fs.mkdirSync(destWebDir, { recursive: true });
  for (const w of webArtifacts) {
    const targetFile = path.join(artifactsBase, w.relativePath);
    fs.mkdirSync(path.dirname(targetFile), { recursive: true });
    fs.copyFileSync(w.sourcePath, targetFile);
  }

  // Process Installer (if built)
  let installerArtifact = null;
  if (shouldBuildInstaller) {
    const installerCandidates = [
      path.join(buildCache, 'release', 'bundle', 'nsis', 'ANANSI Ultimate_0.1.0_x64-setup.exe'),
      path.join(repoRoot, 'target', 'release', 'bundle', 'nsis', 'ANANSI Ultimate_0.1.0_x64-setup.exe'),
    ];
    const builtInstaller = installerCandidates.find(p => fs.existsSync(p));
    if (builtInstaller) {
      const destInstaller = path.join(artifactsBase, 'ANANSI Ultimate_0.1.0_x64-setup.exe');
      fs.copyFileSync(builtInstaller, destInstaller);
      const instStats = fs.statSync(destInstaller);
      const instHash = hashFile(destInstaller);

      let payloadHash = null;
      try {
        payloadHash = nsisPayloadHash(fs.readFileSync(destExe));
      } catch (err) {
        log(`Warning: nsisPayloadHash calculation failed: ${err.message}`);
      }

      installerArtifact = {
        filename: 'ANANSI Ultimate_0.1.0_x64-setup.exe',
        path: destInstaller,
        sizeBytes: instStats.size,
        sizeFormatted: formatBytes(instStats.size),
        sha256: instHash,
        nsisPayloadSha256: payloadHash,
        signingStatus: 'Unsigned (Development Build)',
      };
    }
  }

  const totalBuildDurationMs = Date.now() - buildStartTime;

  // Build the complete manifest
  const manifest = {
    schemaVersion: 1,
    kind: 'unsigned-development-release',
    productName: 'ANANSI Ultimate',
    version: '0.1.0',
    architecture: 'x64',
    platform: 'win32',
    timestamp: new Date().toISOString(),
    buildDurationMs: totalBuildDurationMs,
    buildDurationFormatted: `${(totalBuildDurationMs / 1000).toFixed(2)}s`,
    git: {
      commit: gitMeta.commit,
      dirty: gitMeta.dirty,
    },
    paths: {
      buildCache,
      tempDir,
      artifactsBase,
    },
    artifacts: {
      executable: {
        filename: 'anansi-desktop.exe',
        path: destExe,
        sizeBytes: exeStats.size,
        sizeFormatted: formatBytes(exeStats.size),
        sha256: exeHash,
      },
      api: {
        filename: 'anansi-api.exe',
        path: destApiExe,
        sizeBytes: apiStats.size,
        sizeFormatted: formatBytes(apiStats.size),
        sha256: apiHash,
      },
      installer: installerArtifact,
      webFiles: webArtifacts.map(w => ({
        relativePath: w.relativePath,
        sizeBytes: w.sizeBytes,
        sizeFormatted: w.sizeFormatted,
        sha256: w.sha256,
      })),
    },
    notes: [
      'Development release build strictly compiled to isolated E: storage.',
      'Unsigned development artifact; no certificates or identity forged.',
      'Compilation verified; installation and application acceptance require separate disposable validation.',
    ],
  };

  const manifestPath = path.join(artifactsBase, 'release-manifest.json');
  fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, 2));

  // Write SHA256SUMS file
  const sumsLines = [];
  sumsLines.push(`${exeHash} *anansi-desktop.exe`);
  sumsLines.push(`${apiHash} *anansi-api.exe`);
  if (installerArtifact) {
    sumsLines.push(`${installerArtifact.sha256} *${installerArtifact.filename}`);
  }
  for (const w of manifest.artifacts.webFiles) {
    sumsLines.push(`${w.sha256} *${w.relativePath}`);
  }
  const sumsPath = path.join(artifactsBase, 'SHA256SUMS');
  fs.writeFileSync(sumsPath, sumsLines.join('\n') + '\n');

  log(`ANANSI RELEASE BUILD FINISHED SUCCESSFULLY in ${(totalBuildDurationMs / 1000).toFixed(2)}s.`);
  log(`Manifest written to: ${manifestPath}`);
  logStream.end();

  return {
    ok: true,
    manifest,
    manifestPath,
    sumsPath,
    logFilePath,
    steps,
  };
}

export function verifyManifest(manifestPath, options = {}) {
  const errors = [];
  const warnings = [];

  if (!fs.existsSync(manifestPath)) {
    return { valid: false, errors: [`Manifest file does not exist at ${manifestPath}`], verifiedCount: 0 };
  }

  let manifest;
  try {
    manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
  } catch (err) {
    return { valid: false, errors: [`Failed to parse manifest JSON: ${err.message}`], verifiedCount: 0 };
  }

  const manifestDir = path.dirname(path.resolve(manifestPath));
  let verifiedCount = 0;
  const recordedFiles = new Set();

  function verifyFile(expectedRelPath, expectedBytes, expectedSha256, label) {
    const fullPath = path.join(manifestDir, expectedRelPath);
    recordedFiles.add(path.normalize(fullPath).toLowerCase());

    if (!fs.existsSync(fullPath)) {
      errors.push(`Missing ${label}: file does not exist at ${expectedRelPath}`);
      return;
    }

    try {
      const stats = fs.statSync(fullPath);
      if (stats.size !== expectedBytes) {
        errors.push(`Truncated or altered size for ${label}: expected ${expectedBytes} bytes, found ${stats.size} bytes`);
        return;
      }

      const actualSha256 = hashFile(fullPath);
      if (actualSha256.toLowerCase() !== expectedSha256.toLowerCase()) {
        errors.push(`Checksum mismatch for ${label}: expected ${expectedSha256}, calculated ${actualSha256}`);
        return;
      }

      verifiedCount++;
    } catch (err) {
      errors.push(`Error verifying ${label}: ${err.message}`);
    }
  }

  // 1. Verify executable
  if (manifest.artifacts?.executable) {
    const exe = manifest.artifacts.executable;
    verifyFile(exe.filename || 'anansi-desktop.exe', exe.sizeBytes, exe.sha256, 'desktop executable');
  } else {
    errors.push('Manifest does not contain an executable definition.');
  }

  // 1b. Verify API executable (if present)
  if (manifest.artifacts?.api) {
    const api = manifest.artifacts.api;
    verifyFile(api.filename || 'anansi-api.exe', api.sizeBytes, api.sha256, 'API executable');
  }

  // 2. Verify installer (if present)
  if (manifest.artifacts?.installer) {
    const inst = manifest.artifacts.installer;
    verifyFile(inst.filename, inst.sizeBytes, inst.sha256, 'installer bundle');
  }

  // 3. Verify web files
  if (Array.isArray(manifest.artifacts?.webFiles)) {
    for (const w of manifest.artifacts.webFiles) {
      verifyFile(w.relativePath, w.sizeBytes, w.sha256, `web asset (${w.relativePath})`);
    }
  }

  // 4. Check for unexpected files in the artifact directory
  if (options.checkUnexpected) {
    try {
      const allFiles = fs.readdirSync(manifestDir, { recursive: true, withFileTypes: true });
      for (const f of allFiles) {
        if (f.isFile()) {
          const fullPath = path.normalize(path.join(f.parentPath || manifestDir, f.name)).toLowerCase();
          const baseName = path.basename(fullPath);
          // Allow manifest and log files
          if (baseName === 'release-manifest.json' || baseName === 'sha256sums' || baseName.endsWith('.log')) {
            continue;
          }
          if (!recordedFiles.has(fullPath)) {
            warnings.push(`Unexpected extraneous file found in artifact directory: ${path.relative(manifestDir, fullPath)}`);
          }
        }
      }
    } catch {}
  }

  return {
    valid: errors.length === 0,
    errors,
    warnings,
    verifiedCount,
    manifest,
  };
}

export function main() {
  const isVerify = process.argv.includes('verify') || process.argv.includes('--verify');
  const isBuild = process.argv.includes('build') || process.argv.includes('--build');

  if (isVerify) {
    const manifestIdx = process.argv.findIndex(a => a === '--manifest');
    const manifestPath = manifestIdx !== -1 ? process.argv[manifestIdx + 1] : path.join(CONFIGURED_PATHS.artifacts, 'release-manifest.json');

    console.log(`Verifying offline release manifest: ${manifestPath}`);
    const result = verifyManifest(manifestPath, { checkUnexpected: true });

    console.log('================================================================');
    console.log('ANANSI RELEASE ARTIFACT OFFLINE VERIFICATION');
    console.log('================================================================');
    console.log(`Verified files: ${result.verifiedCount}`);
    if (result.errors.length > 0) {
      console.log('ERRORS:');
      for (const e of result.errors) console.log(`  [FAIL] ${e}`);
    }
    if (result.warnings?.length > 0) {
      console.log('WARNINGS:');
      for (const w of result.warnings) console.log(`  [WARN] ${w}`);
    }
    console.log('----------------------------------------------------------------');
    console.log(`VERDICT: ${result.valid ? 'PASS — All artifacts match recorded SHA-256 and byte length.' : 'FAIL — Artifact discrepancies detected.'}`);
    console.log('================================================================');

    process.exitCode = result.valid ? 0 : 1;
    return;
  }

  if (isBuild) {
    const buildInstaller = process.argv.includes('--installer');
    const standaloneOnly = process.argv.includes('--standalone-only');

    const targetDirIdx = process.argv.findIndex(a => a === '--cargo-target-dir');
    const cargoTargetDir = targetDirIdx !== -1 ? process.argv[targetDirIdx + 1] : undefined;

    const artifactsDirIdx = process.argv.findIndex(a => a === '--artifacts-dir');
    const artifactsDir = artifactsDirIdx !== -1 ? process.argv[artifactsDirIdx + 1] : undefined;

    const tempDirIdx = process.argv.findIndex(a => a === '--temp-dir');
    const tempDir = tempDirIdx !== -1 ? process.argv[tempDirIdx + 1] : undefined;

    console.log('Starting reproducible isolated Windows release build...');
    const result = buildRelease({
      buildInstaller,
      standaloneOnly,
      cargoTargetDir,
      artifactsDir,
      tempDir,
      verbose: true,
    });

    if (result.ok) {
      console.log(`Build successful! Manifest written to ${result.manifestPath}`);
      process.exitCode = 0;
    } else {
      console.error(`Build failed at step ${result.failedStep} with exit code ${result.exitCode}. See ${result.logFilePath}`);
      process.exitCode = result.exitCode || 1;
    }
    return;
  }

  console.log('Usage: node scripts/windows-release/build-verify.mjs <build|verify> [options]');
  console.log('  build   [--installer] [--standalone-only] [--cargo-target-dir <dir>] [--artifacts-dir <dir>] [--temp-dir <dir>]');
  console.log('  verify  [--manifest <path>]');
  process.exitCode = 1;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main();
}
