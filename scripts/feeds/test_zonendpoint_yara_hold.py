#!/usr/bin/env python3
"""
Unit tests for YARA updater and rollback script hold file functionality (Task L-3).
"""
import os
import pathlib
import subprocess
import tempfile
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
        """L-3: Test that rollback scripts create the hold file when executed."""
        # 1. MalwareBazaar rollback
        mb_hold = self.base_path / "HOLD-abusech"
        mb_active = self.base_path / "malwarebazaar.hsb"
        mb_backup = self.base_path / "malwarebazaar.hsb.bak"
        mb_backup.write_text("dummy_sig\n")

        env = os.environ.copy()
        env["HOLD_FILE"] = str(mb_hold)
        env["ACTIVE_FILE"] = str(mb_active)
        env["BACKUP_FILE"] = str(mb_backup)

        # Mock reload_clamd by sourcing or testing purge branch with mock clamd
        # We can run bash with a mocked reload_clamd function:
        test_script = f"""
        reload_clamd() {{ echo "mocked clamd reload"; }}
        source "{MB_ROLLBACK_SCRIPT}" --purge
        """
        proc = subprocess.run(["bash", "-c", test_script], env=env, capture_output=True, text=True)
        self.assertEqual(proc.returncode, 0, f"stdout: {proc.stdout}, stderr: {proc.stderr}")
        self.assertTrue(mb_hold.is_file())

        # 2. YARA rollback
        yara_hold = self.base_path / "HOLD-yara"
        yara_active = self.base_path / "yara-forge-core.yar"
        env["HOLD_FILE"] = str(yara_hold)
        env["ACTIVE_FILE"] = str(yara_active)
        test_script_yara = f"""
        clamdscan() {{ echo "mocked reload"; }}
        nc() {{ echo "PONG"; }}
        source "{YARA_ROLLBACK_SCRIPT}" --purge
        """
        proc_yara = subprocess.run(["bash", "-c", test_script_yara], env=env, capture_output=True, text=True)
        self.assertEqual(proc_yara.returncode, 0)
        self.assertTrue(yara_hold.is_file())


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
                )

        self.assertEqual(self.tag_file.read_text(encoding="utf-8").strip(), "20261108")


if __name__ == "__main__":
    unittest.main()
