import csv
import importlib.util
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
            c=db.connect()
            self.assertEqual(c.execute("select count(*) from iocs").fetchone()[0],2)
            self.assertEqual(c.execute("select ioc from iocs where ioc_type='domain'").fetchone()[0],"example.com")
            c.close()

    def test_bad_zip_and_bad_csv_do_not_replace_existing_db_rows(self):
        with tempfile.TemporaryDirectory() as d:
            db=tf.ThreatDatabase(pathlib.Path(d)/"tf.db")
            db.import_rows([{"ioc":"a"*64,"ioc_type":"sha256","malware":"kept"}])
            broken=pathlib.Path(d)/"bad.csv"; broken.write_text("\n")
            with self.assertRaises(ValueError): db.import_csv(broken)
            c=db.connect(); self.assertEqual(c.execute("select malware from iocs").fetchone()[0],"kept"); c.close()

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
            self.assertIn("ThreatFox.Fam.UNOFFICIAL:73",out.read_text())
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
            c=db.connect(); self.assertEqual(c.execute("select count(*) from iocs").fetchone()[0],0); c.close()

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
        response=mock.MagicMock(); response.__enter__.return_value=response; response.read.return_value=b"not a zip"
        with mock.patch.object(tf.urllib.request,"urlopen",return_value=response):
            with self.assertRaises(ValueError): tf.fetch_full("https://x/{auth_key}/x", "k", limiter)


if __name__ == "__main__": unittest.main()
