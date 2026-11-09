import { spawn } from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';

const ollamaPath = 'C:\\Users\\robin\\AppData\\Local\\Programs\\Ollama\\ollama.exe';
console.log('Starting Ollama daemon for SOMNIA live compaction benchmark...');
const ollama = spawn(ollamaPath, ['serve'], { windowsHide: true, stdio: 'ignore' });

const evidenceDir = path.resolve('E:/Anansi/evidence');
fs.mkdirSync(evidenceDir, { recursive: true });
fs.mkdirSync('docs/evidence', { recursive: true });

try {
  let ok = false;
  for (let i = 0; i < 30; i++) {
    try {
      const res = await fetch('http://127.0.0.1:11434/api/tags');
      if (res.ok) { ok = true; break; }
    } catch {}
    await new Promise(r => setTimeout(r, 200));
  }
  if (!ok) throw Error('Ollama daemon failed to start');
  console.log('Ollama daemon ready on http://127.0.0.1:11434');

  const env = {
    ...process.env,
    ANANSI_SOMNIA_LIVE_ENDPOINT: 'http://127.0.0.1:11434',
    ANANSI_SOMNIA_LIVE_MODEL: 'qwen2.5:0.5b'
  };

  const playwright = spawn(process.execPath, [
    path.resolve('node_modules/@playwright/test/cli.js'),
    'test',
    'tests/e2e/somnia-compaction-benchmark.spec.ts'
  ], { env, windowsHide: true, stdio: 'inherit' });

  const code = await new Promise(r => playwright.on('close', r));
  console.log('Benchmark process exited with code:', code);
  if (code !== 0) throw Error(`Playwright exited with non-zero code ${code}`);

  // Find generated json in test-results
  const testResultsDir = path.resolve('test-results');
  let foundJson = '';
  function findJson(dir) {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) findJson(full);
      else if (entry.name.endsWith('.json') && entry.name.includes('somnia-compaction-benchmark')) {
        foundJson = full;
      }
    }
  }
  findJson(testResultsDir);

  if (!foundJson) throw Error('Could not find generated benchmark json in test-results');
  console.log('Found generated benchmark JSON at:', foundJson);

  const rawJsonContent = fs.readFileSync(foundJson, 'utf8');
  const rawData = JSON.parse(rawJsonContent);

  // Compute checksum
  const sha256 = crypto.createHash('sha256').update(rawJsonContent, 'utf8').digest('hex');

  // Preserve outside disposable test-results directory
  const destEvidence = path.join(evidenceDir, 'somnia-live-compaction-qwen2.5-0.5b-raw.json');
  const destEvidenceSha = path.join(evidenceDir, 'somnia-live-compaction-qwen2.5-0.5b-raw.json.sha256');
  const destDocs = path.resolve('docs/evidence/somnia-live-compaction-qwen2.5-0.5b-raw.json');
  const destDocsSha = path.resolve('docs/evidence/somnia-live-compaction-qwen2.5-0.5b-raw.json.sha256');

  fs.writeFileSync(destEvidence, rawJsonContent, 'utf8');
  fs.writeFileSync(destEvidenceSha, `${sha256} *somnia-live-compaction-qwen2.5-0.5b-raw.json\n`, 'utf8');
  fs.writeFileSync(destDocs, rawJsonContent, 'utf8');
  fs.writeFileSync(destDocsSha, `${sha256} *somnia-live-compaction-qwen2.5-0.5b-raw.json\n`, 'utf8');

  console.log('Preserved raw benchmark JSON at:', destEvidence);
  console.log('Preserved repository benchmark JSON at:', destDocs);
  console.log('SHA-256 Checksum:', sha256);

  // Print verified benchmark summary
  console.log('\n--- VERIFIED BENCHMARK METRICS SUMMARY ---');
  console.log('Provider:               ', rawData.provider);
  console.log('Live Mode Flag:         ', rawData.accuracyLabel);
  console.log('Total Cases:            ', rawData.cases);
  console.log('Total Pairs (Denominator):', rawData.pairsTotal);
  console.log('Calls Made:             ', rawData.callsMade);
  console.log('Original Arm Correct:   ', rawData.correctOriginal, `(${rawData.correctOriginal}/${rawData.pairsTotal})`);
  console.log('Compacted Arm Correct:  ', rawData.correctCompacted, `(${rawData.correctCompacted}/${rawData.pairsTotal})`);
  console.log('Original Arm Abstain:   ', rawData.abstentionsOriginal, `(${rawData.abstentionsOriginal}/${rawData.pairsTotal})`);
  console.log('Compacted Arm Abstain:  ', rawData.abstentionsCompacted, `(${rawData.abstentionsCompacted}/${rawData.pairsTotal})`);
  console.log('Tokens Original Arm:    ', rawData.tokensOriginal);
  console.log('Tokens Compacted Arm:   ', rawData.tokensCompacted);
  const tokenDelta = rawData.tokensOriginal - rawData.tokensCompacted;
  const tokenPct = ((tokenDelta / rawData.tokensOriginal) * 100).toFixed(2);
  console.log(`Token Savings:           ${tokenDelta} tokens (${tokenPct}%)`);
  console.log('Compaction Duration:    ', rawData.compactionMicrosTotal, 'µs');
  console.log('Total Latency:          ', rawData.totalLatencyMs, 'ms');
  console.log('Failed Pairs:           ', rawData.failedPairs);
} finally {
  ollama.kill();
  console.log('Ollama daemon stopped.');
}
