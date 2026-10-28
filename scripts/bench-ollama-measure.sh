#!/bin/bash
# External provider-reported measurement for one Ask prompt.
# ANANSI's Ask path posts to Ollama /api/generate and discards usage, so
# this script replays the EXACT compiled prompt from a manifest with the
# EXACT options Ask uses (num_ctx 2048, num_predict 96, keep_alive 0,
# stream false) and records Ollama's provider-reported counts.
# Input tokens (prompt_eval_count) are deterministic for a fixed prompt
# and tokenizer; output tokens vary with sampling and are reported as a
# sample, not as ANANSI's measurement.
# Usage: bench-ollama-measure.sh <model> <prompt-file> [endpoint]
# Output: one JSON line with prompt_eval_count, eval_count, durations.
set -euo pipefail
MODEL="$1"
PROMPT_FILE="$2"
ENDPOINT="${3:-http://127.0.0.1:11434}"
PROMPT_JSON=$(python3 -c "import json,sys; print(json.dumps(open(sys.argv[1]).read()))" "$PROMPT_FILE")
curl -s -m 300 "$ENDPOINT/api/generate" -H 'Content-Type: application/json' -d "{
  \"model\": \"$MODEL\",
  \"prompt\": $PROMPT_JSON,
  \"stream\": false,
  \"options\": {\"num_ctx\": 2048, \"num_predict\": 96},
  \"keep_alive\": 0
}" | python3 -c "
import json, sys
d = json.load(sys.stdin)
print(json.dumps({
    'model': d.get('model'),
    'prompt_eval_count': d.get('prompt_eval_count'),
    'eval_count': d.get('eval_count'),
    'total_duration_ms': (d.get('total_duration') or 0) / 1e6,
    'load_duration_ms': (d.get('load_duration') or 0) / 1e6,
    'prompt_eval_duration_ms': (d.get('prompt_eval_duration') or 0) / 1e6,
    'eval_duration_ms': (d.get('eval_duration') or 0) / 1e6,
    'done_reason': d.get('done_reason'),
}))"
