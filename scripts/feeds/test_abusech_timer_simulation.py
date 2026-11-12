#!/usr/bin/env python3
"""
Abuse.ch Hourly Timer Simulation & Guard Verification Suite (Task V-43/V-44)

Verifies:
1. Reproduction of Q-31 defect under old timer (OnCalendar=hourly + RandomizedDelaySec=5min):
   Shows MB ~100% and TF ~63.2% across 100,000 simulated iterations due to intervals < 60m.
2. New monotonic timer verification (OnUnitInactiveSec=61min, AccuracySec=1min):
   Guarantees all intervals >= 61m from previous run DEACTIVATION (> 60m sliding window).
   Even with long-running executions (TF delayed up to ~580s near TimeoutStartSec 600s),
   all previous requests expire before next turn begins -> MB 100.0%, TF 100.0%.
3. Guard behavior with intervening manual requests:
   When manual requests reduce remaining capacity below 2, the entire turn is skipped
   cleanly (MB=0, TF=0), eliminating unfair starving of TF.
4. Timer unit file compliance:
   Verifies OnBootSec=5min, OnUnitInactiveSec=61min, AccuracySec=1min, no RandomizedDelaySec.
5. Atomic reservation under shared limiter lock.
"""
import collections
import configparser
import json
import pathlib
import random
import tempfile
import time
import unittest

HERE = pathlib.Path(__file__).resolve().parent
from abusech_rate_limit import RateLimiter, RateLimitError, MAX_PER_HOUR, HOUR


def simulate_old_timer(iterations=100_000, seed=42):
    """
    Simulates old systemd timer: OnCalendar=hourly + RandomizedDelaySec=5min.
    Delay is uniform(0, 300) seconds.
    Interval between run n and n+1 is 3600 + delay_{n+1} - delay_n.
    Sliding window is 3600 seconds with cap of 3 requests/hour.
    Old logic without guard: MB runs first (takes 1 slot if < 3), then TF runs (takes 1 slot if < 3).
    """
    rng = random.Random(seed)
    history = collections.deque()  # (timestamp, source)
    current_time = 0.0
    delay_prev = rng.uniform(0.0, 300.0)
    current_time += delay_prev

    mb_runs = 0
    tf_runs = 0

    for _ in range(iterations):
        # Clean history older than 1 hour (3600s)
        while history and current_time - history[0][0] >= HOUR:
            history.popleft()

        # Step 1: MalwareBazaar
        if len(history) < MAX_PER_HOUR:
            history.append((current_time, "malwarebazaar"))
            mb_runs += 1

        # Step 2: ThreatFox (runs immediately after MB)
        if len(history) < MAX_PER_HOUR:
            history.append((current_time + 0.1, "threatfox"))
            tf_runs += 1

        # Advance to next hour with independent jitter
        delay_next = rng.uniform(0.0, 300.0)
        current_time += 3600.0 + (delay_next - delay_prev)
        delay_prev = delay_next

    return {
        "iterations": iterations,
        "mb_runs": mb_runs,
        "tf_runs": tf_runs,
        "mb_pct": (mb_runs / iterations) * 100.0,
        "tf_pct": (tf_runs / iterations) * 100.0,
    }


def simulate_new_timer(iterations=100_000, max_tf_delay=580.0, seed=42):
    """
    Simulates new monotonic timer: OnUnitInactiveSec=61min + AccuracySec=1min.
    Timer counts 61 minutes from the moment the previous unit finishes (t_finish).
    Even when runs take a long time (e.g. ThreatFox delayed up to 580s near TimeoutStartSec 600s),
    all requests are guaranteed >= 61m old when the next turn begins.
    """
    rng = random.Random(seed)
    history = collections.deque()
    current_time = 300.0  # OnBootSec=5min

    mb_runs = 0
    tf_runs = 0

    for _ in range(iterations):
        while history and current_time - history[0][0] >= HOUR:
            history.popleft()

        # Guard: check if >= 2 slots available
        hourly_remaining = MAX_PER_HOUR - len(history)
        if hourly_remaining >= 2:
            mb_time = current_time + rng.uniform(1.0, 5.0)
            tf_time = current_time + rng.uniform(30.0, max_tf_delay)
            t_finish = tf_time + rng.uniform(1.0, 10.0)

            history.append((mb_time, "malwarebazaar"))
            history.append((tf_time, "threatfox"))
            mb_runs += 1
            tf_runs += 1
        else:
            t_finish = current_time + 1.0

        # Advance timer: OnUnitInactiveSec=61min counts from t_finish + accuracy jitter [0, 60s]
        accuracy_delay = rng.uniform(0.0, 60.0)
        current_time = t_finish + (61 * 60.0) + accuracy_delay

    return {
        "iterations": iterations,
        "mb_runs": mb_runs,
        "tf_runs": tf_runs,
        "mb_pct": (mb_runs / iterations) * 100.0,
        "tf_pct": (tf_runs / iterations) * 100.0,
    }


