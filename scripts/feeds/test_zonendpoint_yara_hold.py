#!/usr/bin/env python3
"""
Unit tests for YARA updater and rollback script hold file functionality (Task L-3).
"""
import os
import pathlib
import subprocess
import tempfile
import unittest
from unittest.mock import patch, MagicMock

HERE = pathlib.Path(__file__).resolve().parent
YARA_UPDATE_SCRIPT = HERE / "zonendpoint-yara-update.py"
YARA_ROLLBACK_SCRIPT = HERE / "zonendpoint-yara-rollback.sh"
MB_ROLLBACK_SCRIPT = HERE / "zonendpoint-malwarebazaar-rollback.sh"
TF_ROLLBACK_SCRIPT = HERE / "zonendpoint-threatfox-rollback.sh"


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


if __name__ == "__main__":
    unittest.main()
