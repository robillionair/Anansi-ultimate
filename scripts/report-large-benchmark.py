#!/usr/bin/env python3
"""Create a cautious summary from the frozen fixture and raw replay ledger."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path


def reduction(a, baseline):
    return f"{100 * (1 - a / baseline):.2f}%" if baseline else "unavailable"


def is_complete(records):
    expected = {(i, a) for i in range(10) for a in ["anansi", "basic", "full"]}
    return (len(records) == 30 and {(r['batch'], r['approach']) for r in records} == expected
            and all(r.get('valid_for_accounting') for r in records))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("fixture", type=Path)
    parser.add_argument("run", type=Path)
    args = parser.parse_args()
    fixture = json.loads(args.fixture.read_text(encoding="utf-8"))
    scored = json.loads((args.run / "scores.json").read_text(encoding="utf-8"))
    records = [json.loads(x) for x in (args.run / "requests.jsonl").read_text(encoding="utf-8").splitlines()]
    totals = scored["totals"]
    complete = is_complete(records)
    lines = ["# ANANSI larger synthetic prompt-replay benchmark", "",
        f"Frozen fixture SHA-256: `{hashlib.sha256(args.fixture.read_bytes()).hexdigest()}`.",
        f"100 distinct questions, 100 independent six-document corpora; {len(records)}/30 scored calls attempted.",
        "One model, one repeat; ten independent question items per request. Actual lexical Ask preparation, no embeddings.",
        "Scope: saved-prompt cloud replay, not end-to-end desktop inference, ingestion, hybrid search or production certification.", "",
        "## Quality and reported usage", "",
        "Scores below are automatic fact matches and strict visible-evidence/citation checks, not independent semantic accuracy grades.",
        "Missing, truncated and malformed outputs remain in the denominator; refusal on an answerable item is a miss.", "",
        "| Approach | Fact matches /100 | Grounded matches /100 | Input tokens | Completion tokens | Reported total | Reported cost |",
        "|---|---:|---:|---:|---:|---:|---:|"]
    for a in ["anansi", "basic", "full"]:
        t = totals[a]
        lines.append(f"| {a} | {t['fact_correct']} | {t['grounded_correct']} | {t.get('prompt_tokens', 'unavailable')} | {t.get('completion_tokens', 'unavailable')} | {t.get('total_tokens', 'unavailable')} | ${t.get('cost', 0):.6f} |")
    lines += ["", f"Complete 30-call comparison: **{complete}**. Partial run counters are not a whole-test savings claim."]
    if complete:
        for baseline in ["full", "basic"]:
            lines += ["", f"ANANSI versus {baseline}: {reduction(totals['anansi']['prompt_tokens'], totals[baseline]['prompt_tokens'])} lower API-reported input; "
                f"{reduction(totals['anansi']['total_tokens'], totals[baseline]['total_tokens'])} lower API-reported total."]
        paired = {i: {r['approach']: r for r in records if r['batch'] == i} for i in range(10)}
        biggest = max(paired, key=lambda i: paired[i]['full']['usage']['total_tokens'] - paired[i]['anansi']['usage']['total_tokens'])
        anansi_without = totals['anansi']['total_tokens'] - paired[biggest]['anansi']['usage']['total_tokens']
        full_without = totals['full']['total_tokens'] - paired[biggest]['full']['usage']['total_tokens']
        lines += ["", f"Excluding the batch with the largest reported-total saving (batch {biggest}, ten questions): "
            f"{reduction(anansi_without, full_without)} lower reported totals. Per-question provider usage is unavailable."]
    anomalies = scored['reasoning_counter_anomalies']
    lines += ["", f"Reasoning > completion counter anomalies: **{len(anomalies)}**. Raw counters preserved; no visible-output subtraction or computation-saving claim.",
        "", "## Diagnostic quality by question category", "",
        "| Category | ANANSI grounded | Basic grounded | Full grounded |", "|---|---:|---:|---:|"]
    for category, approaches in sorted(scored['categories'].items()):
        values = [f"{approaches[a]['grounded_correct']}/{approaches[a]['denominator']}" for a in ["anansi", "basic", "full"]]
        lines.append(f"| {category} | " + " | ".join(values) + " |")
    lines += ["", "## Offline context coverage", "",
        "| Approach | All required fact spans present (answerable cases) | Zero-evidence contexts | Prompt characters (not tokens) |",
        "|---|---:|---:|---:|"]
    for a in ["anansi", "basic", "full"]:
        answerable = [c for c in fixture['cases'] if c['expected']['kind'] != 'insufficient']
        coverage = sum(c['approaches'][a]['sourceCoverage']['presentFactSpans'] == c['approaches'][a]['sourceCoverage']['requiredFactSpans'] for c in answerable)
        empty = sum(not c['approaches'][a]['sources'] for c in fixture['cases'])
        characters = sum(len(c['approaches'][a]['prompt']) for c in fixture['cases'])
        lines.append(f"| {a} | {coverage}/{len(answerable)} | {empty} | {characters} |")
    finishes = Counter(str(r.get('finish_reason')) for r in records)
    lines += ["", "## Limits and reproduction", "",
        f"Finish reasons: `{dict(finishes)}`. Preserve any truncation/parse errors in the raw ledger.",
        "100 distinct instances across ten repeated question templates; synthetic formulaic names, ASCII text, single revisions, six documents per corpus, one free cloud model and no semantic embeddings.",
        "Basic retrieval is a named simple baseline, not a commercial product or a tuned state-of-the-art RAG system.",
        "Full context includes irrelevant eligible documents; a reduction versus it is not proof of superiority over efficient retrieval.",
        "This full-context baseline differs from the earlier eight-question pilot's full bodies of cited memories. Do not present a larger percentage here as an algorithm improvement over that pilot.",
        "Batch usage cannot be allocated to individual questions or question categories. Character counts are not tokenizer measurements.",
        "Per-case zero-evidence app requests would avoid inference; this replay still evaluates all question items in shared batches.",
        "No paid-cost saving is measured: all inference is free. No significance/generalization claim or automatic publication.",
        "", "Reproduce the offline scoring with `scripts/score-large-benchmark.py`, then `scripts/report-large-benchmark.py`. "
        "Live replay requires fresh explicit authorization; do not automatically repeat the run."]
    (args.run / 'REPORT.md').write_text('\n'.join(lines) + '\n', encoding='utf-8')


if __name__ == '__main__':
    main()
