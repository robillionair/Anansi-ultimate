#!/usr/bin/env python3
"""Report benchmark results under the owner's reporting rules.

Usage:
  python3 scripts/bench-report.py <results.json> [<results.json> ...]
      [--replay <replay.jsonl>]

Rules enforced here (raw results JSON is never rewritten):
- Every primary case stays in the overall denominator. Nothing is
  dropped because it failed.
- Tokens spent on failed attempts count in overall consumption.
  Per-correct figures require independent grades for each approach and
  each replay sample; these raw harness files do not supply those grades.
- Successful-case savings are a secondary breakdown only, labelled as
  such, with its sample count.
- Correct insufficient-evidence responses (A6) are counted as correct
  behaviour in the overall success rate but NOT as answered questions
  in tokens per correctly answered question. Retrieval failures on
  answerable questions (A1/A3) and the A12 compilation/budget miss are
  separate failure categories, never lumped with A6.
- Estimated counts (chars/4), direct replay (Ollama reported tokens
  for the exact prompt under Ask's exact options) and actual
  end-to-end provider usage are never mixed. Ask end-to-end provider
  usage is not recorded at all (Ask discards usage by design); replay
  is a separate measurement for the identical prompt, not the ANANSI
  call's usage. Collider end-to-end usage comes from M5
  disclosure/ledger records.

Primary cases per run: A1, A2, A3, A4, A5, A6, A11, A12, A13.
Replay file format: one JSON object per line, each with
  prompt_file (basename, e.g. "A2-anansi.txt"),
  prompt_eval_count, eval_count, and durations in ms.
"""
import argparse
import json
import re

PRIMARY = ["A1", "A2", "A3", "A4", "A5", "A6", "A11", "A12", "A13"]
RECALL_PROBES = {"A1", "A3"}


def parse_fact(notes):
    # Rust formats bools lowercase ("true"/"false").
    m = re.search(r"real_answer_fact=(true|false)", notes or "")
    return None if not m else m.group(1) == "true"


def parse_cites(notes):
    m = re.search(r"real_answer_cites_marker=(true|false)", notes or "")
    return None if not m else m.group(1) == "true"


