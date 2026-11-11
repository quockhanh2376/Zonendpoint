import json
import pathlib
import tempfile
import unittest

from abusech_rate_limit import RateLimitError, RateLimiter


class SharedBudgetTests(unittest.TestCase):
    def test_combined_hourly_limit_alternation_and_legacy_migration(self):
        with tempfile.TemporaryDirectory() as d:
            shared, old = pathlib.Path(d)/"shared.json", pathlib.Path(d)/"old.json"
            old.write_text(json.dumps({"requests": [10]}))
            lim = RateLimiter(shared, old)
            lim.check_and_record("threatfox", now=20)  # migrated MB request makes TF next
            records = json.loads(shared.read_text())["requests"]
            self.assertEqual([r["source"] for r in records], ["malwarebazaar", "threatfox"])
            shared.unlink()
            old.unlink()
            lim.check_and_record("malwarebazaar", now=30)
            lim.check_and_record("threatfox", now=31)
            lim.check_and_record("malwarebazaar", now=32)
            with self.assertRaises(RateLimitError):
                lim.check_and_record("threatfox", now=33)

    def test_combined_daily_cap(self):
        with tempfile.TemporaryDirectory() as d:
            state = pathlib.Path(d)/"shared.json"
            state.write_text(json.dumps({"requests": [{"timestamp": 96000, "source": "malwarebazaar"} for _ in range(400)]}))
            with self.assertRaises(RateLimitError):
                RateLimiter(state, pathlib.Path(d)/"absent").check_and_record("threatfox", now=100000)

    def test_invalid_json_or_missing_requests_refuses_without_overwrite(self):
        with tempfile.TemporaryDirectory() as d:
            state = pathlib.Path(d)/"shared.json"
            cases = [
                '"corrupted_string"',
                "12345",
                "[1, 2, 3]",
                "null",
                '{"version": 1}',  # missing "requests"
                '{"requests": "not_a_list"}',
            ]
            for bad_payload in cases:
                state.write_text(bad_payload, encoding="utf-8")
                lim = RateLimiter(state, pathlib.Path(d)/"absent")
                with self.assertRaises(ValueError):
                    lim.check_and_record("malwarebazaar", now=1000)
                # Verify state file was NOT overwritten with new requests
                self.assertEqual(state.read_text(encoding="utf-8"), bad_payload)

    def test_empty_hourly_with_other_waiting_prioritizes_malwarebazaar(self):
        with tempfile.TemporaryDirectory() as d:
            state = pathlib.Path(d)/"shared.json"
            lim = RateLimiter(state, pathlib.Path(d)/"absent", other_waiting=True)

            # 1. When hourly window is empty and both waiting, ThreatFox MUST NOT run first
            with self.assertRaises(RateLimitError) as ctx:
                lim.check_and_record("threatfox", now=100)
            self.assertIn("malwarebazaar", str(ctx.exception))

            # 2. MalwareBazaar is allowed to run first
            lim.check_and_record("malwarebazaar", now=100)

            # 3. Next turn belongs to ThreatFox
            with self.assertRaises(RateLimitError) as ctx:
                lim.check_and_record("malwarebazaar", now=200)
            self.assertIn("threatfox", str(ctx.exception))

            lim.check_and_record("threatfox", now=200)

            # 4. Alternates back to MalwareBazaar
            with self.assertRaises(RateLimitError) as ctx:
                lim.check_and_record("threatfox", now=300)
            self.assertIn("malwarebazaar", str(ctx.exception))

            lim.check_and_record("malwarebazaar", now=300)


if __name__ == "__main__":
    unittest.main()