def simulate_new_timer_with_manual_interference(iterations=50_000, manual_prob=0.10, seed=42):
    """
    Simulates new timer with occasional manual requests intervening between automated runs.
    Demonstrates that with the guard, automated runs are either fully executed or fully skipped,
    maintaining 100% fairness (MB count == TF count).
    """
    rng = random.Random(seed)
    history = collections.deque()
    current_time = 300.0

    auto_mb_runs = 0
    auto_tf_runs = 0
    skipped_turns = 0

    for _ in range(iterations):
        # Possibly manual requests arrive between runs (1 or 2 requests)
        if rng.random() < manual_prob:
            num_manual = rng.choice([1, 2])
            for _ in range(num_manual):
                manual_time = current_time - rng.uniform(60.0, 1800.0)
                valid_h = [t for t, _ in history if manual_time - t < HOUR]
                if len(valid_h) < MAX_PER_HOUR:
                    history.append((manual_time, "manual_request"))

        # Clean history at current_time
        while history and current_time - history[0][0] >= HOUR:
            history.popleft()

        # Guard check
        remaining = MAX_PER_HOUR - len(history)
        if remaining >= 2:
            mb_time = current_time + rng.uniform(1.0, 5.0)
            tf_time = current_time + rng.uniform(10.0, 300.0)
            t_finish = tf_time + rng.uniform(1.0, 5.0)

            history.append((mb_time, "malwarebazaar"))
            history.append((tf_time, "threatfox"))
            auto_mb_runs += 1
            auto_tf_runs += 1
        else:
            t_finish = current_time + 1.0
            skipped_turns += 1

        current_time = t_finish + (61 * 60.0) + rng.uniform(0.0, 60.0)

    return {
        "iterations": iterations,
        "auto_mb_runs": auto_mb_runs,
        "auto_tf_runs": auto_tf_runs,
        "skipped_turns": skipped_turns,
        "mb_pct": (auto_mb_runs / iterations) * 100.0,
        "tf_pct": (auto_tf_runs / iterations) * 100.0,
    }


