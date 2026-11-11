#!/usr/bin/env python3
"""
Unit and mock tests for Zonendpoint Hourly abuse.ch Update (Task V-42)

Verifies:
1. File locking prevents overlapping concurrent runs.
2. Rate limit exhaustion skips turn without retry.
3. Order of execution is strictly MalwareBazaar -> ThreatFox.
4. If new hashes present: exports staged .hsb, verifies temp DB, backs up, reloads clamd once.
5. If NO new hashes present: skips clamd reload entirely.
6. Deployment error triggers automatic rollback to previous backup.
7. last-run.json NEVER contains Auth-Key under any circumstance.
8. Upstream network/HTTP error skips turn cleanly without crashing or retry.
"""
import http.server
import importlib.util
import json
import os
import pathlib
import shutil
import socketserver
import tempfile
import threading
import time
import unittest
from unittest.mock import patch, MagicMock

HERE = pathlib.Path(__file__).resolve().parent

# Load modules
_update_spec = importlib.util.spec_from_file_location("zonendpoint_abusech_update", HERE / "zonendpoint-abusech-update.py")
update_mod = importlib.util.module_from_spec(_update_spec)
_update_spec.loader.exec_module(update_mod)

from abusech_rate_limit import RateLimiter, RateLimitError


class MockHttpHandler(http.server.BaseHTTPRequestHandler):
    mode = "normal"
    request_log = []
    mb_csv_content = b'# "first_seen_utc","sha256_hash","md5_hash","sha1_hash","reporter","file_name","file_type_guess","mime_type","signature","clamav"\n'
    tf_csv_content = b'# "first_seen_utc","ioc_id","ioc_value","ioc_type","threat_type","fk_malware","malware_alias","malware_printable","last_seen_utc","confidence_level","is_compromised","reference","tags","anonymous","reporter"\n'

    def do_GET(self):
        MockHttpHandler.request_log.append({"path": self.path, "headers": dict(self.headers)})
        if MockHttpHandler.mode == "429":
            self.send_response(429)
            self.send_header("Content-Type", "text/plain")
            self.end_headers()
            self.wfile.write(b"Too Many Requests: key=" + self.path.encode())
            return

        if MockHttpHandler.mode == "503":
            self.send_response(503)
            self.send_header("Content-Type", "text/plain")
            self.end_headers()
            self.wfile.write(b"Service Unavailable")
            return

        self.send_response(200)
        self.send_header("Content-Type", "text/csv")
        self.end_headers()
        if "threatfox" in self.path or "tf" in self.path:
            self.wfile.write(MockHttpHandler.tf_csv_content)
        else:
            self.wfile.write(MockHttpHandler.mb_csv_content)

    def log_message(self, format, *args):
        pass


