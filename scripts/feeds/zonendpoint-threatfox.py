#!/usr/bin/env python3
"""Manual ThreatFox metadata ingestion and ClamAV SHA-256 signature deployment."""
import argparse, csv, datetime, hashlib, importlib.util, io, json, os, pathlib, re, shutil, sqlite3, sys, tempfile, urllib.error, urllib.request, zipfile
import urllib.parse
from abusech_rate_limit import RateLimiter, RateLimitError, DEFAULT_STATE

DEFAULT_KEY = "/etc/zonendpoint/abusech.key"
DEFAULT_DB = "/opt/zonendpoint/threatfox/threatfox.db"
DEFAULT_URL = "https://threatfox-api.abuse.ch/v2/files/exports/{auth_key}/full.csv.zip"
DEFAULT_DUMP = "/opt/zonendpoint/threatfox/last-full.csv.zip"
MAX_BYTES = 512 * 1024 * 1024
HASH = re.compile(r"^[0-9a-f]{64}$")
MD5_REGEX = re.compile(r"^[0-9a-f]{32}$")
SHA1_REGEX = re.compile(r"^[0-9a-f]{40}$")
HERE = pathlib.Path(__file__).resolve().parent
_spec = importlib.util.spec_from_file_location("malwarebazaar", HERE / "zonendpoint-malwarebazaar.py")
mb = importlib.util.module_from_spec(_spec)
sys.modules[_spec.name] = mb
_spec.loader.exec_module(mb)

IOC_TYPE_MAP = {
    "sha256_hash": "sha256",
    "sha256": "sha256",
    "md5_hash": "md5",
    "md5": "md5",
    "sha1_hash": "sha1",
    "sha1": "sha1",
    "ip:port": "ip:port",
    "ip_port": "ip:port",
    "domain": "domain",
    "url": "url",
}


def mask(value, key):
    return str(value).replace(key, "<AUTH-KEY>") if key else str(value)


def extract_csv_from_zip(zip_path, temp_dir=None):
    with zipfile.ZipFile(zip_path) as z:
        items = [i for i in z.infolist() if not i.is_dir() and i.filename.lower().endswith(".csv")]
        if len(items) != 1 or items[0].file_size > MAX_BYTES:
            raise ValueError("Unexpected CSV ZIP contents or size")
        fd, csv_path = tempfile.mkstemp(suffix=".csv", prefix="tf_extracted_", dir=temp_dir)
        csv_tmp = pathlib.Path(csv_path)
        with z.open(items[0]) as src, os.fdopen(fd, "wb") as dst:
            shutil.copyfileobj(src, dst)
        if csv_tmp.stat().st_size > MAX_BYTES:
            csv_tmp.unlink(missing_ok=True)
            raise ValueError("Uncompressed CSV exceeds size limit")
        return str(csv_tmp)


