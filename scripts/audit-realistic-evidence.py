"""Offline independent source-binding and budget audit; never scores model answers."""
import argparse
import hashlib
import json
import math
from pathlib import Path


def chunks_for(row):
    response = row["response"]
    prompt = response["manifest"]["compiledPrompt"]
    citations = response["citations"]
    if not citations:
        assert not prompt
        return []
    headers = []
    for index, c in enumerate(citations, 1):
        assert c["index"] == index
        heading = c["sectionHeading"] or "General Content"
        line = f"(line {c['lineNumber']})" if c["lineNumber"] is not None else ""
        headers.append(f"[{index}] {c['title']} - {heading} {line}\n")
    cursor = prompt.index("--- GROUNDED EVIDENCE PACK ---\n") + len("--- GROUNDED EVIDENCE PACK ---\n")
    chunks = []
    for i, c in enumerate(citations):
        assert prompt[cursor:].startswith(headers[i])
        start = cursor + len(headers[i])
        next_header = headers[i + 1] if i + 1 < len(headers) else f"User Question: {row['question']}\nAnswer:"
        end = prompt.index(next_header, start)
        assert prompt[start:end].endswith("\n\n")
        chunks.append((c["memoryId"], prompt[start:end - 2]))
        cursor = end
    return chunks


def audit(data):
    rows = data["cases"]
    assert len(rows) == 80 and len({r["label"] for r in rows}) == 80
    complete = disjoint = natural = total_bindings = extra = 0
    maximum = prompt_bytes = 0
    canaries = ("999 crates", "SEALED-777", "888 crates", "777 crates", "ARCHIVED-555", "666 crates", "EXPIRED-333")
    for row in rows:
        response = row["response"]
        chunks = chunks_for(row)
        evidence = "\n".join(text for _, text in chunks)
        assert all(c not in evidence for c in canaries), row["label"]
        cited = {c["memoryId"] for c in response["citations"]}
        disclosed = {s["id"] for s in response["manifest"]["sourceMemories"]}
        assert cited == disclosed == {mid for mid, _ in chunks}
        size = sum(len(text.encode("utf-8")) for _, text in chunks)
        assert size <= 4000
        maximum = max(maximum, size)
        length = len(response["manifest"]["compiledPrompt"].encode("utf-8"))
        assert length < 5500
        prompt_bytes += length
        if row["required_bindings"]:
            assert response["status"] == "offline"
            for span, mid in row["required_bindings"]:
                assert any(source == mid and span in text for source, text in chunks), row["label"]
                total_bindings += 1
            complete += 1
            extra += len(cited - {mid for _, mid in row["required_bindings"]})
        else:
            assert all(f not in evidence for f in row["forbidden"])
            if row["label"].startswith("d"):
                assert response["status"] == "insufficientEvidence" and not cited
                disjoint += 1
            else:
                assert response["status"] in ("offline", "insufficientEvidence")
                if cited:
                    assert "state truthfully that evidence is missing" in response["manifest"]["compiledPrompt"]
                natural += 1
    assert (complete, disjoint, natural) == (68, 6, 6)
    timings = sorted(r["ask_ms"] for r in rows)
    return {"scope": "Local evidence only; no model-answer accuracy or token measurement",
            "cases": 80, "complete_answerable": complete, "bound_facts": total_bindings,
            "empty_disjoint": disjoint, "natural_negative_context_checks": natural,
            "max_evidence_bytes": maximum, "prompt_bytes": prompt_bytes,
            "additional_source_appearances": extra,
            "ask_ms_mean": sum(timings) / len(timings),
            "ask_ms_p95_test_convention": timings[max(math.ceil(len(timings) * 0.95) - 1, 0)],
            "ask_ms_max": max(timings), "cloud_calls": data["cloud_calls"]}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path)
    parser.add_argument("out", type=Path)
    args = parser.parse_args()
    raw = args.input.read_bytes()
    result = audit(json.loads(raw))
    result["evidence_sha256"] = hashlib.sha256(raw).hexdigest()
    with args.out.open("x", encoding="utf-8") as handle:
        json.dump(result, handle, indent=2)
    print(json.dumps(result, indent=2))
