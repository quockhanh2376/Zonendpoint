#!/usr/bin/env python3
"""
Unit tests for shared deploy flock between abusech-update, yara-update, and rollback scripts (Task L-6).
"""
import os
import pathlib
import subprocess
import tempfile
import unittest
import fcntl

HERE = pathlib.Path(__file__).resolve().parent
YARA_UPDATE_SCRIPT = HERE / "zonendpoint-yara-update.py"
ABUSECH_UPDATE_SCRIPT = HERE / "zonendpoint-abusech-update.py"
MB_ROLLBACK_SCRIPT = HERE / "zonendpoint-malwarebazaar-rollback.sh"
TF_ROLLBACK_SCRIPT = HERE / "zonendpoint-threatfox-rollback.sh"
YARA_ROLLBACK_SCRIPT = HERE / "zonendpoint-yara-rollback.sh"


class TestSharedDeployLock(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.TemporaryDirectory()
        self.base_path = pathlib.Path(self.temp_dir.name)
        self.lock_file = self.base_path / "deploy.lock"

    def tearDown(self):
        self.temp_dir.cleanup()

    def test_yara_update_aborts_non_zero_when_lock_held(self):
        """L-6: Test that zonendpoint-yara-update exits non-zero if shared deploy lock is held."""
        with open(str(self.lock_file), "w") as fd:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            cmd = [
                "python3",
                str(YARA_UPDATE_SCRIPT),
                "--lock-file",
                str(self.lock_file),
                "--check-only",
            ]
            proc = subprocess.run(cmd, capture_output=True, text=True)
            self.assertNotEqual(proc.returncode, 0)
            self.assertIn("Another clamd deploy/update process is currently running", proc.stdout + proc.stderr)
            fcntl.flock(fd, fcntl.LOCK_UN)

    def test_yara_update_aborts_non_zero_when_lock_uncreatable(self):
        """L-6: Test that zonendpoint-yara-update exits non-zero if lock file cannot be created."""
        cmd = [
            "python3",
            str(YARA_UPDATE_SCRIPT),
            "--lock-file",
            "/proc/zonendpoint_nonexistent/lock.file",
            "--check-only",
        ]
        proc = subprocess.run(cmd, capture_output=True, text=True)
        self.assertNotEqual(proc.returncode, 0)

    def test_mb_rollback_aborts_non_zero_when_lock_held(self):
        """L-6: Test that zonendpoint-malwarebazaar-rollback.sh exits non-zero if shared deploy lock is held."""
        with open(str(self.lock_file), "w") as fd:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            env = os.environ.copy()
            env["DEPLOY_LOCK_FILE"] = str(self.lock_file)
            env["BACKUP_FILE"] = str(self.base_path / "dummy.bak")
            env["ACTIVE_FILE"] = str(self.base_path / "active.hsb")

            proc = subprocess.run(["bash", str(MB_ROLLBACK_SCRIPT)], env=env, capture_output=True, text=True)
            self.assertNotEqual(proc.returncode, 0)
            self.assertIn("Another clamd deploy/update process is currently running", proc.stdout + proc.stderr)
            fcntl.flock(fd, fcntl.LOCK_UN)

    def test_tf_rollback_aborts_non_zero_when_lock_held(self):
        """L-6: Test that zonendpoint-threatfox-rollback.sh exits non-zero if shared deploy lock is held."""
        with open(str(self.lock_file), "w") as fd:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            env = os.environ.copy()
            env["DEPLOY_LOCK_FILE"] = str(self.lock_file)
            env["BACKUP_FILE"] = str(self.base_path / "dummy.bak")
            env["ACTIVE_FILE"] = str(self.base_path / "active.hsb")

            proc = subprocess.run(["bash", str(TF_ROLLBACK_SCRIPT)], env=env, capture_output=True, text=True)
            self.assertNotEqual(proc.returncode, 0)
            self.assertIn("Another clamd deploy/update process is currently running", proc.stdout + proc.stderr)
            fcntl.flock(fd, fcntl.LOCK_UN)

    def test_yara_rollback_aborts_non_zero_when_lock_held(self):
        """L-6: Test that zonendpoint-yara-rollback.sh exits non-zero if shared deploy lock is held."""
        with open(str(self.lock_file), "w") as fd:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            env = os.environ.copy()
            env["DEPLOY_LOCK_FILE"] = str(self.lock_file)
            env["BACKUP_FILE"] = str(self.base_path / "dummy.bak")
            env["ACTIVE_FILE"] = str(self.base_path / "active.yar")

            proc = subprocess.run(["bash", str(YARA_ROLLBACK_SCRIPT)], env=env, capture_output=True, text=True)
            self.assertNotEqual(proc.returncode, 0)
            self.assertIn("Another clamd deploy/update process is currently running", proc.stdout + proc.stderr)
            fcntl.flock(fd, fcntl.LOCK_UN)


if __name__ == "__main__":
    unittest.main()
