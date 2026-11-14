#!/usr/bin/env python3
"""
Unit tests for Zonendpoint Sanesecurity feed updater and rollback script (Task V-68).
All tests strictly mock HTTP, GPG, and clamd interactions.
ZONENDPOINT_TEST_ENV=1 is enforced throughout.
"""

import os
import sys
import json
import time
import socket
import pathlib
import tempfile
import unittest
import subprocess
import importlib.util
from unittest.mock import patch, MagicMock

# Enforce safety guard environment
os.environ["ZONENDPOINT_TEST_ENV"] = "1"

HERE = pathlib.Path(__file__).resolve().parent
SS_SCRIPT = HERE / "zonendpoint-sanesecurity.py"
SS_ROLLBACK_SCRIPT = HERE / "zonendpoint-sanesecurity-rollback.sh"

_ss_spec = importlib.util.spec_from_file_location("zonendpoint_sanesecurity", SS_SCRIPT)
ss_mod = importlib.util.module_from_spec(_ss_spec)
_ss_spec.loader.exec_module(ss_mod)


class TestSanesecurityUpdater(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.TemporaryDirectory()
        self.base_path = pathlib.Path(self.temp_dir.name)
        self.state_dir = self.base_path / "opt_ss"
        self.gnupg_home = self.base_path / "gnupg"
        self.state_file = self.state_dir / "state.json"
        self.last_run_file = self.state_dir / "last-run.json"
        self.hold_file = self.base_path / "HOLD-sanesecurity"
        self.lock_file = self.base_path / "deploy.lock"
        self.reload_state_file = self.base_path / "reload-state.json"
        self.active_dir = self.base_path / "clamav"
        self.backup_dir = self.base_path / "backup"
        self.log_file = self.base_path / "clamav.log"

        self.state_dir.mkdir(parents=True, exist_ok=True)
        self.active_dir.mkdir(parents=True, exist_ok=True)
        self.backup_dir.mkdir(parents=True, exist_ok=True)

    def tearDown(self):
        self.temp_dir.cleanup()

    def test_01_safety_guard_blocks_real_port_3310_in_test_env(self):
        """Safety Guard: Fails immediately if attempting to target port 3310 in test environment."""
        with self.assertRaises(AssertionError) as ctx:
            ss_mod.check_safety_guard(port=3310)
        self.assertIn("SAFETY VIOLATION: Test targeted real clamd port 3310!", str(ctx.exception))

        with self.assertRaises(AssertionError) as ctx:
            ss_mod.check_safety_guard(clamdscan_bin="clamdscan")
        self.assertIn("SAFETY VIOLATION: Test targeted real clamdscan binary!", str(ctx.exception))

    def test_02_hold_file_skips_run_cleanly(self):
        """Task L-3: Updater exits 0 without contacting mirror when HOLD file exists."""
        self.hold_file.write_text("2026-11-14T20:00:00Z\n", encoding="utf-8")
        cmd = [
            sys.executable,
            str(SS_SCRIPT),
            "--hold-file", str(self.hold_file),
            "--last-run-file", str(self.last_run_file),
            "--lock-file", str(self.lock_file),
            "--clamd-port", "9999",
            "--clamdscan-bin", "echo",
        ]
        res = subprocess.run(cmd, capture_output=True, text=True)
        self.assertEqual(res.returncode, 0, f"Stdout: {res.stdout}\nStderr: {res.stderr}")
        self.assertIn("Hold file", res.stdout)
        self.assertTrue(self.last_run_file.is_file())
        data = json.loads(self.last_run_file.read_text(encoding="utf-8"))
        self.assertEqual(data.get("status"), "hold")

    def test_03_shared_deploy_lock_aborts_code_3_when_held(self):
        """Task L-6: Updater exits with code 3 when shared deploy lock is held."""
        import fcntl
        with open(str(self.lock_file), "w") as fd:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            cmd = [
                sys.executable,
                str(SS_SCRIPT),
                "--lock-file", str(self.lock_file),
                "--last-run-file", str(self.last_run_file),
                "--hold-file", str(self.hold_file),
                "--clamd-port", "9999",
                "--clamdscan-bin", "echo",
            ]
            res = subprocess.run(cmd, capture_output=True, text=True)
            self.assertEqual(res.returncode, 3, f"Stdout: {res.stdout}\nStderr: {res.stderr}")
            self.assertIn("Another clamd deploy/update process is currently running", res.stdout)
            fcntl.flock(fd, fcntl.LOCK_UN)

    def test_04_reject_insecure_non_https_mirror(self):
        """Security: Non-HTTPS mirror URL rejected unless local test mock."""
        with patch.dict(os.environ, {"ZONENDPOINT_TEST_ENV": "0"}):
            with self.assertRaises(ValueError) as ctx:
                ss_mod.validate_mirror_url("http://sanesecurity.example.com/mirror")
            self.assertIn("must use HTTPS", str(ctx.exception))

    def test_05_http_304_not_modified_skips_download_and_reload(self):
        """HTTP 304: Preserves state and exits cleanly without reload when files unchanged."""
        with patch.object(ss_mod, "setup_gnupg_keyring", return_value=True), \
             patch.object(ss_mod, "download_conditional", return_value=(False, '"etag-123"', "Fri, 13 Nov 2026")), \
             patch.object(ss_mod, "reload_clamd_and_verify") as mock_reload:

            args = ss_mod.parse_args([])
            args.state_dir = str(self.state_dir)
            args.state_file = str(self.state_file)
            args.last_run_file = str(self.last_run_file)
            args.hold_file = str(self.hold_file)
            args.lock_file = str(self.lock_file)
            args.force_check = True
            args.clamd_port = 9999
            args.clamdscan_bin = "echo"

            with patch.object(ss_mod, "parse_args", return_value=args), \
                 patch("sys.exit", side_effect=SystemExit) as mock_exit:
                with self.assertRaises(SystemExit):
                    ss_mod.main()
                mock_reload.assert_not_called()
                last_run = json.loads(self.last_run_file.read_text(encoding="utf-8"))
                self.assertEqual(last_run["status"], "no_updates")

    def test_06_gpg_fingerprint_mismatch_aborts_immediately(self):
        """GPG: Key with wrong fingerprint causes immediate failure."""
        mock_resp = MagicMock()
        mock_resp.read.return_value = b"fake-key-block"

        with patch("subprocess.run") as mock_run, \
             patch("urllib.request.urlopen") as mock_url:
            mock_url.return_value.__enter__.return_value = mock_resp

            # Mock gpg --show-keys returning wrong fingerprint
            mock_res = MagicMock()
            mock_res.returncode = 0
            mock_res.stdout = "pub:...\nfpr:::::::::AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA:\n"
            mock_run.return_value = mock_res

            with self.assertRaises(RuntimeError) as ctx:
                ss_mod.setup_gnupg_keyring(str(self.gnupg_home), pubkey_source="http://127.0.0.1/fake.gpg")
            self.assertIn("fingerprint mismatch", str(ctx.exception))

    def test_07_gpg_verification_failure_rejects_file_preserves_old(self):
        """GPG: File with invalid detached signature is skipped, existing file preserved."""
        existing_rogue = self.active_dir / "rogue.hdb"
        existing_rogue.write_text("existing_valid_sig:1:name\n")

        with patch.object(ss_mod, "setup_gnupg_keyring", return_value=True), \
             patch.object(ss_mod, "download_conditional", return_value=(True, '"etag-new"', "Sat, 14 Nov 2026")), \
             patch.object(ss_mod, "verify_gpg_signature", return_value=False), \
             patch.object(ss_mod, "preflight_test", return_value=True), \
             patch.object(ss_mod, "reload_clamd_and_verify") as mock_reload:

            args = ss_mod.parse_args([])
            args.state_dir = str(self.state_dir)
            args.state_file = str(self.state_file)
            args.last_run_file = str(self.last_run_file)
            args.hold_file = str(self.hold_file)
            args.lock_file = str(self.lock_file)
            args.active_dir = str(self.active_dir)
            args.backup_dir = str(self.backup_dir)
            args.force_check = True
            args.clamd_port = 9999
            args.clamdscan_bin = "echo"

            with patch.object(ss_mod, "parse_args", return_value=args), \
                 patch("sys.exit", side_effect=SystemExit) as mock_exit:
                with self.assertRaises(SystemExit):
                    ss_mod.main()
                # Existing file was preserved
                self.assertEqual(existing_rogue.read_text(), "existing_valid_sig:1:name\n")
                last_run = json.loads(self.last_run_file.read_text(encoding="utf-8"))
                self.assertIn("failed", last_run)
                mock_reload.assert_not_called()

    def test_08_exceeds_max_file_size_aborts_file(self):
        """Limit: Content-Length > 20 MiB raises ValueError."""
        mock_resp = MagicMock()
        mock_resp.headers = {"Content-Length": str(25 * 1024 * 1024)}  # 25 MiB
        mock_resp.read.side_effect = [b"chunk", b""]
        with patch("urllib.request.urlopen") as mock_url:
            mock_url.return_value.__enter__.return_value = mock_resp
            with self.assertRaises(ValueError) as ctx:
                ss_mod.download_conditional("http://127.0.0.1/huge.hdb", str(self.base_path / "huge.hdb"))
            self.assertIn("exceeds limit", str(ctx.exception))

    def test_09_preflight_clamscan_error_aborts_deployment(self):
        """Task L-9: Syntax or load errors in preflight abort deployment without touching active dir."""
        staged_bad = self.base_path / "bad.hdb"
        staged_bad.write_text("invalid_signature_format\n")

        with patch("subprocess.run") as mock_run:
            mock_res = MagicMock()
            mock_res.returncode = 1
            mock_res.stderr = "LibClamAV Error: malformed signature"
            mock_res.stdout = ""
            mock_run.return_value = mock_res

            res = ss_mod.preflight_test([str(staged_bad)], clamscan_bin="echo")
            self.assertFalse(res)

    def test_10_reload_interval_deferred(self):
        """Task L-8: Minimum reload interval defers clamd reload when within 6.0h window."""
        recent_ts = time.time() - 3600.0  # 1 hour ago
        self.reload_state_file.write_text(json.dumps({"last_reload_timestamp": recent_ts}), encoding="utf-8")

        can_reload, elapsed = ss_mod.check_reload_interval(str(self.reload_state_file), min_interval_hours=6.0)
        self.assertFalse(can_reload)
        self.assertAlmostEqual(elapsed, 1.0, delta=0.1)

    def test_11_emergency_rollback_and_hold_on_reload_failure(self):
        """Emergency rollback: Triggers backup restoration and creates HOLD file if health check fails."""
        (self.backup_dir / "rogue.hdb.bak").write_text("good_backup_sig:1:name\n")
        active_rogue = self.active_dir / "rogue.hdb"
        active_rogue.write_text("bad_new_sig:1:name\n")

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            ss_mod.emergency_rollback(
                backup_dir=str(self.backup_dir),
                active_dir=str(self.active_dir),
                hold_file=str(self.hold_file),
                reload_state_file=str(self.reload_state_file),
                clamdscan_bin="echo",
                clamd_host="127.0.0.1",
                clamd_port=9999,
            )

            # HOLD file must exist
            self.assertTrue(self.hold_file.is_file())
            # Active file restored to backup
            self.assertEqual(active_rogue.read_text(), "good_backup_sig:1:name\n")
            # Reload state recorded forced
            rstate = json.loads(self.reload_state_file.read_text(encoding="utf-8"))
            self.assertEqual(rstate["caller"], "sanesecurity_emergency_rollback")
            self.assertTrue(rstate["forced"])

    def test_12_rollback_script_hold_and_restore(self):
        """Task L-3 / Shell Rollback: zonendpoint-sanesecurity-rollback.sh places HOLD and restores backups."""
        (self.backup_dir / "rogue.hdb.bak").write_text("restored_backup_sig:1:name\n")
        active_rogue = self.active_dir / "rogue.hdb"
        active_rogue.write_text("current_broken_sig:1:name\n")

        env = os.environ.copy()
        env["ZONENDPOINT_TEST_ENV"] = "1"
        env["DEPLOY_LOCK_FILE"] = str(self.lock_file)
        env["HOLD_FILE"] = str(self.hold_file)
        env["BACKUP_DIR"] = str(self.backup_dir)
        env["ACTIVE_DIR"] = str(self.active_dir)
        env["RELOAD_STATE_FILE"] = str(self.reload_state_file)
        env["CLAMD_PORT"] = "9999"
        env["CLAMDSCAN_BIN"] = "echo"
        env["CLAMD_NO_PING_WAIT"] = "1"

        proc = subprocess.run(["bash", str(SS_ROLLBACK_SCRIPT)], env=env, capture_output=True, text=True)
        self.assertEqual(proc.returncode, 0, f"Stderr: {proc.stderr}\nStdout: {proc.stdout}")
        self.assertTrue(self.hold_file.is_file())
        self.assertEqual(active_rogue.read_text(), "restored_backup_sig:1:name\n")

    def test_13_rollback_script_purge(self):
        """Shell Rollback: --purge creates HOLD and removes all Sanesecurity files."""
        for f in ["rogue.hdb", "malwarehash.hsb", "foxhole_generic.cdb"]:
            (self.active_dir / f).write_text("dummy\n")

        env = os.environ.copy()
        env["ZONENDPOINT_TEST_ENV"] = "1"
        env["DEPLOY_LOCK_FILE"] = str(self.lock_file)
        env["HOLD_FILE"] = str(self.hold_file)
        env["ACTIVE_DIR"] = str(self.active_dir)
        env["RELOAD_STATE_FILE"] = str(self.reload_state_file)
        env["CLAMD_PORT"] = "9999"
        env["CLAMDSCAN_BIN"] = "echo"
        env["CLAMD_NO_PING_WAIT"] = "1"

        proc = subprocess.run(["bash", str(SS_ROLLBACK_SCRIPT), "--purge"], env=env, capture_output=True, text=True)
        self.assertEqual(proc.returncode, 0)
        self.assertTrue(self.hold_file.is_file())
        for f in ["rogue.hdb", "malwarehash.hsb", "foxhole_generic.cdb"]:
            self.assertFalse((self.active_dir / f).exists())

    def test_14_min_check_interval_skips_run(self):
        """Rate limit: Min check interval of 2 hours halts before querying mirror."""
        recent_check = time.time() - 1800.0  # 30 minutes ago
        self.state_file.write_text(json.dumps({"last_check_timestamp": recent_check}), encoding="utf-8")

        can_check, elapsed = ss_mod.check_min_check_interval(str(self.state_file), min_interval_hours=2.0)
        self.assertFalse(can_check)
        self.assertAlmostEqual(elapsed, 0.5, delta=0.1)


if __name__ == "__main__":
    unittest.main()
