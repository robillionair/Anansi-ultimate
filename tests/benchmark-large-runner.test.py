"""Offline checks: frozen benchmark tasks never transmit gold answers."""
import importlib.util
from pathlib import Path
import unittest

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


if __name__ == "__main__":
    unittest.main()