class TestAbusechUpdate(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.server = socketserver.TCPServer(("127.0.0.1", 0), MockHttpHandler)
        cls.server_port = cls.server.server_address[1]
        cls.server_thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.server_thread.start()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()

    def setUp(self):
        MockHttpHandler.mode = "normal"
        MockHttpHandler.request_log = []
        MockHttpHandler.mb_csv_content = (
            b'# "first_seen_utc","sha256_hash","md5_hash","sha1_hash","reporter","file_name","file_type_guess","mime_type","signature","clamav"\n'
            b'"2026-11-11 08:00:00","1111111111111111111111111111111111111111111111111111111111111111","","","test","sample1.exe","exe","application/x-dosexec","Trojan.MB.Test",""\n'
        )
        MockHttpHandler.tf_csv_content = (
            b'# "first_seen_utc","ioc_id","ioc_value","ioc_type","threat_type","fk_malware","malware_alias","malware_printable","last_seen_utc","confidence_level","is_compromised","reference","tags","anonymous","reporter"\n'
            b'"2026-11-11 08:00:00","1001","2222222222222222222222222222222222222222222222222222222222222222","sha256_hash","botnet_cc","mal_1","alias","Trojan.TF.Test","2026-11-11 08:30:00","100","0","","","0","test"\n'
        )
        self.temp_dir = tempfile.TemporaryDirectory()
        self.base_path = pathlib.Path(self.temp_dir.name)

        self.key_file = self.base_path / "abusech.key"
        self.secret_key = "SECRET_ABUSECH_AUTH_KEY_XYZ999"
        self.key_file.write_text(self.secret_key + "\n", encoding="utf-8")
        try:
            os.chmod(str(self.key_file), 0o600)
        except OSError:
            pass

        self.rate_limit_file = self.base_path / "rate_limit.json"
        self.lock_file = self.base_path / "update.lock"
        self.last_run_file = self.base_path / "last-run.json"

        self.mb_db = self.base_path / "malwarebazaar.db"
        self.tf_db = self.base_path / "threatfox.db"

        self.mb_hsb = self.base_path / "malwarebazaar.hsb"
        self.tf_hsb = self.base_path / "threatfox.hsb"
        self.mb_backup_dir = self.base_path / "backups_mb"
        self.tf_backup_dir = self.base_path / "backups_tf"
        self.clamav_db_dir = self.base_path / "clamav_db"
        self.clamav_db_dir.mkdir(parents=True, exist_ok=True)
        self.clamd_log_file = self.base_path / "clamav.log"
        self.clamd_log_file.write_text("Database correctly reloaded (1000 signatures)\n", encoding="utf-8")

        self.args = MagicMock()
        self.args.key_file = str(self.key_file)
        self.args.mb_db = str(self.mb_db)
        self.args.tf_db = str(self.tf_db)
        self.args.rate_limit_file = str(self.rate_limit_file)
        self.args.lock_file = str(self.lock_file)
        self.args.last_run_file = str(self.last_run_file)
        self.args.mb_url = f"http://127.0.0.1:{self.server_port}/mb/{{auth_key}}/recent.csv"
        self.args.tf_url = f"http://127.0.0.1:{self.server_port}/tf/{{auth_key}}/recent.csv"
        self.args.tf_recent_dump = str(self.base_path / "last-recent.csv")
        self.args.mb_hsb_path = str(self.mb_hsb)
        self.args.tf_hsb_path = str(self.tf_hsb)
        self.args.mb_backup_dir = str(self.mb_backup_dir)
        self.args.tf_backup_dir = str(self.tf_backup_dir)
        self.args.clamav_db_dir = str(self.clamav_db_dir)
        self.args.clamd_host = "127.0.0.1"
        self.args.clamd_port = 3310
        self.args.log_file = str(self.clamd_log_file)
        self.args.clamscan_bin = "clamscan"
        self.args.timeout = 10.0
        self.args.reload_timeout = 5.0
        self.args.min_ram_bytes = 1024 * 1024

    def tearDown(self):
        self.temp_dir.cleanup()

    def test_01_concurrent_run_blocked_by_lock(self):
        """Test that another running process holding lock file halts new run immediately."""
        lock_fd = open(str(self.lock_file), "a+")
        try:
            import fcntl
            fcntl.flock(lock_fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except Exception:
            pass

        # Second process attempts to run
        ret = update_mod.run_hourly_update(self.args)
        self.assertEqual(ret, 0)
        # No requests should have been made
        self.assertEqual(len(MockHttpHandler.request_log), 0)
        lock_fd.close()

    def test_02_rate_limit_exhausted_skips_turn_without_retry(self):
        """Test that if 3 requests used in sliding hour, turn is skipped without retry."""
        now = time.time()
        # Seed 3 requests in last 10 minutes
        self.rate_limit_file.write_text(json.dumps({
            "requests": [
                {"timestamp": now - 300, "source": "malwarebazaar"},
                {"timestamp": now - 200, "source": "threatfox"},
                {"timestamp": now - 100, "source": "malwarebazaar"},
            ],
            "updated_at": "2026-11-11T08:00:00Z"
        }), encoding="utf-8")

        ret = update_mod.run_hourly_update(self.args)
        self.assertEqual(ret, 0)
        self.assertEqual(len(MockHttpHandler.request_log), 0)

        # Check last-run.json
        self.assertTrue(self.last_run_file.is_file())
        last_run = json.loads(self.last_run_file.read_text(encoding="utf-8"))
        self.assertIn("Shared hourly rate limit reached", str(last_run.get("error")))
        self.assertFalse(last_run["reloaded"])

    def test_03_execution_order_mb_first_then_tf(self):
        """Test that MalwareBazaar is queried before ThreatFox."""
        with patch.object(update_mod, "deploy_combined_hsbs", return_value={"reloaded": True, "sig_before": 1000, "sig_after": 1002, "reload_duration": 1.0}):
            ret = update_mod.run_hourly_update(self.args)
            self.assertEqual(ret, 0)
            self.assertEqual(len(MockHttpHandler.request_log), 2)
            self.assertIn("/mb/", MockHttpHandler.request_log[0]["path"])
            self.assertIn("/tf/", MockHttpHandler.request_log[1]["path"])

    def test_04_new_hashes_triggers_export_and_single_reload(self):
        """Test that presence of new hashes exports .hsb, verifies, and reloads clamd once."""
        fake_deploy = MagicMock(return_value={
            "reloaded": True,
            "sig_before": 1000,
            "sig_after": 1002,
            "reload_duration": 0.5,
        })
        with patch.object(update_mod, "deploy_combined_hsbs", fake_deploy):
            ret = update_mod.run_hourly_update(self.args)
            self.assertEqual(ret, 0)
            fake_deploy.assert_called_once()
            deploy_items = fake_deploy.call_args[1]["deploy_items"]
            self.assertIn("malwarebazaar", deploy_items)
            self.assertIn("threatfox", deploy_items)

            # Check last-run record
            last_run = json.loads(self.last_run_file.read_text(encoding="utf-8"))
            self.assertTrue(last_run["reloaded"])
            self.assertEqual(last_run["new_hashes"]["total"], 2)
            self.assertEqual(last_run["requests"]["total"], 2)
            self.assertIsNone(last_run["error"])

    def test_05_no_new_hashes_skips_clamd_reload(self):
        """Test that if 0 new hashes exist, clamd is NOT reloaded."""
        # Pre-populate databases and active .hsb files with same hashes
        db_mb = update_mod.mb.MalwareDatabase(str(self.mb_db))
        db_mb.upsert_hashes([{"sha256": "1111111111111111111111111111111111111111111111111111111111111111", "signature": "Test"}])
        self.mb_hsb.write_text("1111111111111111111111111111111111111111111111111111111111111111:*:MalwareBazaar.Test.UNOFFICIAL\n", encoding="utf-8")

        db_tf = update_mod.tf.ThreatDatabase(str(self.tf_db))
        db_tf.import_rows([{
            "ioc": "2222222222222222222222222222222222222222222222222222222222222222",
            "ioc_type": "sha256",
            "malware": "Test",
        }])
        self.tf_hsb.write_text("2222222222222222222222222222222222222222222222222222222222222222:*:ThreatFox.Test.UNOFFICIAL:73\n", encoding="utf-8")

        fake_deploy = MagicMock()
        with patch.object(update_mod, "deploy_combined_hsbs", fake_deploy):
            ret = update_mod.run_hourly_update(self.args)
            self.assertEqual(ret, 0)
            fake_deploy.assert_not_called()

            last_run = json.loads(self.last_run_file.read_text(encoding="utf-8"))
            self.assertFalse(last_run["reloaded"])
            self.assertEqual(last_run["new_hashes"]["total"], 0)

    def test_06_deployment_error_triggers_rollback(self):
        """Test that reload timeout or failure restores backups and clamd state."""
        # Initial active files
        self.mb_hsb.write_text("old_mb_sig\n", encoding="utf-8")
        self.tf_hsb.write_text("old_tf_sig\n", encoding="utf-8")

        # Mock preflight pass but reload fails
        with patch.object(update_mod, "verify_hsbs_on_temp_db", return_value={"status": "PASS"}), \
             patch.object(update_mod.mb, "get_available_memory_bytes", return_value=8 * 1024**3), \
             patch.object(update_mod.mb, "trigger_clamd_zreload", return_value=True), \
             patch.object(update_mod.mb, "check_clamd_ping", return_value=False):  # PING fails

            with self.assertRaises(RuntimeError):
                update_mod.deploy_combined_hsbs(
                    deploy_items={
                        "malwarebazaar": {
                            "active_path": self.mb_hsb,
                            "backup_dir": self.mb_backup_dir,
                            "staged_content": "new_mb_sig\n",
                            "new_signatures_delta": 1,
                        },
                        "threatfox": {
                            "active_path": self.tf_hsb,
                            "backup_dir": self.tf_backup_dir,
                            "staged_content": "new_tf_sig\n",
                            "new_signatures_delta": 1,
                        },
                    },
                    clamav_db_dir=str(self.clamav_db_dir),
                    clamd_host="127.0.0.1",
                    clamd_port=3310,
                    log_file=str(self.clamd_log_file),
                    reload_timeout=0.1,
                )

            # Active files must have rolled back to previous content
            self.assertEqual(self.mb_hsb.read_text(encoding="utf-8"), "old_mb_sig\n")
            self.assertEqual(self.tf_hsb.read_text(encoding="utf-8"), "old_tf_sig\n")

    def test_07_last_run_json_never_contains_auth_key(self):
        """Test that secret Auth-Key is masked from last-run.json even on errors echoing the key."""
        MockHttpHandler.mode = "429"  # Echoes key in path
        ret = update_mod.run_hourly_update(self.args)

        self.assertTrue(self.last_run_file.is_file())
        raw_text = self.last_run_file.read_text(encoding="utf-8")
        self.assertNotIn(self.secret_key, raw_text)
        self.assertIn("<AUTH-KEY>", raw_text)

    def test_08_upstream_error_skips_turn_cleanly_no_continuous_retry(self):
        """Test that upstream HTTP 503 skips turn without retry and records error."""
        MockHttpHandler.mode = "503"
        ret = update_mod.run_hourly_update(self.args)
        self.assertEqual(ret, 0)
        # Should attempt each feed exactly once, no retry loops
        self.assertEqual(len(MockHttpHandler.request_log), 2)
        last_run = json.loads(self.last_run_file.read_text(encoding="utf-8"))
        self.assertFalse(last_run["reloaded"])
        self.assertIsNotNone(last_run["error"])


if __name__ == "__main__":
    unittest.main()
