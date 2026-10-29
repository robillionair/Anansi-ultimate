"""Offline checks: frozen benchmark tasks never transmit gold answers."""
import importlib.util
import contextlib
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location(
    "large_runner", Path(__file__).resolve().parents[1] / "scripts" / "benchmark-large-openrouter.py"
)
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)
score_spec = importlib.util.spec_from_file_location(
    "large_score", Path(__file__).resolve().parents[1] / "scripts" / "score-large-benchmark.py"
)
scorer = importlib.util.module_from_spec(score_spec)
score_spec.loader.exec_module(scorer)


class RunnerTests(unittest.TestCase):
    def test_only_public_question_and_context_are_transmitted(self):
        case = {"id": "Q1", "question": "What is the value?",
                "expected": {"secret_gold": "GOLD_MUST_NOT_BE_SENT"},
                "approaches": {"anansi": {"prompt": "[1] source value", "sources": []}}}
        payload = runner.payload_for([case], "anansi")
        self.assertNotIn("GOLD_MUST_NOT_BE_SENT", str(payload))
        self.assertEqual(payload["model"], runner.MODEL)
        self.assertFalse(payload["provider"]["allow_fallbacks"])
        self.assertEqual(payload["temperature"], 0)
        self.assertEqual(payload["max_tokens"], 4096)

    def test_json_and_fenced_json_are_equivalent(self):
        text = '{"answers":[{"id":"Q1","answer":"42","citations":[1],"insufficient":false}]}'
        self.assertEqual(runner.parse_answers(text), runner.parse_answers("```json\n" + text + "\n```"))
        with self.assertRaises(ValueError):
            runner.parse_answers('{"answers":{}}')

    def test_numerical_match_has_boundaries_and_requires_visible_support(self):
        self.assertFalse(scorer.contains_fact("130", "30"))
        case = {"expected": {"kind": "exact", "answer": "30"},
                "evidence": {"supportingMemoryIds": ["m1"], "exactFactSpans": ["Capacity is 30."]},
                "approaches": {"basic": {"sources": [{"index": 1, "memoryId": "m1", "text": "Truncated notes"}]}}}
        result = scorer.grade(case, "basic", {"answer": "30", "citations": [1], "insufficient": False})
        self.assertTrue(result["fact_correct"])
        self.assertFalse(result["grounded_correct"])

    def test_absent_responses_keep_full_denominator(self):
        case = {"id": "Q1", "category": "insufficient",
                "expected": {"kind": "insufficient", "answer": "INSUFFICIENT_EVIDENCE"},
                "evidence": {"supportingMemoryIds": [], "exactFactSpans": []},
                "approaches": {a: {"sources": []} for a in ["anansi", "basic", "full"]}}
        result = scorer.score({"cases": [case]}, [])
        for approach in result["totals"].values():
            self.assertEqual(approach["denominator"], 1)
            self.assertEqual(approach["fact_correct"], 0)

    def run_preflight_failure(self, priced=False):
        with tempfile.TemporaryDirectory(dir=os.environ.get("TEMP")) as directory:
            root = Path(directory)
            fixture = root / "fixture.json"
            fixture.write_text(json.dumps({"cases": [
                {"id": f"Q{i}", "question": "value?", "approaches": {
                    a: {"prompt": "synthetic"} for a in runner.APPROACHES}}
                for i in range(100)]}), encoding="utf-8")
            calls = []

            def fake_open(request, **kwargs):
                calls.append(request.full_url)
                if request.full_url.endswith("/models"):
                    data = {"data": [{"id": runner.MODEL,
                        "pricing": {"prompt": "1" if priced else "0", "completion": "0"}}]}
                else:
                    data = {"model": runner.MODEL, "usage": {"cost": 1},
                        "choices": [{"finish_reason": "stop", "message": {"content": '{"answers":[]}'}}]}
                return contextlib.closing(io.StringIO(json.dumps(data)))

            with patch.dict(os.environ, {"OPENROUTER_API_KEY": "TEST_SECRET_NOT_TO_BE_SAVED"}), \
                 patch("sys.argv", ["runner", str(fixture), str(root / "out")]), \
                 patch.object(runner.urllib.request, "urlopen", fake_open), \
                 contextlib.redirect_stdout(io.StringIO()):
                with self.assertRaises((ValueError, RuntimeError)):
                    runner.main()
            for artifact in (root / "out").glob("*"):
                self.assertNotIn("TEST_SECRET_NOT_TO_BE_SAVED", artifact.read_text(encoding="utf-8"))
            return calls

    def test_nonfree_metadata_prevents_inference(self):
        calls = self.run_preflight_failure(priced=True)
        self.assertEqual(len(calls), 1)
        self.assertTrue(calls[0].endswith("/models"))

    def test_unexpected_nonzero_charge_stops_without_retry(self):
        calls = self.run_preflight_failure()
        self.assertEqual(sum(url.endswith("/chat/completions") for url in calls), 1)


if __name__ == "__main__":
    unittest.main()
