"""Compare locally prepared evidence, never infer model accuracy or API token counts."""
import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path


def summarize(case):
    approach = case["approaches"]["anansi"]
    sources = approach["sources"]
    spans = case["evidence"]["exactFactSpans"]
    ids = case["evidence"]["supportingMemoryIds"]
    assert len(spans) == len(ids)
    covered = sum(any(s["memoryId"] == mid and fact in s["text"] for s in sources)
                  for mid, fact in zip(ids, spans))
    evidence_bytes = sum(len(s["text"].encode("utf-8")) for s in sources)
    assert evidence_bytes <= 4000
    return {"answerable": bool(spans), "required_facts": len(spans),
            "present_facts": covered, "all_facts_present": bool(spans) and covered == len(spans),
            "zero_evidence": not sources, "evidence_bytes": evidence_bytes,
            "prompt_bytes": len(approach["prompt"].encode("utf-8"))}


def compare(before, after):
    old = {c["id"]: c for c in before["cases"]}
    new = {c["id"]: c for c in after["cases"]}
    assert len(old) == len(new) == 100 and old.keys() == new.keys()
    totals = defaultdict(Counter)
    categories = defaultdict(lambda: defaultdict(Counter))
    rows = []
    recovered, regressed = [], []
    for cid, a in old.items():
        b = new[cid]
        # Preserve the source corpus, gold, natural question, baselines and strata.
        assert {k: v for k, v in a.items() if k != "approaches"} == {
            k: v for k, v in b.items() if k != "approaches"}
        for baseline in ("basic", "full"):
            assert a["approaches"][baseline] == b["approaches"][baseline]
        previous, current = summarize(a), summarize(b)
        for label, value in (("before", previous), ("after", current)):
            for summary in (totals[label], categories[a["category"]][label]):
                summary["cases"] += 1
                for key, amount in value.items():
                    summary[key] += int(amount)
        totals["full"]["prompt_bytes"] += len(a["approaches"]["full"]["prompt"].encode("utf-8"))
        if current["all_facts_present"] and not previous["all_facts_present"]:
            recovered.append(cid)
        if previous["all_facts_present"] and not current["all_facts_present"]:
            regressed.append(cid)
        rows.append({"id": cid, "category": a["category"], "before": previous, "after": current})
    return {"scope": "Local exact-source evidence availability only; no new model answers or measured tokens",
            "fixture_and_baselines_identical": True, "totals": dict(totals),
            "categories": {k: dict(v) for k, v in categories.items()},
            "recovered": recovered, "regressed": regressed, "cases": rows}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("before", type=Path)
    parser.add_argument("after", type=Path)
    parser.add_argument("out", type=Path)
    args = parser.parse_args()
    raw_before, raw_after = args.before.read_bytes(), args.after.read_bytes()
    result = compare(json.loads(raw_before), json.loads(raw_after))
    result["before_sha256"] = hashlib.sha256(raw_before).hexdigest()
    result["after_sha256"] = hashlib.sha256(raw_after).hexdigest()
    # Never overwrite prior measurements.
    with args.out.open("x", encoding="utf-8") as handle:
        json.dump(result, handle, indent=2)
    print(json.dumps({k: v for k, v in result.items() if k != "cases"}, indent=2))
