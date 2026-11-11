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


if __name__ == "__main__":
    unittest.main()
