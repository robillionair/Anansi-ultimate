// Reproducible runner for the ANANSI token/quality benchmark.
// Usage:
//   node scripts/benchmark-tokens.mjs [--out <path>] [--real]
//     --real  run against a real local model: requires ANANSI_BENCH_ENDPOINT
//             (loopback URL, e.g. http://127.0.0.1:11434) and
//             ANANSI_BENCH_MODEL (e.g. qwen2.5:1.5b) in the environment.
//             Without --real, the harness uses fake/offline endpoints and
//             measures the request side exactly; provider-reported usage
//             and output tokens are recorded as null.
//   The harness writes JSON to $ANANSI_BENCH_OUT or
//   target/benchmark-tokens/results.json and prints a per-record table.
//   Local Ollama setup used for the Linux runs (see the report):
//     OLLAMA_MODELS=~/workspace/ollama-models OLLAMA_HOST=127.0.0.1:11434 \
//       ~/workspace/ollama/bin/ollama serve
//     OLLAMA_MODELS=~/workspace/ollama-models \
//       ~/workspace/ollama/bin/ollama pull qwen2.5:1.5b
//   Then: ANANSI_BENCH_ENDPOINT=http://127.0.0.1:11434 \
//         ANANSI_BENCH_MODEL=qwen2.5:1.5b node scripts/benchmark-tokens.mjs --real
import { spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const args = process.argv.slice(2);
const outIdx = args.indexOf("--out");
const out = outIdx >= 0 ? args[outIdx + 1] : null;
const real = args.includes("--real");
if (real) {
  if (!process.env.ANANSI_BENCH_ENDPOINT || !process.env.ANANSI_BENCH_MODEL) {
    console.error(
      "error: --real requires ANANSI_BENCH_ENDPOINT and ANANSI_BENCH_MODEL in the environment",
    );
    process.exit(2);
  }
}
const env = { ...process.env };
if (out) env.ANANSI_BENCH_OUT = out;
const res = spawnSync(
  "node",
  [
    "scripts/cargo.mjs",
    "test",
    "-p",
    "anansi-core",
    "--locked",
    "--test",
    "benchmark_tokens",
    "--",
    "--nocapture",
    "--test-threads=1",
  ],
  { cwd: root, env, stdio: "inherit" },
);
if (res.status !== 0) process.exit(res.status ?? 1);
const resultsPath =
  out ?? path.join(root, "target", "benchmark-tokens", "results.json");
if (existsSync(resultsPath)) {
  const doc = JSON.parse(readFileSync(resultsPath, "utf8"));
  const recs = doc.records ?? [];
  const anansi = recs.filter((r) => r.approach === "anansi");
  const naive = recs.filter((r) => r.approach === "baseline_naive_full_bodies");
  const sum = (rows, key) =>
    rows.reduce((a, r) => a + (r[key] ?? 0), 0);
  console.log(`\nrecords: ${recs.length} mode: ${doc.mode}`);
  console.log(
    `anansi ask prompt chars: ${sum(anansi, "prompt_chars")} est tokens: ${sum(anansi, "anansi_estimate_tokens")}`,
  );
  console.log(
    `naive baseline prompt chars: ${sum(naive, "prompt_chars")} est tokens: ${sum(naive, "anansi_estimate_tokens")}`,
  );
  console.log(`results: ${resultsPath}`);
}
