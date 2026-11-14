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
import urllib.request
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
        mock_resp.geturl.return_value = "http://127.0.0.1/fake.gpg"
        mock_resp.headers = {"Content-Length": "15"}
        mock_resp.read.side_effect = [b"fake-key-block", b""]
        mock_opener = MagicMock()
        mock_opener.open.return_value.__enter__.return_value = mock_resp

        with patch("subprocess.run") as mock_run, \
             patch.object(ss_mod, "build_secure_opener", return_value=mock_opener), \
             patch.object(ss_mod, "get_keyring_fingerprints", return_value=[]):

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
        mock_resp.geturl.return_value = "http://127.0.0.1/huge.hdb"
        mock_resp.read.side_effect = [b"chunk", b""]
        mock_opener = MagicMock()
        mock_opener.open.return_value.__enter__.return_value = mock_resp
        with patch.object(ss_mod, "build_secure_opener", return_value=mock_opener):
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

    def test_15_keyring_unauthorized_key_aborts_immediately(self):
        """Q-49: Keyring containing foreign/unauthorized key causes immediate failure."""
        foreign_fpr = "1111222233334444555566667777888899990000"
        with patch.object(ss_mod, "get_keyring_fingerprints", return_value=[ss_mod.EXPECTED_FINGERPRINT, foreign_fpr]):
            with self.assertRaises(RuntimeError) as ctx:
                ss_mod.setup_gnupg_keyring(str(self.gnupg_home))
            self.assertIn("unauthorized key", str(ctx.exception))

    def test_16_signer_fingerprint_mismatch_rejects_signature(self):
        """Q-49: Signature verified with foreign signer fingerprint in VALIDSIG is rejected."""
        mock_res_valid = MagicMock()
        mock_res_valid.returncode = 0
        mock_res_valid.stdout = (
            f"[GNUPG:] NEWSIG\n"
            f"[GNUPG:] GOODSIG 97CD8EE6 Sanesecurity\n"
            f"[GNUPG:] VALIDSIG {ss_mod.EXPECTED_FINGERPRINT} 2026-11-14 1763138888 0 4 0 1 10 00 {ss_mod.EXPECTED_FINGERPRINT}\n"
        )
        mock_res_valid.stderr = ""

        mock_res_foreign = MagicMock()
        mock_res_foreign.returncode = 0
        mock_res_foreign.stdout = (
            "[GNUPG:] NEWSIG\n"
            "[GNUPG:] GOODSIG 11223344 Foreign Signer\n"
            "[GNUPG:] VALIDSIG 1111222233334444555566667777888899990000 2026-11-14 1763138888 0 4 0 1 10 00 1111222233334444555566667777888899990000\n"
        )
        mock_res_foreign.stderr = ""

        with patch("subprocess.run") as mock_run:
            # Case 1: Matching fingerprint -> True
            mock_run.return_value = mock_res_valid
            self.assertTrue(ss_mod.verify_gpg_signature(str(self.gnupg_home), "/tmp/test.hdb", "/tmp/test.hdb.sig"))

            # Case 2: Foreign fingerprint -> False
            mock_run.return_value = mock_res_foreign
            self.assertFalse(ss_mod.verify_gpg_signature(str(self.gnupg_home), "/tmp/test.hdb", "/tmp/test.hdb.sig"))

            # Case 3: Missing VALIDSIG -> False
            mock_res_invalid = MagicMock(returncode=1, stdout="[GNUPG:] BADSIG\n", stderr="Bad signature")
            mock_run.return_value = mock_res_invalid
            self.assertFalse(ss_mod.verify_gpg_signature(str(self.gnupg_home), "/tmp/test.hdb", "/tmp/test.hdb.sig"))

    def test_17_reject_redirect_to_http(self):
        """Q-49: Redirect downgrading to non-HTTPS scheme is rejected."""
        handler = ss_mod.StrictRedirectHandler(allowed_hosts=["mirror.seichter.de"])
        req = urllib.request.Request("https://mirror.seichter.de/sanesecurity/rogue.hdb")
        with patch.dict(os.environ, {"ZONENDPOINT_TEST_ENV": "0"}):
            with self.assertRaises(ValueError) as ctx:
                handler.redirect_request(req, None, 302, "Found", {}, "http://mirror.seichter.de/sanesecurity/rogue.hdb")
            self.assertIn("must use HTTPS", str(ctx.exception))

    def test_18_reject_cross_host_redirect(self):
        """Q-49: Redirect switching to unauthorized foreign host is rejected."""
        handler = ss_mod.StrictRedirectHandler(allowed_hosts=["mirror.seichter.de"])
        req = urllib.request.Request("https://mirror.seichter.de/sanesecurity/rogue.hdb")
        with self.assertRaises(ValueError) as ctx:
            handler.redirect_request(req, None, 302, "Found", {}, "https://evil-mirror.attacker.com/sanesecurity/rogue.hdb")
        self.assertIn("Cross-host redirect rejected", str(ctx.exception))

    def test_19_pubkey_size_exceeded_aborts(self):
        """Q-49: Public key exceeding 64 KiB size limit raises ValueError."""
        huge_key_file = self.base_path / "huge_key.gpg"
        huge_key_file.write_bytes(b"A" * (65 * 1024))  # 65 KiB
        with self.assertRaises(ValueError) as ctx:
            ss_mod.setup_gnupg_keyring(str(self.gnupg_home), pubkey_source=str(huge_key_file))
        self.assertIn("exceeds limit 65536 bytes", str(ctx.exception))

    def test_20_batch_deploy_failure_at_file_n_restores_all_and_holds(self):
        """Q-49: Failure at Nth file in batch triggers immediate HOLD and restores all files from snapshot."""
        staged_1 = self.base_path / "rogue.hdb"
        staged_1.write_text("new_rogue_sig\n")
        staged_2 = self.base_path / "malwarehash.hsb"
        staged_2.write_text("new_malware_sig\n")

        # Active dir initially has rogue.hdb with special mode, but not malwarehash.hsb
        active_1 = self.active_dir / "rogue.hdb"
        active_1.write_text("old_rogue_sig\n")
        active_1.chmod(0o600)

        active_2 = self.active_dir / "malwarehash.hsb"
        self.assertFalse(active_2.exists())

        # Mock safe_atomic_write to fail when target is active_2 (2nd file in batch)
        orig_safe_atomic = ss_mod.safe_atomic_write

        def failing_atomic_write(target_path, source_file, **kwargs):
            if "malwarehash.hsb" in target_path:
                raise IOError("Simulated disk error on 2nd file")
            orig_safe_atomic(target_path, source_file, **kwargs)

        with patch.object(ss_mod, "safe_atomic_write", side_effect=failing_atomic_write), \
             patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=0)

            with self.assertRaises(RuntimeError) as ctx:
                ss_mod.deploy_batch(
                    staged_files=[str(staged_1), str(staged_2)],
                    active_dir=str(self.active_dir),
                    backup_dir=str(self.backup_dir),
                    hold_file=str(self.hold_file),
                    reload_state_file=str(self.reload_state_file),
                    last_run_file=str(self.last_run_file),
                    clamdscan_bin="echo",
                    clamd_host="127.0.0.1",
                    clamd_port=9999,
                )
            self.assertIn("Batch deployment failed", str(ctx.exception))

            # 1. HOLD file was placed
            self.assertTrue(self.hold_file.is_file())

            # 2. 1st file was restored back to old content
            self.assertEqual(active_1.read_text(), "old_rogue_sig\n")
            if os.name == "posix":
                self.assertEqual(active_1.stat().st_mode & 0o777, 0o600)

            # 3. 2nd file is not present in active dir
            self.assertFalse(active_2.exists())

            # 4. last-run.json recorded failure
            last_run = json.loads(self.last_run_file.read_text(encoding="utf-8"))
            self.assertEqual(last_run["status"], "batch_deploy_failed")
            self.assertEqual(last_run["failed_file"], "malwarehash.hsb")


if __name__ == "__main__":
    unittest.main()
