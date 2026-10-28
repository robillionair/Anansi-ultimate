"""Regression checks for measurement/grade separation in the report CLI."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


REPORT = Path(__file__).resolve().parents[1] / "scripts" / "bench-report.py"


class ReportCliTests(unittest.TestCase):
    def run_report(self, *arguments):
        return subprocess.run(
            [sys.executable, str(REPORT), *map(str, arguments)],
            capture_output=True,
            text=True,
            encoding="utf-8",
            env={**os.environ, "PYTHONUTF8": "1"},
            check=False,
        )

    def fixture(self, directory):
        result = Path(directory) / "results.json"
        replay = Path(directory) / "replay.jsonl"
        result.write_text(json.dumps({
            "mode": "real_model",
            "records": [
                {"scenario": "A2", "approach": "anansi", "status": "Answered",
                 "wire_checks_passed": True, "anansi_estimate_tokens": 10,
                 "notes": "real_answer_fact=true;real_answer_cites_marker=true"},
                {"scenario": "A2", "approach": "baseline_naive_full_bodies",
                 "status": "synthetic", "anansi_estimate_tokens": 20},
            ],
        }), encoding="utf-8")
        replay.write_text(
            json.dumps({"prompt_file": "A2-anansi.txt", "prompt_eval_count": 12,
                        "eval_count": 4, "done_reason": "stop"}) + "\n" +
            json.dumps({"prompt_file": "A2-naive.txt", "prompt_eval_count": 25,
                        "eval_count": 8, "done_reason": "stop"}) + "\n",
            encoding="utf-8",
        )
        return result, replay

    def test_ungraded_baseline_does_not_borrow_anansi_correct_count(self):
        with tempfile.TemporaryDirectory(prefix="anansi-report-review-") as directory:
            result, _ = self.fixture(directory)
            run = self.run_report(result)
            self.assertEqual(run.returncode, 0, run.stderr)
            self.assertNotIn("same denominator", run.stdout)
            self.assertIn("independent answer grades", run.stdout)
            self.assertIn("harness proxies", run.stdout)

    def test_replay_option_and_measurements_do_not_reuse_original_grades(self):
        with tempfile.TemporaryDirectory(prefix="anansi-report-review-") as directory:
            result, replay = self.fixture(directory)
            original = result.read_bytes(), replay.read_bytes()
            for args in [(result, "--replay", replay), ("--replay", replay, result)]:
                run = self.run_report(*args)
                self.assertEqual(run.returncode, 0, run.stderr)
                self.assertIn("input 12, output 4, total 16", run.stdout)
                self.assertNotIn("failed-attempt replay included", run.stdout)
                self.assertIn("replayed answers require their own grades", run.stdout)
            self.assertEqual(original, (result.read_bytes(), replay.read_bytes()))


if __name__ == "__main__":
    unittest.main()
