import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath } from 'node:url';
import {
  CONFIGURED_PATHS,
  formatBytes,
  hashFile,
  hashBuffer,
  redactSecrets,
} from './common.mjs';
import { collectPreflight } from './preflight.mjs';
import { verifyManifest } from './build-verify.mjs';

export function validateString(val, { minLength = 0, maxLength = 256, regex = null } = {}) {
  if (typeof val !== 'string') return null;
  const trimmed = val.trim();
  if (trimmed.length < minLength || trimmed.length > maxLength) return null;
  if (regex && !regex.test(trimmed)) return null;
  return trimmed;
}

export function validateNumber(val, { min = 0, max = Number.MAX_SAFE_INTEGER, integerOnly = true } = {}) {
  if (typeof val !== 'number' || !Number.isFinite(val)) return null;
  if (integerOnly && !Number.isInteger(val)) return null;
  if (val < min || val > max) return null;
  return val;
}

export function validateBoolean(val) {
  return typeof val === 'boolean' ? val : null;
}

export function validateUuid(val) {
  if (typeof val !== 'string') return null;
  const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
  return uuidRegex.test(val.trim()) ? val.trim() : null;
}

export function validateIsoDate(val) {
  if (typeof val !== 'string') return null;
  if (val.length < 10 || val.length > 35) return null;
  const d = new Date(val);
  return !Number.isNaN(d.getTime()) && val.includes('T') ? d.toISOString() : null;
}

export function validateStringArray(arr, { maxItems = 20, maxItemLength = 128 } = {}) {
  if (!Array.isArray(arr)) return [];
  const result = [];
  for (const item of arr) {
    const s = validateString(item, { maxLength: maxItemLength });
    if (s !== null) result.push(s);
    if (result.length >= maxItems) break;
  }
  return result;
}