def classify(rec, real_mode):
    """Return (category, is_correct_answer, is_correct_behaviour)."""
    sc = rec["scenario"]
    status = rec["status"]
    wire_ok = rec.get("wire_checks_passed") is True
    if sc == "A6":
        if status == "InsufficientEvidence":
            return ("correct-insufficient", False, True)
        return ("unexpected-answer", False, False)
    if sc in RECALL_PROBES:
        if status == "InsufficientEvidence":
            return ("retrieval-failure", False, False)
        if status in ("Answered", "Offline") and wire_ok:
            if real_mode:
                fact = parse_fact(rec.get("notes", ""))
                if fact is False:
                    return ("model-wrong-answer", False, False)
            return ("correct", True, True)
        return ("evidence-failure", False, False)
    # Graded answerable cases: A2, A4, A5, A11, A13.
    if status in ("Answered", "Offline") and wire_ok:
        if real_mode:
            fact = parse_fact(rec.get("notes", ""))
            if fact is False:
                # A5 exception: its must_contain is the topic phrase
                # "Harbor Hall capacity", not an answer fact, so the
                # harness's answer.contains(topic phrase) check is a
                # grading artifact, not evidence of a model miss. The
                # retained answer text (runs with ANANSI_BENCH_ANSWER_DIR)
                # must be checked manually; without it, A5 is
                # unverified, never counted as model-wrong.
                if sc == "A5":
                    return ("grading-artifact-unverified", False, False)
                return ("model-wrong-answer", False, False)
        return ("correct", True, True)
    if sc == "A12" and status in ("Answered", "Offline"):
        # A12's known failure mode is compilation/budget omission of
        # the fact chunk, not retrieval (the memory is hit).
        return ("compilation-failure", False, False)
    return ("evidence-failure", False, False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("results", nargs="+", help="Raw harness JSON result files")
    parser.add_argument("--replay", help="Separate provider-usage JSONL replay file")
    args = parser.parse_args()
    replay_path = args.replay
    docs = []
    for path in args.results:
        with open(path, encoding="utf-8") as fh:
            docs.append(json.load(fh))
    real_mode = any(d.get("mode", "").startswith("real") for d in docs)
    print(f"# Benchmark report ({'real' if real_mode else 'fake/offline'} mode, "
          f"{len(docs)} run(s))")
    print()
    print("Quality counts below are diagnostic harness proxies, not validated "
          "answer grades. The substring checks can misgrade A3/A5 and other "
          "cases. Use the retained answers and owner rubric for final scoring.")
    print()
    print("## Primary denominator: every case, no failures dropped")
    print("| case | attempts | correct | category counts |")
    print("|---|---|---|---|")
    total_attempts = 0
    total_correct_answers = 0
    total_correct_behaviour = 0
    total_anansi_est = 0
    total_naive_est = 0
    secondary_anansi = 0
    secondary_naive = 0
    secondary_n = 0
    for case in PRIMARY:
        anansi_recs = []
        naive_recs = []
        for doc in docs:
            for r in doc["records"]:
                if r["scenario"] != case:
                    continue
                if r["approach"] == "anansi":
                    anansi_recs.append(r)
                elif r["approach"] == "baseline_naive_full_bodies":
                    naive_recs.append(r)
        cats = {}
        for r in anansi_recs:
            cat, is_ans, is_beh = classify(r, real_mode)
            cats[cat] = cats.get(cat, 0) + 1
            total_attempts += 1
            total_correct_answers += 1 if is_ans else 0
            total_correct_behaviour += 1 if is_beh else 0
            total_anansi_est += r.get("anansi_estimate_tokens") or 0
            if is_ans:
                secondary_anansi += r.get("anansi_estimate_tokens") or 0
        for r in naive_recs:
            total_naive_est += r.get("anansi_estimate_tokens") or 0
            # Secondary breakdown pairs a correct ANANSI attempt with
            # the naive estimate for the same case in the same run.
            # Naive records are per run in file order; pair by index.
        # Pair naive by run order: naive_recs and anansi_recs share run order.
        for ar, nr in zip(anansi_recs, naive_recs):
            cat, is_ans, _ = classify(ar, real_mode)
            if is_ans:
                secondary_naive += nr.get("anansi_estimate_tokens") or 0
                secondary_n += 1
        cat_str = ", ".join(f"{k} {v}" for k, v in sorted(cats.items()))
        correct_n = cats.get("correct", 0)
        print(f"| {case} | {len(anansi_recs)} | {correct_n} | {cat_str} |")
    print()
    print(f"Diagnostic harness success rate (fact proxies + correct-insufficient, "
          f"over all primary attempts): "
          f"{total_correct_behaviour}/{total_attempts} "
          f"= {100.0 * total_correct_behaviour / max(total_attempts, 1):.1f}%")
    print(f"Correctly answered answerable questions: {total_correct_answers}")
    if real_mode:
        cites_yes = cites_no = 0
        for doc in docs:
            for r in doc["records"]:
                if r["approach"] != "anansi" or r["scenario"] not in PRIMARY:
                    continue
                cat, is_ans, _ = classify(r, True)
                if is_ans:
                    c = parse_cites(r.get("notes", ""))
                    if c is True:
                        cites_yes += 1
                    elif c is False:
                        cites_no += 1
        print(f"Citation support among correctly answered questions "
              f"(answer contains '[' — a marker-presence proxy, not a "
              f"verified citation to the cited memory): {cites_yes} with "
              f"marker, {cites_no} without.")
    print(f"Overall ANANSI estimated tokens (ALL primary attempts, "
          f"including failed attempts): {total_anansi_est}")
    print(f"Overall naive estimated tokens (corresponding naive attempts, "
          f"A6 has no naive record): {total_naive_est}")
    print("Tokens per correctly answered question: unavailable here. These "
          "records lack independent answer grades for each approach; "
          "ANANSI's harness count cannot be used for the naive baseline.")
    print()
    print("## Secondary breakdown ONLY: successful cases (labelled, "
          "excludes failed attempts)")
    print("Selection uses diagnostic harness proxies, not the manual rubric.")
    print(f"Sample count (correct ANANSI attempts paired with naive): "
          f"{secondary_n}")
    print(f"ANANSI estimated tokens on successful cases only: "
          f"{secondary_anansi}")
    print(f"Naive estimated tokens on the same successful cases only: "
          f"{secondary_naive}")
    if secondary_naive:
        delta = secondary_anansi - secondary_naive
        print(f"Secondary delta: {delta:+d} "
              f"({100.0 * delta / secondary_naive:+.1f}%) — "
              f"this is NOT the overall result; it excludes "
              f"{total_attempts - total_correct_behaviour} failed "
              f"primary attempts.")
    print()
    print("## Full attempt denominator (all ANANSI Ask records)")
    all_recs = []
    for doc in docs:
        all_recs += [r for r in doc["records"] if r["approach"] == "anansi"]
    by_scenario = {}
    for r in all_recs:
        by_scenario.setdefault(r["scenario"], []).append(r)
    for sc in sorted(by_scenario):
        rows = by_scenario[sc]
        ok = sum(1 for r in rows if r.get("wire_checks_passed") is True)
        print(f"- {sc}: {len(rows)} record(s), wire_checks_passed true "
              f"on {ok}")
    print(f"Total ANANSI Ask records: {len(all_recs)}")
    print("Note: wire_checks_passed is true on honest recall-probe misses "
          "by design (A1/A3); the primary table above, not this count, "
          "is the correctness classification.")
    print()
    print("## Measurement types kept distinct")
    print("- Estimated: chars/4 from the Ask manifest (above).")
    if replay_path:
        replay = {}
        with open(replay_path, encoding="utf-8") as fh:
            for line in fh:
                line = line.strip()
                if not line:
                    continue
                obj = json.loads(line)
                replay[obj["prompt_file"]] = obj
        tot_in = tot_out = 0
        for case in PRIMARY:
            obj = replay.get(f"{case}-anansi.txt")
            if not obj:
                continue
            tot_in += obj.get("prompt_eval_count") or 0
            tot_out += obj.get("eval_count") or 0
            print(f"- replay {case}-anansi: input "
                  f"{obj.get('prompt_eval_count')}, output "
                  f"{obj.get('eval_count')}, done "
                  f"{obj.get('done_reason')}")
        print(f"Replay totals (ANANSI prompts, all primary cases replayed, "
              f"including failed cases): input {tot_in}, output {tot_out}, "
              f"total {tot_in + tot_out}")
        print("Replay tokens per correctly answered question: unavailable here; "
              "replayed answers require their own grades, independently of "
              "the original Ask run and the other approach.")
    else:
        print("- Direct replay: no --replay file supplied; replay totals "
              "are not claimed.")
    print("- Actual end-to-end provider usage (Ask): NOT recorded. Ask "
          "discards provider usage by design; replay above is a separate "
          "measurement for the identical prompt, not the ANANSI call's "
          "usage.")
    print("- Actual end-to-end provider usage (Collider, M5): see "
          "collider records below; disclosure/ledger, not estimates.")
    print()
    print("## Collider records (disclosure/ledger, not estimates)")
    for doc in docs:
        for r in doc["records"]:
            if r["scenario"].startswith(("C", "M")) and r["approach"] != "anansi":
                print(f"- {r['scenario']} {r['approach']}: status "
                      f"{r['status']}, provider-reported input "
                      f"{r.get('provider_reported_input_tokens')}, output "
                      f"{r.get('provider_reported_output_tokens')}, ledger "
                      f"input {r.get('ledger_input_tokens')}, output "
                      f"{r.get('ledger_output_tokens')}, notes "
                      f"{r.get('notes', '')[:200]}")


if __name__ == "__main__":
    main()
