#!/usr/bin/env python3
"""Analyze ANANSI benchmark results.json into tables and an SVG chart.

Reads target/benchmark-tokens/results.json (or $ANANSI_BENCH_OUT) and
prints a markdown summary plus an SVG grouped bar chart of estimated
input tokens per Ask scenario (ANANSI vs naive full-body baseline).

Units are ANANSI estimates (ceil(chars/4)) unless a record carries
provider-reported tokens; provider-reported fields are shown separately
and never mixed into estimate totals. Records with null token fields
are excluded from sums and counted as missing.
"""
import json
import os
import sys
from collections import defaultdict

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def load(path):
    with open(path) as f:
        return json.load(f)


def fmt(n):
    return "n/a" if n is None else f"{n:,}"


def main():
    path = os.environ.get("ANANSI_BENCH_OUT") or os.path.join(
        ROOT, "target", "benchmark-tokens", "results.json"
    )
    doc = load(path)
    recs = doc["records"]
    print(f"# Benchmark analysis (mode: {doc.get('mode')})")
    print(f"# records: {len(recs)}")
    print()
    print("| scenario | approach | prompt_chars | est_tokens | status |")
    print("|---|---|---|---|---|")
    for r in recs:
        print(
            f"| {r['scenario']} | {r['approach']} | {fmt(r.get('prompt_chars'))} "
            f"| {fmt(r.get('anansi_estimate_tokens'))} | {r['status']} |"
        )
    print()
    # Ask comparison: ANANSI vs naive per scenario (answerable only).
    by = defaultdict(dict)
    for r in recs:
        if r["scenario"].startswith("A") and r["approach"] in (
            "anansi",
            "baseline_naive_full_bodies",
        ):
            by[(r["scenario"], r["question_id"])][r["approach"]] = r
    print("## Ask: ANANSI vs naive full-body baseline (est. tokens)")
    print("| question | anansi | naive | delta | delta % |")
    print("|---|---|---|---|---|")
    tot_a = tot_n = 0
    for key in sorted(by):
        pair = by[key]
        a = pair.get("anansi")
        n = pair.get("baseline_naive_full_bodies")
        if not a or not n:
            continue
        ae = a.get("anansi_estimate_tokens")
        ne = n.get("anansi_estimate_tokens")
        if ae is None or ne is None:
            continue
        if a["status"] == "InsufficientEvidence":
            print(f"| {key[1]} | miss | {ne} | — | recall miss, not a saving |")
            continue
        d = ae - ne  # negative = ANANSI sends fewer tokens (a saving)
        pct = 100.0 * d / ne if ne else 0.0
        tot_a += ae
        tot_n += ne
        print(f"| {key[1]} | {ae} | {ne} | {d:+} | {pct:+.1f}% |")
    if tot_n:
        print(f"| **answerable total** | **{tot_a}** | **{tot_n}** | **{tot_a - tot_n:+}** | **{100.0 * (tot_a - tot_n) / tot_n:+.1f}%** |")
    print()
    print("Delta sign: negative = ANANSI sends fewer tokens than the baseline.")
    print()
    # A8 conversation: ANANSI per-turn vs history baseline growth.
    print("## A8 conversation: ANANSI per-turn vs history baseline (est. tokens)")
    print("| turn | anansi | history | anansi - history |")
    print("|---|---|---|---|")
    a8 = defaultdict(dict)
    for r in recs:
        if r["scenario"] == "A8":
            a8[r["question_id"]][r["approach"]] = r
    for qid in sorted(a8):
        pair = a8[qid]
        a = pair.get("anansi")
        h = pair.get("baseline_history")
        if not a or not h:
            continue
        ae = a.get("anansi_estimate_tokens")
        he = h.get("anansi_estimate_tokens")
        if ae is None or he is None:
            continue
        if a["status"] == "InsufficientEvidence":
            print(f"| {qid} | miss | {he} | recall miss, not a saving |")
            continue
        print(f"| {qid} | {ae} | {he} | {ae - he:+} |")
    print()
    # Provider-reported vs estimate agreement (M5 records).
    if doc.get("mode") == "fake_and_offline":
        print("NOTE: in fake mode, reported tokens are fixture values (10/40), not model measurements.")
        print()
    print("## Provider-reported vs estimate (M5 records)")
    for r in recs:
        if r["approach"] in ("anansi_m5_accounting", "anansi_collider_selected_context",
                              "anansi_collider_beams_only", "anansi_collider_reviewed_context"):
            print(
                f"- {r['scenario']}: est={fmt(r.get('anansi_estimate_tokens'))} "
                f"reported_in={fmt(r.get('provider_reported_input_tokens'))} "
                f"reported_out={fmt(r.get('provider_reported_output_tokens'))} "
                f"ledger_in={fmt(r.get('ledger_input_tokens'))} "
                f"ledger_out={fmt(r.get('ledger_output_tokens'))}"
            )
    # SVG chart: answerable Ask scenarios, ANANSI vs naive.
    items = []
    for key in sorted(by):
        pair = by[key]
        a = pair.get("anansi")
        n = pair.get("baseline_naive_full_bodies")
        if not a or not n or a["status"] == "InsufficientEvidence":
            continue
        ae = a.get("anansi_estimate_tokens")
        ne = n.get("anansi_estimate_tokens")
        if ae is not None and ne is not None:
            items.append((key[1], ae, ne))
    if items:
        width, bar_h, gap = 760, 18, 14
        height = 40 + len(items) * (bar_h * 2 + gap + 6)
        maxv = max(ne for _, _, ne in items) or 1
        scale = 480.0 / maxv
        parts = [
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" font-family="sans-serif" font-size="12">',
            "<text x='10' y='20'>Estimated input tokens per Ask scenario (ANANSI estimate = ceil(chars/4))</text>",
        ]
        y = 40
        for label, ae, ne in items:
            parts.append(f"<text x='10' y='{y + 13}'>{label}</text>")
            parts.append(
                f"<rect x='140' y='{y}' width='{ae * scale:.1f}' height='{bar_h}' fill='#2f7de1'/>" )
            parts.append(
                f"<text x='{140 + ae * scale + 4:.0f}' y='{y + 13}'>{ae} ANANSI</text>")
            parts.append(
                f"<rect x='140' y='{y + bar_h + 2}' width='{ne * scale:.1f}' height='{bar_h}' fill='#c0392b'/>" )
            parts.append(
                f"<text x='{140 + ne * scale + 4:.0f}' y='{y + bar_h + 15}'>{ne} naive</text>")
            y += bar_h * 2 + gap + 6
        parts.append(
            f"<text x='10' y='{height - 6}'>n={len(items)} answerable scenarios; recall misses excluded (see table). Axes start at 0.</text>")
        parts.append("</svg>")
        out = os.path.join(ROOT, "target", "benchmark-tokens", "chart.svg")
        os.makedirs(os.path.dirname(out), exist_ok=True)
        with open(out, "w") as f:
            f.write("\n".join(parts))
        print(f"chart: {out}")


if __name__ == "__main__":
    main()