class ThreatDatabase:
    def __init__(self, path):
        self.path = pathlib.Path(path)
        self.path.parent.mkdir(parents=True, exist_ok=True)
        with self.connect() as c:
            c.execute("CREATE TABLE IF NOT EXISTS iocs (ioc TEXT NOT NULL, ioc_type TEXT NOT NULL, malware TEXT, confidence INTEGER, first_seen TEXT, last_seen TEXT, source TEXT NOT NULL, updated_at TEXT NOT NULL, PRIMARY KEY(ioc,ioc_type))")
            c.execute("CREATE TABLE IF NOT EXISTS hashes (sha256 TEXT PRIMARY KEY, malware TEXT, file_size INTEGER, confidence INTEGER)")

    def connect(self):
        return sqlite3.connect(self.path, uri=True)

    def import_rows(self, rows):
        now = datetime.datetime.now(datetime.timezone.utc).isoformat()
        conn = self.connect()
        imported = 0
        rejected = 0
        counts_by_type = {}
        try:
            conn.execute("BEGIN IMMEDIATE")
            for row in rows:
                raw_ioc = str(row.get("ioc", "") or "").strip()
                if (raw_ioc.startswith('"') and raw_ioc.endswith('"')) or (raw_ioc.startswith("'") and raw_ioc.endswith("'")):
                    raw_ioc = raw_ioc[1:-1].strip()

                raw_typ = str(row.get("ioc_type", "") or "").strip().lower()
                if (raw_typ.startswith('"') and raw_typ.endswith('"')) or (raw_typ.startswith("'") and raw_typ.endswith("'")):
                    raw_typ = raw_typ[1:-1].strip()

                typ = IOC_TYPE_MAP.get(raw_typ)
                if not typ and raw_typ.endswith("_hash") and raw_typ[:-5] in {"sha256", "md5", "sha1"}:
                    typ = raw_typ[:-5]

                if not raw_ioc or not typ or typ not in {"sha256", "md5", "sha1", "ip:port", "domain", "url"}:
                    rejected += 1
                    continue

                normalized = raw_ioc.lower() if typ in {"sha256", "md5", "sha1", "domain"} else raw_ioc
                if typ == "sha256" and not HASH.fullmatch(normalized):
                    rejected += 1
                    continue
                if typ == "md5" and not MD5_REGEX.fullmatch(normalized):
                    rejected += 1
                    continue
                if typ == "sha1" and not SHA1_REGEX.fullmatch(normalized):
                    rejected += 1
                    continue
                if typ == "ip:port" and (":" not in normalized or normalized.startswith(":") or normalized.endswith(":")):
                    rejected += 1
                    continue

                raw_malware = str(row.get("malware", "") or "").strip()
                if (raw_malware.startswith('"') and raw_malware.endswith('"')) or (raw_malware.startswith("'") and raw_malware.endswith("'")):
                    raw_malware = raw_malware[1:-1].strip()
                if not raw_malware or raw_malware.lower() in ("", "none", "null", "unknown"):
                    malware = "Unknown"
                else:
                    malware = raw_malware

                confidence = None
                raw_conf = row.get("confidence_level")
                if raw_conf is not None:
                    c_str = str(raw_conf).strip().strip('"').strip("'")
                    if c_str.isdigit():
                        confidence = int(c_str)

                first_seen = str(row.get("first_seen", "") or "").strip().strip('"').strip("'") or None
                last_seen = str(row.get("last_seen", "") or "").strip().strip('"').strip("'") or None

                conn.execute(
                    "INSERT INTO iocs VALUES(?,?,?,?,?,?,?,?) ON CONFLICT(ioc,ioc_type) DO UPDATE SET "
                    "malware=excluded.malware, confidence=excluded.confidence, "
                    "first_seen=COALESCE(iocs.first_seen,excluded.first_seen), last_seen=excluded.last_seen, "
                    "source=excluded.source, updated_at=excluded.updated_at",
                    (normalized, typ, malware, confidence, first_seen, last_seen, "ThreatFox", now),
                )
                counts_by_type[typ] = counts_by_type.get(typ, 0) + 1

                if typ == "sha256":
                    size = row.get("file_size")
                    size = int(size) if str(size or "").isdigit() and int(size or 0) > 0 else None
                    conn.execute(
                        "INSERT INTO hashes VALUES(?,?,?,?) ON CONFLICT(sha256) DO UPDATE SET "
                        "malware=excluded.malware, file_size=excluded.file_size, confidence=excluded.confidence",
                        (normalized, malware, size, confidence),
                    )
                imported += 1

            total = imported + rejected
            reject_rate = (rejected / total) if total > 0 else 0.0

            if not imported:
                conn.rollback()
                print(f"[ALERT-LOKI] ThreatFox import failed: 0 valid rows imported (total={total}, rejected={rejected})", file=sys.stderr)
                raise ValueError("No valid ThreatFox IOC rows found; database left unchanged")

            if total > 0 and reject_rate > 0.05:
                conn.rollback()
                print(f"[ALERT-LOKI] ThreatFox import rejected rows {rejected}/{total} ({reject_rate:.2%}) exceeds 5% threshold; database left unchanged", file=sys.stderr)
                raise ValueError(f"ThreatFox import rejection rate {reject_rate:.2%} ({rejected}/{total}) exceeds 5% threshold; database left unchanged")

            conn.commit()

            # Check duplicate sha256 with MalwareBazaar
            mb_db = os.getenv("MALWAREBAZAAR_DB_PATH", "/opt/zonendpoint/malwarebazaar/malwarebazaar.db")
            mb_dupes = 0
            if pathlib.Path(mb_db).exists():
                try:
                    uri = f"file:{pathlib.Path(mb_db).resolve().as_posix()}?immutable=1"
                    conn.execute(f"ATTACH DATABASE '{uri}' AS mbdb")
                    mb_dupes = conn.execute("SELECT count(*) FROM hashes WHERE EXISTS (SELECT 1 FROM mbdb.malware_hashes mb WHERE mb.sha256=hashes.sha256)").fetchone()[0]
                    conn.execute("DETACH DATABASE mbdb")
                except Exception:
                    pass

            sha256_count = conn.execute("SELECT count(*) FROM hashes").fetchone()[0]
            net_sha256 = sha256_count - mb_dupes

            print(
                f"ThreatFox import completed: total_rows={total}, imported={imported}, rejected={rejected} ({reject_rate:.2%}), "
                f"by_type={counts_by_type}, sha256={sha256_count}, mb_dupes_excluded={mb_dupes}, net_deployable_sha256={net_sha256}"
            )
        except Exception:
            conn.rollback()
            raise
        finally:
            conn.close()
        return imported

    def import_csv(self, path):
        with open(path, encoding="utf-8-sig", errors="replace", newline="") as f:
            header_line = None
            for raw_line in f:
                stripped = raw_line.strip()
                if not stripped:
                    continue
                if stripped.startswith("#"):
                    cleaned = stripped.lstrip("#").strip()
                    if not cleaned:
                        continue
                    try:
                        parts = next(csv.reader([cleaned], skipinitialspace=True))
                    except Exception:
                        continue
                    parts_lower = [p.strip().strip('"').strip("'").lower() for p in parts if p.strip()]
                    if any(n in parts_lower for n in ("ioc", "ioc_value", "indicator", "value", "sha256", "sha256_hash", "md5", "md5_hash")):
                        header_line = cleaned + "\n"
                        break
                    continue
                else:
                    try:
                        parts = next(csv.reader([stripped], skipinitialspace=True))
                    except Exception:
                        raise ValueError("CSV header not found before data lines")
                    parts_lower = [p.strip().strip('"').strip("'").lower() for p in parts if p.strip()]
                    if any(n in parts_lower for n in ("ioc", "ioc_value", "indicator", "value", "sha256", "sha256_hash", "md5", "md5_hash")):
                        header_line = stripped + "\n"
                        break
                    else:
                        raise ValueError("CSV header not found before data lines")

            if header_line is None:
                raise ValueError("CSV has no header")

            def line_iter():
                yield header_line
                for l in f:
                    s = l.strip()
                    if not s or s.startswith("#"):
                        continue
                    yield l

            reader = csv.DictReader(line_iter(), skipinitialspace=True)
            raw_header = [n.strip().strip('"').strip("'") for n in (reader.fieldnames or []) if n]
            print(f"CSV header detected: {raw_header}")
            fields = {n.strip().strip('"').strip("'").lower(): n for n in (reader.fieldnames or []) if n}
            if not any(n in fields for n in ("ioc", "ioc_value", "indicator", "value", "sha256", "sha256_hash", "md5", "md5_hash")):
                raise ValueError("CSV header has no recognized IOC value column")
            if not any(n in fields for n in ("ioc_type", "type", "ioc_type_desc")):
                raise ValueError("CSV header has no recognized IOC type column")

            def get(row, *names):
                for name in names:
                    if name in fields:
                        val = row.get(fields[name])
                        if val is not None:
                            val_str = str(val).strip()
                            if (val_str.startswith('"') and val_str.endswith('"')) or (val_str.startswith("'") and val_str.endswith("'")):
                                val_str = val_str[1:-1].strip()
                            return val_str
                return None

            rows_gen = (
                {
                    "ioc": get(row, "ioc_value", "ioc", "indicator", "value", "sha256", "sha256_hash", "md5", "md5_hash"),
                    "ioc_type": get(row, "ioc_type", "type", "ioc_type_desc"),
                    "malware": get(row, "malware_printable", "malware", "signature"),
                    "confidence_level": get(row, "confidence_level", "confidence"),
                    "first_seen": get(row, "first_seen_utc", "first_seen"),
                    "last_seen": get(row, "last_seen_utc", "last_seen"),
                    "file_size": get(row, "file_size", "size"),
                }
                for row in reader
            )
            count = self.import_rows(rows_gen)
        return count

    def export_hsb(self, output):
        conn = self.connect()
        mb_conn = None
        try:
            # MalwareBazaar hashes are excluded to prevent duplicate clamd signatures.
            mb_db = os.getenv("MALWAREBAZAAR_DB_PATH", "/opt/zonendpoint/malwarebazaar/malwarebazaar.db")
            existing = set()
            if pathlib.Path(mb_db).exists():
                mb_conn = sqlite3.connect(f"file:{pathlib.Path(mb_db).resolve().as_posix()}?immutable=1", uri=True)
                existing = {r[0] for r in mb_conn.execute("SELECT sha256 FROM malware_hashes")}
            lines = []
            for sha, malware, size in conn.execute("SELECT sha256,malware,file_size FROM hashes ORDER BY sha256"):
                if not HASH.fullmatch(sha) or sha in existing:
                    continue
                safe = re.sub(r"[^A-Za-z0-9_.-]", "_", malware or "Unknown").strip("._") or "Unknown"
                name = f"ThreatFox.{safe}.UNOFFICIAL"
                lines.append(f"{sha}:{size}:{name}\n" if size else f"{sha}:*:{name}:73\n")
            pathlib.Path(output).write_text("".join(lines), encoding="utf-8")
            return len(lines)
        finally:
            conn.close()
            if mb_conn:
                mb_conn.close()


