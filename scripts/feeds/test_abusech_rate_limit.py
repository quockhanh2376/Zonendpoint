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

    def test_reserve_slots_success_and_fulfillment(self):
        with tempfile.TemporaryDirectory() as d:
            state = pathlib.Path(d) / "shared.json"
            lim = RateLimiter(state, pathlib.Path(d) / "absent")

            # Reserve 2 slots
            token = lim.reserve_slots(count=2, sources=("malwarebazaar", "threatfox"), now=1000)
            self.assertTrue(token.startswith("res_"))
            stat = lim.get_status(now=1005)
            self.assertEqual(stat["hourly_used"], 2)
            self.assertEqual(stat["hourly_remaining"], 1)

            # Fulfill MalwareBazaar
            lim.check_and_record("malwarebazaar", now=1010)
            stat = lim.get_status(now=1015)
            self.assertEqual(stat["hourly_used"], 2)
            self.assertEqual(stat["hourly_remaining"], 1)

            # Fulfill ThreatFox
            lim.check_and_record("threatfox", now=1020)
            stat = lim.get_status(now=1025)
            self.assertEqual(stat["hourly_used"], 2)
            self.assertEqual(stat["hourly_remaining"], 1)

            # Confirm file has both fulfilled records without reserved flag
            records = json.loads(state.read_text())["requests"]
            self.assertEqual(len(records), 2)
            self.assertFalse(any(r.get("reserved", False) for r in records))
            self.assertFalse(any("token" in r for r in records))

    def test_reserve_slots_fails_when_insufficient_hourly(self):
        with tempfile.TemporaryDirectory() as d:
            state = pathlib.Path(d) / "shared.json"
            lim = RateLimiter(state, pathlib.Path(d) / "absent")

            # Seed 2 requests in the hour (only 1 remaining)
            lim.check_and_record("malwarebazaar", now=1000)
            lim.check_and_record("threatfox", now=1100)

            # Attempt to reserve 2 slots -> fails
            with self.assertRaises(RateLimitError) as ctx:
                lim.reserve_slots(count=2, sources=("malwarebazaar", "threatfox"), now=1200)
            self.assertIn("Shared hourly rate limit reached", str(ctx.exception))
            self.assertIn("only 1/3 remaining", str(ctx.exception))

    def test_reserve_slots_blocks_intervening_manual_request(self):
        with tempfile.TemporaryDirectory() as d:
            state = pathlib.Path(d) / "shared.json"
            lim_auto = RateLimiter(state, pathlib.Path(d) / "absent")
            lim_manual = RateLimiter(state, pathlib.Path(d) / "absent")

            # Prior request used 1 slot (2 remaining)
            lim_manual.check_and_record("malwarebazaar", now=1000)

            # Automated update reserves remaining 2 slots
            token = lim_auto.reserve_slots(count=2, sources=("malwarebazaar", "threatfox"), now=1050)
            self.assertIsNotNone(token)

            # Manual request tries to run while slots are reserved -> blocked (3/3 used)
            with self.assertRaises(RateLimitError) as ctx:
                lim_manual.check_and_record("threatfox", now=1060)
            self.assertIn("rate limit exceeded: 3/3 requests", str(ctx.exception))

            # Both automated feeds still succeed because their slots were reserved!
            lim_auto.check_and_record("malwarebazaar", now=1070)
            lim_auto.check_and_record("threatfox", now=1080)

    def test_cancel_unconsumed_reservations(self):
        with tempfile.TemporaryDirectory() as d:
            state = pathlib.Path(d) / "shared.json"
            lim = RateLimiter(state, pathlib.Path(d) / "absent")

            # Reserve 2 slots
            lim.reserve_slots(count=2, sources=("malwarebazaar", "threatfox"), now=1000)
            self.assertEqual(lim.get_status(now=1005)["hourly_remaining"], 1)

            # Fulfill only MalwareBazaar
            lim.check_and_record("malwarebazaar", now=1010)

            # ThreatFox aborted; clean up unconsumed
            lim.cancel_unconsumed_reservations()

            # ThreatFox reservation removed; only MB remains -> 2 slots remaining!
            stat = lim.get_status(now=1015)
            self.assertEqual(stat["hourly_used"], 1)
            self.assertEqual(stat["hourly_remaining"], 2)


if __name__ == "__main__":
    unittest.main()
