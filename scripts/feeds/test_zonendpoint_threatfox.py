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
            csv_path=pathlib.Path(d)/"mock.csv"
            csv_path.write_text("malware,last_seen,ioc_type,confidence_level,ioc,first_seen\nFam,2026-11-10,sha256,90,"+"A"*64+",2026-11-01\nFam,2026-11-10,sha256,95,"+"a"*64+",2026-11-01\nFam,,domain,50,Example.COM,2026-11-01\n",encoding="utf-8")
            db=tf.ThreatDatabase(pathlib.Path(d)/"tf.db")
            self.assertEqual(db.import_csv(csv_path),3)
            with db.connect() as c:
                self.assertEqual(c.execute("select count(*) from iocs").fetchone()[0],2)
                self.assertEqual(c.execute("select ioc from iocs where ioc_type='domain'").fetchone()[0],"example.com")

    def test_csv_with_comment_headers_and_preamble(self):
        with tempfile.TemporaryDirectory() as d:
            csv_path=pathlib.Path(d)/"commented.csv"
            csv_path.write_text(
                '# ThreatFox export database\n'
                '# Generated at 2026-11-11\n'
                '# "id","ioc","ioc_type","threat_type","malware","malware_printable","first_seen_utc","last_seen_utc","confidence_level"\n'
                '1,"' + "c"*64 + '","sha256","botnet","Trojan","Trojan.Clean","2026-11-01","2026-11-10",90\n',
                encoding="utf-8"
            )
            db=tf.ThreatDatabase(pathlib.Path(d)/"tf.db")
            self.assertEqual(db.import_csv(csv_path), 1)
            with db.connect() as c:
                self.assertEqual(c.execute("select malware from iocs where ioc=?", ("c"*64,)).fetchone()[0], "Trojan")

    def test_bad_zip_and_bad_csv_do_not_replace_existing_db_rows(self):
        with tempfile.TemporaryDirectory() as d:
            db=tf.ThreatDatabase(pathlib.Path(d)/"tf.db")
            db.import_rows([{"ioc":"a"*64,"ioc_type":"sha256","malware":"kept"}])
            broken=pathlib.Path(d)/"bad.csv"; broken.write_text("\n")
            with self.assertRaises(ValueError): db.import_csv(broken)
            with db.connect() as c:
                self.assertEqual(c.execute("select malware from iocs").fetchone()[0],"kept")

    def test_hsb_wildcard_and_malwarebazaar_hash_exclusion(self):
        with tempfile.TemporaryDirectory() as d:
            db=tf.ThreatDatabase(pathlib.Path(d)/"tf.db")
            db.import_rows([{"ioc":"a"*64,"ioc_type":"sha256","malware":"Fam"},{"ioc":"b"*64,"ioc_type":"sha256","malware":""}])
            mb_path=pathlib.Path(d)/"mb.db"
            import sqlite3
            with sqlite3.connect(mb_path) as c:
                c.execute("CREATE TABLE malware_hashes(sha256 TEXT)"); c.execute("INSERT INTO malware_hashes VALUES(?)",("a"*64,))
            old=os.environ.get("MALWAREBAZAAR_DB_PATH"); os.environ["MALWAREBAZAAR_DB_PATH"]=str(mb_path)
            out=pathlib.Path(d)/"out.hsb"
            try: self.assertEqual(db.export_hsb(out),1)
            finally:
                if old is None: os.environ.pop("MALWAREBAZAAR_DB_PATH",None)
                else: os.environ["MALWAREBAZAAR_DB_PATH"]=old
            self.assertNotIn("ThreatFox.Fam.UNOFFICIAL",out.read_text())
            self.assertIn("ThreatFox.Unknown.UNOFFICIAL:73",out.read_text())

    def test_invalid_zip_and_key_masking(self):
        key="secret-token"
        self.assertNotIn(key,tf.mask(f"download failed at /exports/{key}/full.csv.zip",key))
        with tempfile.TemporaryDirectory() as d:
            limiter=mock.Mock()
            limiter.check_and_record.return_value=None
            with mock.patch.object(tf.urllib.request,"urlopen",side_effect=OSError(f"failed /exports/{key}/file")):
                with self.assertRaises(RuntimeError) as err:
                    tf.fetch_full("https://example.test/{auth_key}/dump.zip",key,limiter)
            self.assertNotIn(key,str(err.exception))

    def test_malformed_rows_are_not_accepted(self):
        with tempfile.TemporaryDirectory() as d:
            db=tf.ThreatDatabase(pathlib.Path(d)/"tf.db")
            with self.assertRaises(ValueError): db.import_rows([{"ioc":"not-a-hash","ioc_type":"sha256"},{"ioc":"1.2.3.4","ioc_type":"ip"}])
            with db.connect() as c:
                self.assertEqual(c.execute("select count(*) from iocs").fetchone()[0],0)

    def test_http_429_is_fail_fast_and_masks_key(self):
        key="secret-token"
        limiter=mock.Mock()
        error=urllib.error.HTTPError(f"https://x/{key}/full.csv.zip",429,"Too Many Requests",{},None)
        with mock.patch.object(tf.urllib.request,"urlopen",side_effect=error):
            with self.assertRaises(RuntimeError) as err: tf.fetch_full("https://x/{auth_key}/full.csv.zip",key,limiter)
        self.assertNotIn(key,str(err.exception))
        self.assertEqual(limiter.check_and_record.call_count,1)

    def test_invalid_zip_is_rejected(self):
        limiter=mock.Mock()
        response=mock.MagicMock()
        response.__enter__.return_value=response
        response.read.side_effect=[b"not a zip", b""]
        with mock.patch.object(tf.urllib.request,"urlopen",return_value=response):
            with self.assertRaises(ValueError): tf.fetch_full("https://x/{auth_key}/x", "k", limiter)

    def test_fetch_full_streams_to_temp_file_and_extracts_cleanly(self):
        csv_content = b"ioc,ioc_type,malware\n" + b"d" * 64 + b",sha256,TestMalware\n"
        buf = io.BytesIO()
        with zipfile.ZipFile(buf, "w", compression=zipfile.ZIP_DEFLATED) as z:
            z.writestr("threatfox.csv", csv_content)
        zip_bytes = buf.getvalue()

        # Split into 20-byte chunks to verify streaming assembly
        chunk_size = 20
        chunks = [zip_bytes[i:i + chunk_size] for i in range(0, len(zip_bytes), chunk_size)]
        chunks.append(b"")

        limiter = mock.Mock()
        response = mock.MagicMock()
        response.__enter__.return_value = response
        response.read.side_effect = chunks

        with tempfile.TemporaryDirectory() as d:
            with mock.patch.object(tf.urllib.request, "urlopen", return_value=response):
                csv_path = tf.fetch_full("https://x/{auth_key}/full.csv.zip", "key123", limiter, temp_dir=d)
            try:
                self.assertTrue(pathlib.Path(csv_path).exists())
                self.assertEqual(pathlib.Path(csv_path).read_bytes(), csv_content)
            finally:
                if os.path.exists(csv_path):
                    os.unlink(csv_path)

    def test_fetch_full_oversized_download_is_rejected(self):
        limiter = mock.Mock()
        response = mock.MagicMock()
        response.__enter__.return_value = response
        # 10 chunks of 20 bytes = 200 bytes total
        response.read.side_effect = [b"x" * 20] * 10

        with mock.patch.object(tf, "MAX_BYTES", 50):
            with mock.patch.object(tf.urllib.request, "urlopen", return_value=response):
                with self.assertRaises(ValueError) as ctx:
                    tf.fetch_full("https://x/{auth_key}/full.csv.zip", "key123", limiter)
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


if __name__ == "__main__": unittest.main()