def fetch_full(url, key, limiter, dump_path=None, timeout=120, temp_dir=None):
    limiter.check_and_record("threatfox")
    actual = url.format(auth_key=key)
    req = urllib.request.Request(actual, headers={"User-Agent": "Zonendpoint ThreatFox importer/1.0"})

    if dump_path is None:
        target_dump = pathlib.Path(temp_dir) / "last-full.csv.zip" if temp_dir else pathlib.Path(DEFAULT_DUMP)
    else:
        target_dump = pathlib.Path(dump_path)

    target_dump.parent.mkdir(parents=True, exist_ok=True)
    tmp_dump = target_dump.with_suffix(".zip.tmp")

    try:
        try:
            with urllib.request.urlopen(req, timeout=timeout) as response:
                total_bytes = 0
                with open(tmp_dump, "wb") as f:
                    while True:
                        chunk = response.read(65536)
                        if not chunk:
                            break
                        f.write(chunk)
                        total_bytes += len(chunk)
                        if total_bytes > MAX_BYTES:
                            raise ValueError("ThreatFox export exceeds download size limit")
        except Exception as exc:
            if isinstance(exc, (ValueError, RateLimitError)):
                raise
            raise RuntimeError(mask(exc, key)) from None

        try:
            with zipfile.ZipFile(tmp_dump) as z:
                items = [i for i in z.infolist() if not i.is_dir() and i.filename.lower().endswith(".csv")]
                if len(items) != 1 or items[0].file_size > MAX_BYTES:
                    raise ValueError("Unexpected CSV ZIP contents or size")
        except zipfile.BadZipFile:
            raise ValueError("ThreatFox export is not a valid ZIP") from None

        # Atomically move to final dump location and set root 600
        shutil.move(str(tmp_dump), str(target_dump))
        try:
            os.chmod(str(target_dump), 0o600)
        except OSError:
            pass
        print(f"Preserved ThreatFox dump file: {target_dump} (mode 0600)")

        # Extract CSV for ingestion
        csv_path = extract_csv_from_zip(str(target_dump), temp_dir=temp_dir)
        return csv_path
    finally:
        if tmp_dump.exists():
            tmp_dump.unlink(missing_ok=True)