class TestAbusechTimerSimulation(unittest.TestCase):
    def test_reproduce_old_timer_defect(self):
        """Verifies that the old timer produces ~63.2% TF success rate reported in Q-31."""
        res = simulate_old_timer(iterations=50_000, seed=42)
        # MB gets 100% of runs
        self.assertEqual(res["mb_pct"], 100.0)
        # TF is severely degraded to ~63.2%
        self.assertAlmostEqual(res["tf_pct"], 63.2, delta=2.0)

    def test_new_timer_guarantees_100_percent_both_feeds(self):
        """Verifies that the new timer (>= 61 min interval from inactive) achieves 100.0% for both MB and TF."""
        res = simulate_new_timer(iterations=100_000, max_tf_delay=580.0, seed=42)
        self.assertEqual(res["mb_runs"], 100_000)
        self.assertEqual(res["tf_runs"], 100_000)
        self.assertEqual(res["mb_pct"], 100.0)
        self.assertEqual(res["tf_pct"], 100.0)

    def test_long_run_delayed_tf_near_timeout_guarantees_full_slots_next_run(self):
        """
        Verifies that when a run takes nearly TimeoutStartSec (600s), OnUnitInactiveSec=61min
        guarantees that next run starts >= 61m AFTER completion, so TF request is >= 61m old
        and both slots are available.
        """
        history = collections.deque()
        t_start = 10000.0
        mb_time = t_start + 5.0
        tf_time = t_start + 580.0  # TF delayed to 580s near 600s timeout!
        t_finish = tf_time + 10.0  # Run finishes at 590s

        history.append((mb_time, "malwarebazaar"))
        history.append((tf_time, "threatfox"))

        # Next run starts at t_finish + 61min
        next_run_start = t_finish + (61 * 60.0)

        # Difference between next_run_start and tf_time is:
        delta_tf = next_run_start - tf_time
        self.assertGreaterEqual(delta_tf, 61 * 60.0)
        self.assertGreater(delta_tf, HOUR)

        # At next_run_start, clean history
        valid_requests = [r for r in history if next_run_start - r[0] < HOUR]
        self.assertEqual(len(valid_requests), 0)
        remaining = MAX_PER_HOUR - len(valid_requests)
        self.assertEqual(remaining, 3)
        self.assertGreaterEqual(remaining, 2)

    def test_manual_interference_drops_entire_turn_preserving_fairness(self):
        """Verifies that with manual requests, turns are skipped in full, keeping MB == TF exactly."""
        res = simulate_new_timer_with_manual_interference(iterations=20_000, manual_prob=0.15, seed=42)
        # MB and TF must run the exact same number of times
        self.assertEqual(res["auto_mb_runs"], res["auto_tf_runs"])
        self.assertGreater(res["skipped_turns"], 0)
        self.assertEqual(res["auto_mb_runs"] + res["skipped_turns"], res["iterations"])

    def test_timer_unit_file_configuration(self):
        """Verifies that zonendpoint-abusech-update.timer strictly enforces OnUnitInactiveSec=61min."""
        timer_file = HERE / "zonendpoint-abusech-update.timer"
        self.assertTrue(timer_file.is_file())
        content = timer_file.read_text(encoding="utf-8")

        # Must have OnBootSec=5min
        self.assertIn("OnBootSec=5min", content)
        # Must have OnUnitInactiveSec=61min
        self.assertIn("OnUnitInactiveSec=61min", content)
        # Must NOT have OnUnitActiveSec
        self.assertNotIn("OnUnitActiveSec", content)
        # Must have AccuracySec=1min
        self.assertIn("AccuracySec=1min", content)
        # Must NOT have RandomizedDelaySec
        self.assertNotIn("RandomizedDelaySec", content)
        # Must NOT have OnCalendar=hourly
        self.assertNotIn("OnCalendar=hourly", content)

    def test_guard_reserves_slots_under_lock(self):
        """Verifies that reserve_slots atomically secures 2 slots and blocks manual usurpation."""
        with tempfile.TemporaryDirectory() as d:
            state_file = pathlib.Path(d) / "rate_limit.json"
            lim = RateLimiter(state_file, pathlib.Path(d) / "absent")

            # Initially 3 slots available
            stat = lim.get_status(now=1000)
            self.assertEqual(stat["hourly_remaining"], 3)

            # Reserve 2 slots
            token = lim.reserve_slots(count=2, sources=("malwarebazaar", "threatfox"), now=1000)
            self.assertIsNotNone(token)

            # Remaining unreserved slots is now 1
            stat = lim.get_status(now=1001)
            self.assertEqual(stat["hourly_remaining"], 1)

            # A manual request comes in and takes the 1 remaining slot
            lim_manual = RateLimiter(state_file, pathlib.Path(d) / "absent")
            lim_manual.check_and_record("malwarebazaar", now=1002)

            # Now 0 remaining slots for any further unreserved request
            stat = lim.get_status(now=1003)
            self.assertEqual(stat["hourly_remaining"], 0)

            # A 2nd manual request fails
            with self.assertRaises(RateLimitError):
                lim_manual.check_and_record("threatfox", now=1004)

            # BUT the automated instance can still execute its 2 reserved feeds!
            lim.check_and_record("malwarebazaar", now=1005)
            lim.check_and_record("threatfox", now=1006)

            # Confirm clean state
            raw = json.loads(state_file.read_text(encoding="utf-8"))
            self.assertEqual(len(raw["requests"]), 3)
            self.assertFalse(any(r.get("reserved", False) for r in raw["requests"]))


if __name__ == "__main__":
    unittest.main()