export function collectDiagnostics(options = {}) {
  const artifactsBase = options.artifactsDir || CONFIGURED_PATHS.artifacts;
  const extraCanaries = options.extraCanaries || [];

  // 1. Gather baseline preflight report
  const { report } = collectPreflight({ customPnpmRoots: options.customPnpmRoots });

  // 2. Check release manifest if present (strictly validated and bounded)
  const manifestPath = path.join(artifactsBase, 'release-manifest.json');
  let verificationSummary = null;
  if (fs.existsSync(manifestPath)) {
    try {
      const v = verifyManifest(manifestPath);
      verificationSummary = {
        manifestFound: true,
        valid: validateBoolean(v.valid) ?? false,
        verifiedCount: validateNumber(v.verifiedCount, { min: 0, max: 100000 }) ?? 0,
        errorCount: validateNumber(v.errors?.length, { min: 0, max: 100000 }) ?? 0,
        warningCount: validateNumber(v.warnings?.length, { min: 0, max: 100000 }) ?? 0,
      };
    } catch {
      verificationSummary = { manifestFound: true, valid: false, verifiedCount: 0, errorCount: 1, warningCount: 0 };
    }
  } else {
    verificationSummary = { manifestFound: false, valid: false, verifiedCount: 0, errorCount: 0, warningCount: 0 };
  }

  // 3. Check build log presence and size without copying free-form text
  let buildLogStatus = null;
  const buildLogPath = path.join(artifactsBase, 'build-execution.log');
  if (fs.existsSync(buildLogPath)) {
    try {
      const stats = fs.statSync(buildLogPath);
      buildLogStatus = {
        present: true,
        sizeBytes: validateNumber(stats.size, { min: 0, max: 10 * 1024 * 1024 * 1024 }) ?? 0,
      };
    } catch {
      buildLogStatus = { present: false, sizeBytes: 0 };
    }
  } else {
    buildLogStatus = { present: false, sizeBytes: 0 };
  }

  // 4. Check launch evidence metrics without copying free-form excerpts (strictly typed/bounded)
  let launchEvidenceSummary = null;
  const launchEvidencePath = path.join(artifactsBase, 'launch-evidence.json');
  if (fs.existsSync(launchEvidencePath)) {
    try {
      const rawEvidence = fs.readFileSync(launchEvidencePath, 'utf8');
      const evidence = JSON.parse(rawEvidence);
      if (typeof evidence === 'object' && evidence !== null && !Array.isArray(evidence)) {
        launchEvidenceSummary = {
          present: true,
          launchId: validateUuid(evidence.launchId),
          timestamp: validateIsoDate(evidence.timestamp),
          smokeDurationMs: validateNumber(evidence.smokeDurationMs, { min: 0, max: 86400000 }),
          restartTested: validateBoolean(evidence.restartTested),
          readinessVerified: validateBoolean(evidence.readinessVerified),
          persistenceVerified: validateBoolean(evidence.persistenceVerified),
          qualificationPassed: validateBoolean(evidence.qualificationPassed),
          createdFilesCount: validateNumber(evidence.createdFilesCount, { min: 0, max: 1000000 }),
          phase1ExitCode: validateNumber(evidence.phase1?.exitCode, { min: -100000, max: 100000 }),
          phase2ExitCode: validateNumber(evidence.phase2?.exitCode, { min: -100000, max: 100000 }),
        };
      } else {
        launchEvidenceSummary = { present: false };
      }
    } catch {
      launchEvidenceSummary = { present: false };
    }
  } else {
    launchEvidenceSummary = { present: false };
  }

  // Allowlist-only diagnostics structure with strict type/length/bounds validation
  const diagnostics = {
    schemaVersion: 1,
    diagnosticKind: 'anansi-windows-support-diagnostic',
    timestamp: new Date().toISOString(),
    build: {
      productName: 'ANANSI Ultimate',
      version: '0.1.0',
      gitCommit: validateString(report.toolchain?.git?.commit, { minLength: 7, maxLength: 64, regex: /^[0-9a-fA-F]+$/ }),
      dirtySource: validateBoolean(report.toolchain?.git?.dirty),
    },
    host: {
      platform: validateString(report.system?.platform, { maxLength: 32 }),
      arch: validateString(report.system?.arch, { maxLength: 32 }),
      osRelease: validateString(os.release(), { maxLength: 64 }),
      osType: validateString(os.type(), { maxLength: 64 }),
      totalRam: validateString(report.system?.ram?.totalFormatted, { maxLength: 32 }),
      freeRam: validateString(report.system?.ram?.freeFormatted, { maxLength: 32 }),
      percentFreeRam: validateNumber(report.system?.ram?.percentFree, { min: 0, max: 100, integerOnly: false }),
      diskC: {
        total: validateString(report.system?.diskC?.totalFormatted, { maxLength: 32 }),
        free: validateString(report.system?.diskC?.freeFormatted, { maxLength: 32 }),
        percentFree: validateNumber(report.system?.diskC?.percentFree, { min: 0, max: 100, integerOnly: false }),
      },
      diskE: {
        total: validateString(report.system?.diskE?.totalFormatted, { maxLength: 32 }),
        free: validateString(report.system?.diskE?.freeFormatted, { maxLength: 32 }),
        percentFree: validateNumber(report.system?.diskE?.percentFree, { min: 0, max: 100, integerOnly: false }),
      },
    },
    toolchains: {
      node: validateString(report.toolchain?.node?.version, { maxLength: 32 }),
      pnpm: validateString(report.toolchain?.pnpm?.version, { maxLength: 32 }),
      git: validateString(report.toolchain?.git?.version, { maxLength: 32 }),
      rustc: validateString(report.toolchain?.rustc?.output?.split('\n')[0], { maxLength: 128 }),
      cargo: validateString(report.toolchain?.cargo?.output?.split('\n')[0], { maxLength: 128 }),
      msvc: report.toolchain?.native?.cpp ? 'Installed' : 'Missing',
      windowsSdk: validateString(report.toolchain?.native?.sdkEvidence?.version, { maxLength: 64 }) || 'Missing',
      webView2: validateStringArray(report.toolchain?.native?.webviewVersions, { maxItems: 10, maxItemLength: 64 }),
      makensis: validateString(report.toolchain?.makensis?.version, { maxLength: 64 }) || (report.toolchain?.makensis?.ok ? 'Available' : 'Missing'),
    },
    buildLog: buildLogStatus,
    verification: verificationSummary,
    launchEvidence: launchEvidenceSummary,
    privacyNotice: [
      'This diagnostic export adheres strictly to ANANSI Master Spec privacy rules.',
      'Strict allowlist of bounded structured system, toolchain, and verification metrics only.',
      'No raw build logs, error backtraces, console output, personal workspaces, memories, prompts, or databases are exported.',
    ],
  };

  return diagnostics;
}

export function exportDiagnostics(options = {}) {
  const artifactsBase = options.artifactsDir || CONFIGURED_PATHS.artifacts;
  const diagnosticsDir = path.join(artifactsBase, 'diagnostics');
  fs.mkdirSync(diagnosticsDir, { recursive: true });

  const diagnostics = collectDiagnostics(options);
  const jsonContent = JSON.stringify(diagnostics, null, 2);
  const sha256 = hashBuffer(Buffer.from(jsonContent, 'utf8'));

  const timestampStr = new Date().toISOString().replace(/[:.]/g, '-');
  const filename = `anansi-support-diagnostics-${timestampStr}.json`;
  const exportPath = path.join(diagnosticsDir, filename);

  fs.writeFileSync(exportPath, jsonContent);

  return {
    exportPath,
    filename,
    sha256,
    diagnostics,
  };
}

