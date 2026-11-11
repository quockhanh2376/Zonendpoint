import csv
import importlib.util
import io
import json
import os
import pathlib
import tempfile
import unittest
from unittest import mock
import urllib.error
import zipfile

HERE = pathlib.Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("threatfox", HERE / "zonendpoint-threatfox.py")
tf = importlib.util.module_from_spec(spec)
spec.loader.exec_module(tf)


class ThreatFoxTests(unittest.TestCase):
    def test_csv_columns_are_resolved_by_header_and_duplicates_dedup(self):
        with tempfile.TemporaryDirectory() as d:
            csv_path = pathlib.Path(d) / "mock.csv"
            csv_path.write_text(
                "malware,last_seen,ioc_type,confidence_level,ioc,first_seen\n"
                "Fam,2026-11-10,sha256,90," + "A" * 64 + ",2026-11-01\n"
                "Fam,2026-11-10,sha256,95," + "a" * 64 + ",2026-11-01\n"
                "Fam,,domain,50,Example.COM,2026-11-01\n",
                encoding="utf-8",
            )
            db = tf.ThreatDatabase(pathlib.Path(d) / "tf.db")
            self.assertEqual(db.import_csv(csv_path), 3)
            with db.connect() as c:
                self.assertEqual(c.execute("select count(*) from iocs").fetchone()[0], 2)
                self.assertEqual(c.execute("select ioc from iocs where ioc_type='domain'").fetchone()[0], "example.com")

    def test_csv_with_comment_headers_and_preamble(self):
        with tempfile.TemporaryDirectory() as d:
            csv_path = pathlib.Path(d) / "commented.csv"
            csv_path.write_text(
                '# ThreatFox export database\n'
                '# Generated at 2026-11-11\n'
                '# "id","ioc","ioc_type","threat_type","malware","malware_printable","first_seen_utc","last_seen_utc","confidence_level"\n'
                '1,"' + "c" * 64 + '","sha256","botnet","Trojan","Trojan.Clean","2026-11-01","2026-11-10",90\n',
                encoding="utf-8",
            )
            db = tf.ThreatDatabase(pathlib.Path(d) / "tf.db")
            self.assertEqual(db.import_csv(csv_path), 1)
            with db.connect() as c:
                self.assertEqual(c.execute("select malware from iocs where ioc=?", ("c" * 64,)).fetchone()[0], "Trojan.Clean")

    def test_bad_zip_and_bad_csv_do_not_replace_existing_db_rows(self):
        with tempfile.TemporaryDirectory() as d:
            db = tf.ThreatDatabase(pathlib.Path(d) / "tf.db")
            db.import_rows([{"ioc": "a" * 64, "ioc_type": "sha256", "malware": "kept"}])
            broken = pathlib.Path(d) / "bad.csv"
            broken.write_text("\n")
            with self.assertRaises(ValueError):
                db.import_csv(broken)
            with db.connect() as c:
                self.assertEqual(c.execute("select malware from iocs").fetchone()[0], "kept")

    def test_hsb_wildcard_and_malwarebazaar_hash_exclusion(self):
        with tempfile.TemporaryDirectory() as d:
            db = tf.ThreatDatabase(pathlib.Path(d) / "tf.db")
            db.import_rows([{"ioc": "a" * 64, "ioc_type": "sha256", "malware": "Fam"}, {"ioc": "b" * 64, "ioc_type": "sha256", "malware": ""}])
            mb_path = pathlib.Path(d) / "mb.db"
            import sqlite3
            with sqlite3.connect(mb_path) as c:
                c.execute("CREATE TABLE malware_hashes(sha256 TEXT)")
                c.execute("INSERT INTO malware_hashes VALUES(?)", ("a" * 64,))
            old = os.environ.get("MALWAREBAZAAR_DB_PATH")
            os.environ["MALWAREBAZAAR_DB_PATH"] = str(mb_path)
            out = pathlib.Path(d) / "out.hsb"
            try:
                self.assertEqual(db.export_hsb(out), 1)
            finally:
                if old is None:
                    os.environ.pop("MALWAREBAZAAR_DB_PATH", None)
                else:
                    os.environ["MALWAREBAZAAR_DB_PATH"] = old
            self.assertNotIn("ThreatFox.Fam.UNOFFICIAL", out.read_text())
            self.assertIn("ThreatFox.Unknown.UNOFFICIAL:73", out.read_text())

    def test_invalid_zip_and_key_masking(self):
        key = "secret-token"
        self.assertNotIn(key, tf.mask(f"download failed at /exports/{key}/full.csv.zip", key))
        with tempfile.TemporaryDirectory() as d:
            limiter = mock.Mock()
            limiter.check_and_record.return_value = None
            with mock.patch.object(tf.urllib.request, "urlopen", side_effect=OSError(f"failed /exports/{key}/file")):
                with self.assertRaises(RuntimeError) as err:
                    tf.fetch_full("https://example.test/{auth_key}/dump.zip", key, limiter, temp_dir=d)
            self.assertNotIn(key, str(err.exception))

    def test_malformed_rows_are_not_accepted(self):
        with tempfile.TemporaryDirectory() as d:
            db = tf.ThreatDatabase(pathlib.Path(d) / "tf.db")
            with self.assertRaises(ValueError):
                db.import_rows([{"ioc": "not-a-hash", "ioc_type": "sha256"}, {"ioc": "1.2.3.4", "ioc_type": "ip"}])
            with db.connect() as c:
                self.assertEqual(c.execute("select count(*) from iocs").fetchone()[0], 0)

    def test_http_429_is_fail_fast_and_masks_key(self):
        key = "secret-token"
        limiter = mock.Mock()
        error = urllib.error.HTTPError(f"https://x/{key}/full.csv.zip", 429, "Too Many Requests", {}, None)
        with tempfile.TemporaryDirectory() as d:
            with mock.patch.object(tf.urllib.request, "urlopen", side_effect=error):
                with self.assertRaises(RuntimeError) as err:
                    tf.fetch_full("https://x/{auth_key}/full.csv.zip", key, limiter, temp_dir=d)
            self.assertNotIn(key, str(err.exception))
            self.assertEqual(limiter.check_and_record.call_count, 1)

    def test_invalid_zip_is_rejected(self):
        limiter = mock.Mock()
        response = mock.MagicMock()
        response.__enter__.return_value = response
        response.read.side_effect = [b"not a zip", b""]
        with tempfile.TemporaryDirectory() as d:
            with mock.patch.object(tf.urllib.request, "urlopen", return_value=response):
                with self.assertRaises(ValueError):
                    tf.fetch_full("https://x/{auth_key}/x", "k", limiter, temp_dir=d)

    def test_fetch_full_streams_to_temp_file_and_extracts_cleanly(self):
        csv_content = b'# "first_seen_utc", "ioc_id", "ioc_value", "ioc_type", "malware_printable"\n"2026-11-11", "1", "' + b"d" * 64 + b'", "sha256_hash", "TestMalware"\n'
        buf = io.BytesIO()
        with zipfile.ZipFile(buf, "w", compression=zipfile.ZIP_DEFLATED) as z:
            z.writestr("threatfox.csv", csv_content)
        zip_bytes = buf.getvalue()

        chunk_size = 20
        chunks = [zip_bytes[i:i + chunk_size] for i in range(0, len(zip_bytes), chunk_size)]
        chunks.append(b"")

        limiter = mock.Mock()
        response = mock.MagicMock()
        response.__enter__.return_value = response
        response.read.side_effect = chunks

        with tempfile.TemporaryDirectory() as d:
            dump_file = pathlib.Path(d) / "saved_dump.zip"
            with mock.patch.object(tf.urllib.request, "urlopen", return_value=response):
                csv_path = tf.fetch_full("https://x/{auth_key}/full.csv.zip", "key123", limiter, dump_path=dump_file, temp_dir=d)
            try:
                self.assertTrue(pathlib.Path(csv_path).exists())
                self.assertEqual(pathlib.Path(csv_path).read_bytes(), csv_content)
                self.assertTrue(dump_file.exists())
                self.assertEqual(dump_file.read_bytes(), zip_bytes)
            finally:
                if os.path.exists(csv_path):
                    os.unlink(csv_path)

    def test_fetch_full_oversized_download_is_rejected(self):
        limiter = mock.Mock()
        response = mock.MagicMock()
        response.__enter__.return_value = response
        response.read.side_effect = [b"x" * 20] * 10

        with tempfile.TemporaryDirectory() as d:
            with mock.patch.object(tf, "MAX_BYTES", 50):
                with mock.patch.object(tf.urllib.request, "urlopen", return_value=response):
                    with self.assertRaises(ValueError) as ctx:
                        tf.fetch_full("https://x/{auth_key}/full.csv.zip", "key123", limiter, temp_dir=d)
                    self.assertIn("exceeds download size limit", str(ctx.exception))

    def test_deploy_hsb_passes_log_file(self):
        with tempfile.TemporaryDirectory() as d:
            fake_db = pathlib.Path(d) / "tf.db"
            with mock.patch.object(tf.mb, "deploy_hsb") as mock_deploy:
                with mock.patch.object(tf.mb, "load_auth_key", return_value="dummy"):
                    with mock.patch("sys.argv", ["zonendpoint-threatfox.py", "deploy-hsb", "--db", str(fake_db), "--log-file", "/tmp/custom_clamav.log"]):
                        ret = tf.main()
                        self.assertEqual(ret, 0)
                        mock_deploy.assert_called_once()
                        _, kwargs = mock_deploy.call_args
                        self.assertEqual(kwargs.get("log_file"), "/tmp/custom_clamav.log")

    def test_real_abusech_export_fixture_with_preamble_quotes_spaces_and_postamble(self):
        # Synthetic fixture strictly mimicking abuse.ch export format without real indicators
        fixture = (
            '# ThreatFox export (CSV)\n'
            '# https://threatfox.abuse.ch/\n'
            '# Terms of use: non-commercial\n'
            '# "first_seen_utc", "ioc_id", "ioc_value", "ioc_type", "threat_type", "fk_malware", "malware_alias", "malware_printable", "last_seen_utc", "confidence_level", "is_compromised", "reference", "tags", "anonymous", "reporter"\n'
            '# Mid-comment line\n'
            '"2026-11-11 00:00:00", "101", "' + "1" * 64 + '", "sha256_hash", "payload", "win.test1", "Alias1", "Test Family One", "2026-11-11 01:00:00", "100", "0", "None", "trojan, c2", "0", "rep1"\n'
            '"2026-11-11 00:00:00", "102", "' + "2" * 32 + '", "md5_hash", "payload", "win.test2", "Alias2", "", "2026-11-11 01:00:00", "90", "0", "None", "loader", "0", "rep2"\n'
            '"2026-11-11 00:00:00", "103", "' + "3" * 40 + '", "sha1_hash", "payload", "win.test3", "Alias3", "None", "2026-11-11 01:00:00", "85", "0", "None", "stealer", "0", "rep3"\n'
            '"2026-11-11 00:00:00", "104", "192.0.2.100:443", "ip:port", "botnet_cc", "win.test4", "Alias4", "C2 Network", "2026-11-11 01:00:00", "75", "0", "None", "botnet", "0", "rep4"\n'
            '"2026-11-11 00:00:00", "105", "evil-sample.test", "domain", "botnet_cc", "win.test5", "Alias5", "Domain Evil", "2026-11-11 01:00:00", "80", "0", "None", "c2", "0", "rep5"\n'
            '"2026-11-11 00:00:00", "106", "https://evil-sample.test/bin.exe", "url", "payload_delivery", "win.test6", "Alias6", "URL Delivery", "2026-11-11 01:00:00", "95", "0", "None", "drop", "0", "rep6"\n'
            '# End of export: 6 IOCs\n'
        )
        with tempfile.TemporaryDirectory() as d:
            csv_path = pathlib.Path(d) / "real_format.csv"
            csv_path.write_text(fixture, encoding="utf-8")
            db = tf.ThreatDatabase(pathlib.Path(d) / "tf.db")
            imported = db.import_csv(csv_path)
            self.assertEqual(imported, 6)

            with db.connect() as c:
                # 1. Total IOCs in DB
                self.assertEqual(c.execute("SELECT count(*) FROM iocs").fetchone()[0], 6)

                # 2. Map ioc_type check
                types = {r[0] for r in c.execute("SELECT DISTINCT ioc_type FROM iocs")}
                self.assertEqual(types, {"sha256", "md5", "sha1", "ip:port", "domain", "url"})

                # 3. Malware printable resolution: non-empty, empty -> Unknown, None -> Unknown
                m1 = c.execute("SELECT malware FROM iocs WHERE ioc=?", ("1" * 64,)).fetchone()[0]
                self.assertEqual(m1, "Test Family One")
                m2 = c.execute("SELECT malware FROM iocs WHERE ioc=?", ("2" * 32,)).fetchone()[0]
                self.assertEqual(m2, "Unknown")
                m3 = c.execute("SELECT malware FROM iocs WHERE ioc=?", ("3" * 40,)).fetchone()[0]
                self.assertEqual(m3, "Unknown")

                # 4. Confidence level check
                conf = c.execute("SELECT confidence FROM iocs WHERE ioc=?", ("1" * 64,)).fetchone()[0]
                self.assertEqual(conf, 100)

                # 5. Timestamps check
                fs, ls = c.execute("SELECT first_seen, last_seen FROM iocs WHERE ioc=?", ("1" * 64,)).fetchone()
                self.assertEqual(fs, "2026-11-11 00:00:00")
                self.assertEqual(ls, "2026-11-11 01:00:00")

                # 6. Hashes table: only sha256 with wildcard size
                hashes = c.execute("SELECT sha256, malware, file_size FROM hashes").fetchall()
                self.assertEqual(len(hashes), 1)
                self.assertEqual(hashes[0][0], "1" * 64)
                self.assertEqual(hashes[0][1], "Test Family One")
                self.assertIsNone(hashes[0][2])

    def test_dump_file_preserved_and_import_file_command(self):
        csv_content = (
            '# "first_seen_utc", "ioc_id", "ioc_value", "ioc_type", "malware_printable"\n'
            '"2026-11-11", "201", "' + "e" * 64 + '", "sha256_hash", "DumpFamily"\n'
        ).encode("utf-8")
        buf = io.BytesIO()
        with zipfile.ZipFile(buf, "w", compression=zipfile.ZIP_DEFLATED) as z:
            z.writestr("threatfox.csv", csv_content)
        zip_bytes = buf.getvalue()

        limiter = mock.Mock()
        response = mock.MagicMock()
        response.__enter__.return_value = response
        response.read.side_effect = [zip_bytes, b""]

        with tempfile.TemporaryDirectory() as d:
            dump_file = pathlib.Path(d) / "threatfox-dump.zip"
            db_path = pathlib.Path(d) / "tf.db"

            with mock.patch.object(tf.urllib.request, "urlopen", return_value=response):
                csv_path = tf.fetch_full("https://x/{auth_key}/full.csv.zip", "key123", limiter, dump_path=dump_file, temp_dir=d)
            try:
                self.assertTrue(dump_file.exists())
                self.assertEqual(dump_file.read_bytes(), zip_bytes)
            finally:
                if os.path.exists(csv_path):
                    os.unlink(csv_path)

            # Re-import from preserved dump file via import-file CLI command (0 requests)
            with mock.patch("sys.argv", ["zonendpoint-threatfox.py", "import-file", "--file", str(dump_file), "--db", str(db_path)]):
                ret = tf.main()
                self.assertEqual(ret, 0)

            with tf.ThreatDatabase(db_path).connect() as c:
                self.assertEqual(c.execute("SELECT count(*) FROM iocs").fetchone()[0], 1)
                self.assertEqual(c.execute("SELECT malware FROM iocs WHERE ioc=?", ("e" * 64,)).fetchone()[0], "DumpFamily")

    def test_rejection_rate_over_5_percent_aborts(self):
        content = (
            '# "first_seen_utc", "ioc_id", "ioc_value", "ioc_type", "malware_printable"\n'
            '"2026-11-11", "1", "' + "f" * 64 + '", "sha256_hash", "Good"\n'
            '"2026-11-11", "2", "bad_hash_too_short", "sha256_hash", "Bad1"\n'
            '"2026-11-11", "3", "invalid_ip_format", "ip:port", "Bad2"\n'
        )
        with tempfile.TemporaryDirectory() as d:
            csv_path = pathlib.Path(d) / "bad_ratio.csv"
            csv_path.write_text(content, encoding="utf-8")
            db = tf.ThreatDatabase(pathlib.Path(d) / "tf.db")
            with self.assertRaises(ValueError) as ctx:
                db.import_csv(csv_path)
            self.assertIn("exceeds 5% threshold", str(ctx.exception))
            with db.connect() as c:
                self.assertEqual(c.execute("SELECT count(*) FROM iocs").fetchone()[0], 0)

    def test_import_rows_log_reports_unique_sha256_and_matches_hsb_export(self):
        # Verify Q-29: duplicate sha256 rows in CSV are merged in DB;
        # log outputs unique sha256 count from DB (not raw CSV count) and net_deployable matches .hsb line count.
        rows = [
            {"ioc": "a" * 64, "ioc_type": "sha256", "malware": "OldFam", "confidence_level": 50},
            {"ioc": "a" * 64, "ioc_type": "sha256", "malware": "NewFam", "confidence_level": 90},
            {"ioc": "b" * 64, "ioc_type": "sha256", "malware": "SecondFam", "confidence_level": 80},
        ]
        with tempfile.TemporaryDirectory() as d:
            db_path = pathlib.Path(d) / "tf.db"
            db = tf.ThreatDatabase(db_path)

            # Setup mock MalwareBazaar DB with one overlapping hash ('b'*64)
            mb_path = pathlib.Path(d) / "mb.db"
            import sqlite3
            with sqlite3.connect(mb_path) as c:
                c.execute("CREATE TABLE malware_hashes(sha256 TEXT)")
                c.execute("INSERT INTO malware_hashes VALUES(?)", ("b" * 64,))

            old_env = os.environ.get("MALWAREBAZAAR_DB_PATH")
            os.environ["MALWAREBAZAAR_DB_PATH"] = str(mb_path)
            try:
                buf = io.StringIO()
                with mock.patch("sys.stdout", buf):
                    imported = db.import_rows(rows)
                output = buf.getvalue()

                self.assertEqual(imported, 3)
                # Raw CSV count of sha256 was 3, but unique hashes in DB is 2.
                # MalwareBazaar duplicate is 1, so net deployable is 2 - 1 = 1.
                self.assertIn("sha256=2", output)
                self.assertIn("mb_dupes_excluded=1", output)
                self.assertIn("net_deployable_sha256=1", output)

                # Verify DB merged duplicates and retained latest malware name & confidence
                with db.connect() as c:
                    self.assertEqual(c.execute("SELECT count(*) FROM hashes").fetchone()[0], 2)
                    row = c.execute("SELECT malware, confidence FROM hashes WHERE sha256=?", ("a" * 64,)).fetchone()
                    self.assertEqual(row[0], "NewFam")
                    self.assertEqual(row[1], 90)

                # Verify exported .hsb line count matches net_deployable_sha256 exactly
                hsb_path = pathlib.Path(d) / "test.hsb"
                hsb_count = db.export_hsb(hsb_path)
                self.assertEqual(hsb_count, 1)
                self.assertIn("net_deployable_sha256=1", output)
                hsb_lines = [line for line in hsb_path.read_text(encoding="utf-8").splitlines() if line.strip()]
                self.assertEqual(len(hsb_lines), hsb_count)
            finally:
                if old_env is None:
                    os.environ.pop("MALWAREBAZAAR_DB_PATH", None)
                else:
                    os.environ["MALWAREBAZAAR_DB_PATH"] = old_env


if __name__ == "__main__":
    unittest.main()
