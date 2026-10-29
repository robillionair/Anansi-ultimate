#!/usr/bin/env python3
"""Offline diagnostic fact/citation scoring, not independent semantic adjudication."""
import argparse
from collections import Counter, defaultdict
import json
from pathlib import Path
import re


def contains_fact(text, expected):
    pattern = r"(?<!\w)" + re.escape(expected.strip()).replace(r"\ ", r"\s+") + r"(?!\w)"
    return bool(re.search(pattern, text, re.IGNORECASE))


def grade(case, approach, answer):
    expected = case["expected"]
    sources = case["approaches"][approach]["sources"]
    by_index = {s["index"]: s for s in sources}
    if not isinstance(answer, dict):
        return {"fact_correct": False, "grounded_correct": False, "missing_or_malformed": True}
    text = answer.get("answer", "")
    cites = answer.get("citations", [])
    if not isinstance(text, str) or not isinstance(cites, list):
        return {"fact_correct": False, "grounded_correct": False, "missing_or_malformed": True}
    valid = bool(cites) and all(type(i) is int and i in by_index for i in cites)
    if expected["kind"] == "insufficient":
        fact_ok = answer.get("insufficient") is True
        grounded = fact_ok and not cites
    else:
        values = expected.get("values", [expected["answer"]])
        fact_ok = all(contains_fact(text, value) for value in values) and answer.get("insufficient") is False
        if expected["kind"] == "conflict":
            fact_ok = fact_ok and bool(re.search(r"conflict|both|different|two|undated|disagree|no priority", text, re.I))
        required = set(case["evidence"]["supportingMemoryIds"])
        cited_ids = {by_index[i]["memoryId"] for i in cites if type(i) is int and i in by_index}
        exact_facts = case["evidence"]["exactFactSpans"]
        cited_texts = [by_index[i]["text"] for i in cites if type(i) is int and i in by_index]
        visible = all(any(fact in text for text in cited_texts) for fact in exact_facts)
        grounded = fact_ok and valid and required.issubset(cited_ids) and visible
    return {"fact_correct": bool(fact_ok), "grounded_correct": bool(grounded),
            "citation_indices_valid": valid, "missing_or_malformed": False,
            "answer": answer}


def score(fixture, records):
    found = {}
    totals = defaultdict(Counter)
    anomalies = []
    for record in records:
        approach = record["approach"]
        usage = record.get("usage", {})
        totals[approach]["requests"] += 1
        for key in ["prompt_tokens", "completion_tokens", "total_tokens"]:
            if isinstance(usage.get(key), int):
                totals[approach][key] += usage[key]
        totals[approach]["cost"] += usage.get("cost", 0) or 0
        reasoning = (usage.get("completion_tokens_details") or {}).get("reasoning_tokens")
        if isinstance(reasoning, int) and reasoning > usage.get("completion_tokens", reasoning):
            anomalies.append({"tag": record["tag"], "reasoning": reasoning,
                              "completion": usage.get("completion_tokens")})
        if record.get("http_status") != 200 or record.get("finish_reason") != "stop":
            continue
        seen = Counter(a.get("id") for a in record.get("answers", []) if isinstance(a, dict))
        for answer in record.get("answers", []):
            if isinstance(answer, dict) and answer.get("id") in record["case_ids"]:
                found[(approach, answer["id"])] = answer if seen[answer["id"]] == 1 else None
    grades = []
    categories = defaultdict(lambda: defaultdict(Counter))
    for case in fixture["cases"]:
        for approach in ["anansi", "basic", "full"]:
            g = grade(case, approach, found.get((approach, case["id"])))
            for summary in [totals[approach], categories[case["category"]][approach]]:
                summary["denominator"] += 1
                for key in ["fact_correct", "grounded_correct", "missing_or_malformed"]:
                    summary[key] += int(g[key])
            grades.append({"id": case["id"], "approach": approach,
                           "category": case["category"], "expected": case["expected"], **g})
    return {"scope": "Deterministic diagnostic scoring; semantic correctness requires review",
            "totals": dict(totals), "categories": {k: dict(v) for k, v in categories.items()},
            "reasoning_counter_anomalies": anomalies, "grades": grades,
            "usage_scope": "batch-level API counters; no per-question token usage"}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("fixture", type=Path)
    parser.add_argument("ledger", type=Path)
    parser.add_argument("out", type=Path)
    args = parser.parse_args()
    fixture = json.loads(args.fixture.read_text(encoding="utf-8"))
    records = [json.loads(line) for line in args.ledger.read_text(encoding="utf-8").splitlines() if line.strip()]
    args.out.write_text(json.dumps(score(fixture, records), indent=2, ensure_ascii=False), encoding="utf-8")
