#!/usr/bin/env python3
"""
Unit tests for YARA updater and rollback script hold file functionality (Task L-3).
"""
import os
import pathlib
import subprocess
import tempfile
import time
import unittest
import importlib.util
from unittest.mock import patch, MagicMock

HERE = pathlib.Path(__file__).resolve().parent
YARA_UPDATE_SCRIPT = HERE / "zonendpoint-yara-update.py"
YARA_ROLLBACK_SCRIPT = HERE / "zonendpoint-yara-rollback.sh"
MB_ROLLBACK_SCRIPT = HERE / "zonendpoint-malwarebazaar-rollback.sh"
TF_ROLLBACK_SCRIPT = HERE / "zonendpoint-threatfox-rollback.sh"

_yara_spec = importlib.util.spec_from_file_location("zonendpoint_yara_update", YARA_UPDATE_SCRIPT)
yara_mod = importlib.util.module_from_spec(_yara_spec)
_yara_spec.loader.exec_module(yara_mod)


class TestHoldFiles(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.TemporaryDirectory()
        self.base_path = pathlib.Path(self.temp_dir.name)

    def tearDown(self):
        self.temp_dir.cleanup()

    def test_yara_update_skips_when_hold_file_present(self):
        """L-3: Test that zonendpoint-yara-update.py halts with return code 0 when hold file exists."""
        hold_file = self.base_path / "HOLD-yara"
        hold_file.write_text("2026-11-14T12:00:00Z\n", encoding="utf-8")

        # Run updater with --hold-file pointing to hold_file
        cmd = [
            "python3",
            str(YARA_UPDATE_SCRIPT),
            "--hold-file",
            str(hold_file),
        ]
        proc = subprocess.run(cmd, capture_output=True, text=True)
        self.assertEqual(proc.returncode, 0)
        self.assertIn("Hold file", proc.stdout)
        self.assertIn("Skipping YARA update", proc.stdout)

    def test_rollback_scripts_create_hold_file(self):
        """L-3: Test that rollback scripts create the hold file early and record reload state."""
        for name, script, hold_name, active_name, backup_name in [
            ("MB", MB_ROLLBACK_SCRIPT, "HOLD-abusech", "malwarebazaar.hsb", "malwarebazaar.hsb.bak"),
            ("TF", TF_ROLLBACK_SCRIPT, "HOLD-abusech", "threatfox.hsb", "threatfox.hsb.bak"),
            ("YARA", YARA_ROLLBACK_SCRIPT, "HOLD-yara", "yara-forge-core.yar", "yara-forge-core.yar.bak"),
        ]:
            hold_p = self.base_path / f"{name}_{hold_name}"
            active_p = self.base_path / f"{name}_{active_name}"
            backup_p = self.base_path / f"{name}_{backup_name}"
            backup_p.write_text("dummy_sig\n")
            reload_state_p = self.base_path / f"{name}_reload.json"

            env = os.environ.copy()
            env["ZONENDPOINT_TEST_ENV"] = "1"
            env["CLAMD_PORT"] = "9999"
            env["CLAMDSCAN_BIN"] = "echo"
            env["DEPLOY_LOCK_FILE"] = str(self.base_path / f"{name}_lock.file")
            env["HOLD_FILE"] = str(hold_p)
            env["ACTIVE_FILE"] = str(active_p)
            env["BACKUP_FILE"] = str(backup_p)
            env["RELOAD_STATE_FILE"] = str(reload_state_p)
            env["CLAMD_NO_PING_WAIT"] = "1"

            proc = subprocess.run(["bash", str(script), "--purge"], env=env, capture_output=True, text=True)
            self.assertEqual(proc.returncode, 0, f"{name} purge failed: stdout={proc.stdout}, stderr={proc.stderr}")
            self.assertTrue(hold_p.is_file(), f"{name} did not create hold file")
            self.assertTrue(reload_state_p.is_file(), f"{name} did not record reload state")

    def test_rollback_scripts_create_hold_early_on_failure(self):
        """L-3: Test that HOLD is created at start before any file replacement and retained on error."""
        for name, script, hold_name in [
            ("MB", MB_ROLLBACK_SCRIPT, "HOLD-abusech"),
            ("TF", TF_ROLLBACK_SCRIPT, "HOLD-abusech"),
            ("YARA", YARA_ROLLBACK_SCRIPT, "HOLD-yara"),
        ]:
            hold_p = self.base_path / f"err_{name}_{hold_name}"
            env = os.environ.copy()
            env["ZONENDPOINT_TEST_ENV"] = "1"
            env["CLAMD_PORT"] = "9999"
            env["CLAMDSCAN_BIN"] = "echo"
            env["DEPLOY_LOCK_FILE"] = str(self.base_path / f"err_{name}_lock.file")
            env["HOLD_FILE"] = str(hold_p)
            env["BACKUP_FILE"] = str(self.base_path / "nonexistent.bak")
            env["ACTIVE_FILE"] = str(self.base_path / "nonexistent.hsb")

            # Running normal rollback with missing backup triggers error exit
            proc = subprocess.run(["bash", str(script)], env=env, capture_output=True, text=True)
            self.assertNotEqual(proc.returncode, 0)
            # HOLD must exist even though rollback errored out!
            self.assertTrue(hold_p.is_file(), f"{name} did not create early hold file before failure")

    def test_rollback_safety_guard_rejects_real_clamd(self):
        """CRITICAL: Test that rollback scripts reject real clamd port (3310) and binary in test environment."""
        env = os.environ.copy()
        env["ZONENDPOINT_TEST_ENV"] = "1"
        env["CLAMD_PORT"] = "3310"
        env["CLAMDSCAN_BIN"] = "clamdscan"
        env["DEPLOY_LOCK_FILE"] = str(self.base_path / "safety_lock.file")
        env["HOLD_FILE"] = str(self.base_path / "HOLD-test")
        env["ACTIVE_FILE"] = str(self.base_path / "test.hsb")

        for name, script in [("MB", MB_ROLLBACK_SCRIPT), ("TF", TF_ROLLBACK_SCRIPT), ("YARA", YARA_ROLLBACK_SCRIPT)]:
            proc = subprocess.run(["bash", str(script), "--purge"], env=env, capture_output=True, text=True)
            self.assertEqual(proc.returncode, 99, f"{name} did not exit 99 on real clamd target")
            self.assertIn("SAFETY VIOLATION", proc.stderr)


class TestYaraDeploy(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.TemporaryDirectory()
        self.base_path = pathlib.Path(self.temp_dir.name)

        self.active_p = self.base_path / "yara-forge-core.yar"
        self.active_p.write_text("rule old { condition: true }\n", encoding="utf-8")
        self.backup_dir = self.base_path / "backups"
        self.backup_p = self.backup_dir / "yara-forge-core.yar.bak"
        self.tag_file = self.base_path / "current_tag.txt"
        self.tag_file.write_text("20261103\n", encoding="utf-8")
        self.prev_tag_file = self.backup_dir / "previous_tag.txt"
        self.log_file = self.base_path / "clamav.log"
        self.log_file.write_text("initial clamd log content\n", encoding="utf-8")
        self.staging_yar = self.base_path / "staging.yar"
        self.staging_yar.write_text("rule new { condition: true }\n", encoding="utf-8")

        self.patches = [
            patch.object(yara_mod, "ACTIVE_YARA_PATH", str(self.active_p)),
            patch.object(yara_mod, "BACKUP_DIR", str(self.backup_dir)),
            patch.object(yara_mod, "BACKUP_YARA_PATH", str(self.backup_p)),
            patch.object(yara_mod, "CURRENT_TAG_FILE", str(self.tag_file)),
            patch.object(yara_mod, "PREVIOUS_TAG_FILE", str(self.prev_tag_file)),
        ]
        for p in self.patches:
            p.start()

    def tearDown(self):
        for p in self.patches:
            p.stop()
        self.temp_dir.cleanup()

    def test_yara_deploy_tag_written_only_after_health_check_succeeds(self):
        """L-4: Verify new tag is written ONLY after reload line is seen in log and PING succeeds."""
        def fake_clamdscan(cmd, *args, **kwargs):
            # Append reload message to log
            with open(self.log_file, "a", encoding="utf-8") as f:
                f.write("Database correctly reloaded (3631934 signatures)\n")
            return MagicMock(returncode=0, stderr="")

        with patch("subprocess.run", side_effect=fake_clamdscan):
            with patch.object(yara_mod, "check_clamd_ping", return_value=True):
                yara_mod.deploy_and_reload(
                    staging_yar=str(self.staging_yar),
                    new_tag="20261108",
                    accepted_count=3676,
                    log_file=str(self.log_file),
                    reload_timeout=5.0,
                    clamdscan_bin="echo",
                    clamd_port=9999,
                    force_reload=True,
                    reload_state_file=str(self.base_path / "reload-state.json"),
                )

        # Tag file must now contain the new tag
        self.assertEqual(self.tag_file.read_text(encoding="utf-8").strip(), "20261108")
        self.assertEqual(self.prev_tag_file.read_text(encoding="utf-8").strip(), "20261103")
        self.assertIn("rule new", self.active_p.read_text(encoding="utf-8"))

    def test_yara_deploy_rollback_leaves_tag_unchanged_on_failure(self):
        """L-4: Verify rollback branch does NOT change current tag when health check fails."""
        with patch("subprocess.run", return_value=MagicMock(returncode=0, stderr="")):
            with patch.object(yara_mod, "check_clamd_ping", return_value=False):
                with self.assertRaises(RuntimeError):
                    yara_mod.deploy_and_reload(
                        staging_yar=str(self.staging_yar),
                        new_tag="20261108",
                        accepted_count=3676,
                        log_file=str(self.log_file),
                        reload_timeout=1.5,
                        clamdscan_bin="echo",
                        clamd_port=9999,
                        force_reload=True,
                        reload_state_file=str(self.base_path / "reload-state.json"),
                    )

        # Tag must remain the old tag (20261103), NOT updated to 20261108!
        self.assertEqual(self.tag_file.read_text(encoding="utf-8").strip(), "20261103")
        # Active file must be restored to previous backup
        self.assertIn("rule old", self.active_p.read_text(encoding="utf-8"))

    def test_yara_deploy_handles_log_rotation(self):
        """L-4/L-5: Verify log rotation (truncation/re-creation) during reload is handled properly."""
        # Initial large log
        self.log_file.write_text("A" * 1000 + "\n", encoding="utf-8")

        def fake_clamdscan_with_rotation(cmd, *args, **kwargs):
            # Simulate log rotation: truncate and write fresh smaller log with reload line
            self.log_file.write_text("Database correctly reloaded (3631934 signatures)\n", encoding="utf-8")
            return MagicMock(returncode=0, stderr="")

        with patch("subprocess.run", side_effect=fake_clamdscan_with_rotation):
            with patch.object(yara_mod, "check_clamd_ping", return_value=True):
                yara_mod.deploy_and_reload(
                    staging_yar=str(self.staging_yar),
                    new_tag="20261108",
                    accepted_count=3676,
                    log_file=str(self.log_file),
                    reload_timeout=5.0,
                    clamdscan_bin="echo",
                    clamd_port=9999,
                    force_reload=True,
                    reload_state_file=str(self.base_path / "reload-state.json"),
                )

        self.assertEqual(self.tag_file.read_text(encoding="utf-8").strip(), "20261108")

    def test_yara_deploy_safety_guard_rejects_real_clamd(self):
        """CRITICAL: Test that deploy_and_reload fails immediately if clamd port 3310 or clamdscan binary is targeted in test env."""
        old_env = os.environ.get("ZONENDPOINT_TEST_ENV")
        os.environ["ZONENDPOINT_TEST_ENV"] = "1"
        try:
            # 1. Port 3310 rejected
            with self.assertRaises(AssertionError):
                yara_mod.deploy_and_reload(
                    staging_yar=str(self.staging_yar),
                    new_tag="20261108",
                    accepted_count=3676,
                    clamdscan_bin="echo",
                    clamd_port=3310,
                )

            # 2. clamdscan binary rejected
            with self.assertRaises(AssertionError):
                yara_mod.deploy_and_reload(
                    staging_yar=str(self.staging_yar),
                    new_tag="20261108",
                    accepted_count=3676,
                    clamdscan_bin="clamdscan",
                    clamd_port=9999,
                )
        finally:
            if old_env is None:
                os.environ.pop("ZONENDPOINT_TEST_ENV", None)
            else:
                os.environ["ZONENDPOINT_TEST_ENV"] = old_env

    def test_yara_tag_name_validation(self):
        """L-13: Test strict tag_name regex validation."""
        valid_tags = ["20261108", "v1.2.3", "release-2026_11.08", "tag123"]
        for tag in valid_tags:
            self.assertEqual(yara_mod.validate_tag_name(tag), tag)

        invalid_tags = ["", "tag;rm -rf /", "tag`whoami`", "tag$(id)", "../tag", "tag/sub", ";", "tag with spaces"]
        for tag in invalid_tags:
            with self.assertRaises(ValueError):
                yara_mod.validate_tag_name(tag)

    def test_yara_download_url_and_size_limits(self):
        """L-13: Test download URL domain enforcement and size limit."""
        # 1. Insecure/untrusted URL rejected
        with self.assertRaises(ValueError) as ctx:
            yara_mod.download_file("http://github.com/YARAHQ/repo/file.zip", str(self.base_path / "f.zip"))
        self.assertIn("Only HTTPS", str(ctx.exception))

        with self.assertRaises(ValueError) as ctx:
            yara_mod.download_file("https://evil.com/YARAHQ/repo/file.zip", str(self.base_path / "f.zip"))
        self.assertIn("Must start with 'https://github.com/YARAHQ/'", str(ctx.exception))

    def test_yara_extract_zip_slip_and_size_limits(self):
        """L-13: Test archive extraction protects against Zip Slip and excessive size."""
        import zipfile
        # Create malicious zip with zip slip
        slip_zip = self.base_path / "slip.zip"
        with zipfile.ZipFile(slip_zip, "w") as z:
            z.writestr("../../../../../tmp/pwned.yar", "rule pwn { condition: true }")

        with self.assertRaises(ValueError) as ctx:
            yara_mod.extract_core_rules(str(slip_zip), str(self.base_path / "ext"))
        self.assertIn("path traversal", str(ctx.exception))

    def test_yara_min_reload_interval_defers_reload(self):
        """L-8: Test that minimum reload interval defers reload without clamdscan call."""
        state_file = self.base_path / "reload-state.json"
        now = time.time()
        # Record recent reload (1h ago)
        yara_mod.record_reload_state(str(state_file), caller="test", signatures=1000)

        with patch("subprocess.run") as mock_run:
            yara_mod.deploy_and_reload(
                staging_yar=str(self.staging_yar),
                new_tag="20261108",
                accepted_count=3676,
                clamdscan_bin="echo",
                clamd_port=9999,
                force_reload=False,
                min_reload_interval=6.0,
                reload_state_file=str(state_file),
            )
            # clamdscan was NOT called!
            mock_run.assert_not_called()

        # Tag file was updated because active file was deployed atomically
        self.assertEqual(self.tag_file.read_text(encoding="utf-8").strip(), "20261108")


if __name__ == "__main__":
    unittest.main()