export function testCanaryRedaction() {
  const userProfile = os.homedir();
  const testCases = [
    { category: 'Bearer Token', text: 'Authorization: Bearer abcd1234efgh5678ijkl9012mnop3456', forbidden: 'abcd1234efgh5678ijkl9012mnop3456' },
    { category: 'OpenAI API Key', text: 'sk-abcdef1234567890abcdef1234567890', forbidden: 'sk-abcdef1234567890abcdef1234567890' },
    { category: 'GitHub Token', text: 'ghp_abcdefghijklmnopqrstuvwxyz1234567890', forbidden: 'ghp_abcdefghijklmnopqrstuvwxyz1234567890' },
    { category: 'AWS Key', text: 'AKIAIOSFODNN7EXAMPLE', forbidden: 'AKIAIOSFODNN7EXAMPLE' },
    { category: 'AWS Secret', text: 'aws_secret_access_key="wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"', forbidden: 'wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY' },
    { category: 'Pairing Code', text: 'pairing_code: "PAIR-9876-CODE"', forbidden: 'PAIR-9876-CODE' },
    { category: 'Session Token', text: 'session_token = "sess_secret_token_12345"', forbidden: 'sess_secret_token_12345' },
    { category: 'Password', text: 'password: "MySuperSecretPassword123!"', forbidden: 'MySuperSecretPassword123!' },
    { category: 'API Key Param', text: 'api_key="super_confidential_key_9999"', forbidden: 'super_confidential_key_9999' },
    { category: 'Memory/Prompt Body', text: 'prompt: "Classified private memory body content that must never leak"', forbidden: 'Classified private memory body content' },
    { category: 'SQL Query', text: 'INSERT INTO private_user_table VALUES ("secret")', forbidden: 'INSERT INTO private_user_table' },
    { category: 'Personal Profile Path', text: `Writing cache to ${path.join(userProfile, 'AppData', 'Roaming', 'secret.txt')}`, forbidden: userProfile },
  ];

  const extraCanary = 'CUSTOM_SYNTHETIC_CANARY_STRING_ABCXYZ';
  const customCase = { category: 'Explicit Canary List', text: `Custom log with ${extraCanary}`, forbidden: extraCanary };

  const failures = [];

  for (const tc of testCases) {
    const redacted = redactSecrets(tc.text);
    if (redacted.includes(tc.forbidden)) {
      failures.push(`[${tc.category}] leaked: "${tc.forbidden}" was not redacted from "${tc.text}" -> "${redacted}"`);
    }
  }

  // Also test explicit extra canary redaction
  const redactedCustom = redactSecrets(customCase.text, [extraCanary]);
  if (redactedCustom.includes(customCase.forbidden)) {
    failures.push(`[${customCase.category}] leaked: "${customCase.forbidden}" was not redacted.`);
  }

  return {
    passed: failures.length === 0,
    failures,
    totalTested: testCases.length + 1,
  };
}

export function main() {
  const isPreview = process.argv.includes('--preview');
  const isExport = process.argv.includes('--export');
  const isCanaryTest = process.argv.includes('--test-canary');

  const artifactsIdx = process.argv.indexOf('--artifacts-dir');
  const artifactsDir = artifactsIdx !== -1 ? process.argv[artifactsIdx + 1] : undefined;

  if (isCanaryTest) {
    console.log('Testing synthetic secret canary redaction...');
    const result = testCanaryRedaction();
    if (result.passed) {
      console.log('PASS: All synthetic secret canaries were strictly redacted.');
      process.exitCode = 0;
    } else {
      console.error('FAIL: Canary redaction failed:');
      for (const f of result.failures) console.error(`  - ${f}`);
      process.exitCode = 1;
    }
    return;
  }

  if (isPreview) {
    const diag = collectDiagnostics({ artifactsDir });
    console.log('================================================================');
    console.log('ANANSI PRIVACY-SAFE SUPPORT DIAGNOSTICS — PREVIEW');
    console.log('================================================================');
    console.log(JSON.stringify(diag, null, 2));
    console.log('================================================================');
    console.log('No files written. To export, run with --export.');
    process.exitCode = 0;
    return;
  }

  if (isExport) {
    console.log('Generating privacy-safe support diagnostics bundle...');
    const result = exportDiagnostics({ artifactsDir });
    console.log('================================================================');
    console.log('ANANSI SUPPORT DIAGNOSTICS EXPORTED');
    console.log('================================================================');
    console.log(`Export File: ${result.exportPath}`);
    console.log(`SHA-256:     ${result.sha256}`);
    console.log('Privacy Check: Allowlisted telemetry only. Zero credentials or databases included.');
    console.log('================================================================');
    process.exitCode = 0;
    return;
  }

  console.log('Usage: node scripts/windows-release/support-diagnostics.mjs <--preview|--export|--test-canary>');
  process.exitCode = 1;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main();
}
