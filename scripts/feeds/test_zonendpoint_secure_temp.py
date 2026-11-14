#!/usr/bin/env python3
"""
Unit tests for secure temporary file handling, fchmod, fsync, and atomic replace (Task L-11).
"""
import os
import pathlib
import stat
import tempfile
import unittest
import importlib.util
from unittest.mock import patch

HERE = pathlib.Path(__file__).resolve().parent
MB_SCRIPT = HERE / "zonendpoint-malwarebazaar.py"
YARA_SCRIPT = HERE / "zonendpoint-yara-update.py"

_mb_spec = importlib.util.spec_from_file_location("mb_mod", MB_SCRIPT)
mb_mod = importlib.util.module_from_spec(_mb_spec)
_mb_spec.loader.exec_module(mb_mod)

_yara_spec = importlib.util.spec_from_file_location("yara_mod", YARA_SCRIPT)
yara_mod = importlib.util.module_from_spec(_yara_spec)
_yara_spec.loader.exec_module(yara_mod)


class TestSecureTempFiles(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.TemporaryDirectory()
        self.base_path = pathlib.Path(self.temp_dir.name)

    def tearDown(self):
        self.temp_dir.cleanup()

    def test_mb_safe_atomic_write_content(self):
        """L-11: Verify mb.safe_atomic_write creates file atomically with 0644 mode and fsync."""
        target = self.base_path / "signatures.hsb"
        test_data = "d41d8cd98f00b204e9800998ecf8427e:0:TestSig\n"

        with patch("os.fsync") as mock_fsync:
            res_p = mb_mod.safe_atomic_write(target, content=test_data, mode=0o644)
            self.assertEqual(res_p, target)
            self.assertTrue(target.is_file())
            self.assertEqual(target.read_text(encoding="utf-8"), test_data)
            mock_fsync.assert_called()

        mode = stat.S_IMODE(target.stat().st_mode)
        self.assertEqual(mode, 0o644)

    def test_mb_safe_atomic_write_source_file(self):
        """L-11: Verify mb.safe_atomic_write copies source file atomically with fsync."""
        src = self.base_path / "backup.hsb.bak"
        src.write_text("source_backup_content\n", encoding="utf-8")
        target = self.base_path / "restored.hsb"

        with patch("os.fsync") as mock_fsync:
            res_p = mb_mod.safe_atomic_write(target, source_file=src, mode=0o644)
            self.assertEqual(res_p, target)
            self.assertTrue(target.is_file())
            self.assertEqual(target.read_text(encoding="utf-8"), "source_backup_content\n")
            mock_fsync.assert_called()

    def test_yara_safe_atomic_write(self):
        """L-11: Verify yara safe_atomic_write copies source file atomically with fsync and 0644."""
        src = self.base_path / "rules.yar"
        src.write_text("rule test { condition: true }\n", encoding="utf-8")
        target = self.base_path / "active.yar"

        with patch("os.fsync") as mock_fsync:
            yara_mod.safe_atomic_write(str(target), source_file=str(src), mode=0o644)
            self.assertTrue(target.is_file())
            self.assertEqual(target.read_text(encoding="utf-8"), "rule test { condition: true }\n")
            mock_fsync.assert_called()

        mode = stat.S_IMODE(target.stat().st_mode)
        self.assertEqual(mode, 0o644)

    def test_no_predictable_dot_tmp_left_behind(self):
        """L-11: Verify that predictable .tmp files are not created or left in target dir."""
        target = self.base_path / "active.yar"
        src = self.base_path / "source.yar"
        src.write_text("content\n", encoding="utf-8")

        yara_mod.safe_atomic_write(str(target), source_file=str(src))
        predictable_tmp = self.base_path / "active.yar.tmp"
        self.assertFalse(predictable_tmp.exists())


if __name__ == "__main__":
    unittest.main()