def check_api(key, limiter, timeout=30):
    limiter.check_and_record("threatfox")
    req = urllib.request.Request(
        "https://threatfox-api.abuse.ch/api/v1/",
        data=json.dumps({"query": "get_iocs", "days": 1}).encode(),
        headers={"Auth-Key": key, "Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout) as response:
            raw = response.read(4 * 1024 * 1024 + 1)
        if len(raw) > 4 * 1024 * 1024:
            raise ValueError("ThreatFox check response exceeds size limit")
        payload = json.loads(raw)
        if payload.get("query_status") not in ("ok", "no_result"):
            raise ValueError("ThreatFox API rejected check request")
        return payload.get("query_status")
    except Exception as exc:
        raise RuntimeError(mask(exc, key)) from None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("command", choices=["check", "fetch-full", "import-file", "export-hsb", "deploy-hsb"])
    ap.add_argument("--key-file", default=DEFAULT_KEY)
    ap.add_argument("--db", default=DEFAULT_DB)
    ap.add_argument("--rate-limit-file", default=DEFAULT_STATE)
    ap.add_argument("--output", default="/tmp/threatfox.hsb")
    ap.add_argument("--dump-file", default=DEFAULT_DUMP)
    ap.add_argument("--file", default=None, help="Local ZIP or CSV file for import-file command")
    ap.add_argument("--url", default=DEFAULT_URL)
    ap.add_argument("--hsb-path", default="/var/lib/clamav/threatfox.hsb")
    ap.add_argument("--malwarebazaar-pending", action="store_true", help="Enforce MalwareBazaar's alternating turn when both feeds are waiting")
    ap.add_argument("--backup-dir", default="/var/backups/clamav-threatfox")
    ap.add_argument("--clamav-db-dir", default="/var/lib/clamav")
    ap.add_argument("--clamd-host", default="127.0.0.1")
    ap.add_argument("--clamd-port", type=int, default=3310)
    ap.add_argument("--log-file", default="/var/log/clamav/clamav.log")
    args = ap.parse_args()
    key = ""
    try:
        if args.command == "check":
            key = mb.load_auth_key(args.key_file)
            print(f"ThreatFox API check: {check_api(key, RateLimiter(args.rate_limit_file, other_waiting=args.malwarebazaar_pending))}")
        elif args.command == "fetch-full":
            key = mb.load_auth_key(args.key_file)
            csv_path = fetch_full(
                args.url,
                key,
                RateLimiter(args.rate_limit_file, other_waiting=args.malwarebazaar_pending),
                dump_path=args.dump_file,
            )
            try:
                db = ThreatDatabase(args.db)
                count = db.import_csv(csv_path)
                print(f"ThreatFox export imported: rows={count}")
            finally:
                if os.path.exists(csv_path):
                    os.unlink(csv_path)
        elif args.command == "import-file":
            input_file = pathlib.Path(args.file or args.dump_file)
            if not input_file.is_file():
                raise FileNotFoundError(f"Import file not found: {input_file}")
            csv_path = None
            is_temp_csv = False
            try:
                if input_file.suffix.lower() == ".zip":
                    csv_path = extract_csv_from_zip(str(input_file))
                    is_temp_csv = True
                else:
                    csv_path = str(input_file)
                db = ThreatDatabase(args.db)
                count = db.import_csv(csv_path)
                print(f"ThreatFox file imported: rows={count}")
            finally:
                if is_temp_csv and csv_path and os.path.exists(csv_path):
                    os.unlink(csv_path)
        elif args.command == "export-hsb":
            count = ThreatDatabase(args.db).export_hsb(args.output)
            print(f"Exported {count} signatures")
        else:
            db = ThreatDatabase(args.db)
            # Reuse MalwareBazaar's guarded staged deploy via an adapter exposing its expected query.
            class Adapter:
                def _get_connection(self):
                    c = db.connect()
                    mb_db = os.getenv("MALWAREBAZAAR_DB_PATH", "/opt/zonendpoint/malwarebazaar/malwarebazaar.db")
                    if pathlib.Path(mb_db).exists():
                        uri = f"file:{pathlib.Path(mb_db).resolve().as_posix()}?immutable=1"
                        try:
                            c.execute(f"ATTACH DATABASE '{uri}' AS mbdb")
                        except Exception:
                            c.execute("ATTACH DATABASE ? AS mbdb", (mb_db,))
                        c.execute("CREATE TEMP VIEW malware_hashes AS SELECT sha256,file_size,malware AS signature FROM hashes WHERE NOT EXISTS (SELECT 1 FROM mbdb.malware_hashes mb WHERE mb.sha256=hashes.sha256)")
                    else:
                        c.execute("CREATE TEMP VIEW malware_hashes AS SELECT sha256,file_size,malware AS signature FROM hashes")
                    return c

            with db.connect() as conn:
                tf_hashes = conn.execute("SELECT count(*) FROM hashes").fetchone()[0]
                mb_db = os.getenv("MALWAREBAZAAR_DB_PATH", "/opt/zonendpoint/malwarebazaar/malwarebazaar.db")
                dupes = 0
                if pathlib.Path(mb_db).exists():
                    uri = f"file:{pathlib.Path(mb_db).resolve().as_posix()}?immutable=1"
                    try:
                        conn.execute(f"ATTACH DATABASE '{uri}' AS mbdb")
                    except Exception:
                        conn.execute("ATTACH DATABASE ? AS mbdb", (mb_db,))
                    dupes = conn.execute("SELECT count(*) FROM hashes WHERE EXISTS (SELECT 1 FROM mbdb.malware_hashes mb WHERE mb.sha256=hashes.sha256)").fetchone()[0]
                net_unique = tf_hashes - dupes
                print(f"ThreatFox total hashes: {tf_hashes}, duplicates with MalwareBazaar excluded: {dupes}, net unique to deploy: {net_unique}")

            result = mb.deploy_hsb(
                Adapter(),
                active_hsb_path=args.hsb_path,
                backup_dir=args.backup_dir,
                clamav_db_dir=args.clamav_db_dir,
                clamd_host=args.clamd_host,
                clamd_port=args.clamd_port,
                log_file=args.log_file,
                prefix="ThreatFox",
                allow_wildcard=True,
            )
            print(f"Deploy result: {result}")
    except Exception as exc:
        print(f"ERROR: {mask(exc, key)}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
