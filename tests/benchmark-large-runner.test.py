"""Offline checks: frozen benchmark tasks never transmit gold answers."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location(
    "large_runner", Path(__file__).resolve().parents[1] / "scripts" / "benchmark-large-openrouter.py"
)
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


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


if __name__ == "__main__":
    unittest.main()
