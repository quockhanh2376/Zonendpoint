#!/usr/bin/env python3
"""Manual ThreatFox metadata ingestion and ClamAV SHA-256 signature deployment."""
import argparse, csv, datetime, hashlib, importlib.util, io, json, os, pathlib, re, shutil, sqlite3, sys, tempfile, urllib.error, urllib.request, zipfile
import urllib.parse
from abusech_rate_limit import RateLimiter, RateLimitError, DEFAULT_STATE

DEFAULT_KEY = "/etc/zonendpoint/abusech.key"
DEFAULT_DB = "/opt/zonendpoint/threatfox/threatfox.db"
DEFAULT_URL = "https://threatfox-api.abuse.ch/v2/files/exports/{auth_key}/full.csv.zip"
MAX_BYTES = 512 * 1024 * 1024
HASH = re.compile(r"^[0-9a-f]{64}$")
HERE = pathlib.Path(__file__).resolve().parent
_spec = importlib.util.spec_from_file_location("malwarebazaar", HERE / "zonendpoint-malwarebazaar.py")
mb = importlib.util.module_from_spec(_spec)
sys.modules[_spec.name] = mb
_spec.loader.exec_module(mb)


def mask(value, key):
    return str(value).replace(key, "<AUTH-KEY>") if key else str(value)


class ThreatDatabase:
    def __init__(self, path):
        self.path = pathlib.Path(path)
        self.path.parent.mkdir(parents=True, exist_ok=True)
        with self.connect() as c:
            c.execute("CREATE TABLE IF NOT EXISTS iocs (ioc TEXT NOT NULL, ioc_type TEXT NOT NULL, malware TEXT, confidence INTEGER, first_seen TEXT, last_seen TEXT, source TEXT NOT NULL, updated_at TEXT NOT NULL, PRIMARY KEY(ioc,ioc_type))")
            c.execute("CREATE TABLE IF NOT EXISTS hashes (sha256 TEXT PRIMARY KEY, malware TEXT, file_size INTEGER, confidence INTEGER)")

    def connect(self):
        return sqlite3.connect(self.path)

    def import_rows(self, rows):
        now = datetime.datetime.now(datetime.timezone.utc).isoformat()
        conn = self.connect()
        imported = 0
        try:
            conn.execute("BEGIN IMMEDIATE")
            for row in rows:
                ioc = str(row.get("ioc", "")).strip()
                typ = str(row.get("ioc_type", "")).strip().lower()
                aliases = {"sha256_hash": "sha256", "md5_hash": "md5", "ip_port": "ip:port"}
                typ = aliases.get(typ, typ)
                if not ioc or typ not in {"sha256", "md5", "ip:port", "domain", "url"}:
                    continue
                normalized = ioc.lower() if typ in {"sha256", "md5", "domain"} else ioc
                if typ == "sha256" and not HASH.fullmatch(normalized):
                    continue
                if typ == "md5" and not re.fullmatch(r"[0-9a-f]{32}", normalized):
                    continue
                conn.execute("INSERT INTO iocs VALUES(?,?,?,?,?,?,?,?) ON CONFLICT(ioc,ioc_type) DO UPDATE SET malware=excluded.malware, confidence=excluded.confidence, first_seen=COALESCE(iocs.first_seen,excluded.first_seen), last_seen=excluded.last_seen, source=excluded.source, updated_at=excluded.updated_at", (normalized, typ, row.get("malware") or "Unknown", row.get("confidence_level"), row.get("first_seen"), row.get("last_seen"), "ThreatFox", now))
                if typ == "sha256":
                    size = row.get("file_size")
                    size = int(size) if str(size or "").isdigit() and int(size or 0) > 0 else None
                    conn.execute("INSERT INTO hashes VALUES(?,?,?,?) ON CONFLICT(sha256) DO UPDATE SET malware=excluded.malware, file_size=excluded.file_size, confidence=excluded.confidence", (normalized, row.get("malware") or "Unknown", size, row.get("confidence_level")))
                imported += 1
            if not imported:
                raise ValueError("No valid ThreatFox IOC rows found; database left unchanged")
            conn.commit()
        except Exception:
            conn.rollback()
            raise
        finally:
            conn.close()
        return imported

    def import_csv(self, path):
        with open(path, encoding="utf-8-sig", newline="") as f:
            reader = csv.DictReader(f)
            if not reader.fieldnames:
                raise ValueError("CSV has no header")
            fields = {n.strip().lower(): n for n in reader.fieldnames if n}
            if not any(n in fields for n in ("ioc", "ioc_value", "indicator", "value", "sha256", "sha256_hash", "md5", "md5_hash")):
                raise ValueError("CSV header has no recognized IOC value column")
            if not any(n in fields for n in ("ioc_type", "type", "ioc_type_desc")):
                raise ValueError("CSV header has no recognized IOC type column")
            def get(row, *names):
                for name in names:
                    if name in fields: return row.get(fields[name])
                return None
            count = self.import_rows({"ioc": get(row,"ioc","ioc_value","indicator","value","sha256","sha256_hash","md5","md5_hash"), "ioc_type": get(row,"ioc_type","type","ioc_type_desc"), "malware": get(row,"malware","malware_printable","signature"), "confidence_level": get(row,"confidence_level","confidence"), "first_seen": get(row,"first_seen","first_seen_utc"), "last_seen": get(row,"last_seen","last_seen_utc"), "file_size": get(row,"file_size","size")} for row in reader)
        return count

    def export_hsb(self, output):
        conn = self.connect()
        mb_conn = None
        try:
            # MalwareBazaar hashes are excluded to prevent duplicate clamd signatures.
            mb_db = os.getenv("MALWAREBAZAAR_DB_PATH", "/opt/zonendpoint/malwarebazaar/malwarebazaar.db")
            existing = set()
            if pathlib.Path(mb_db).exists():
                mb_conn = sqlite3.connect(mb_db)
                existing = {r[0] for r in mb_conn.execute("SELECT sha256 FROM malware_hashes")}
            lines=[]
            for sha, malware, size in conn.execute("SELECT sha256,malware,file_size FROM hashes ORDER BY sha256"):
                if not HASH.fullmatch(sha) or sha in existing: continue
                safe = re.sub(r"[^A-Za-z0-9_.-]", "_", malware or "Unknown").strip("._") or "Unknown"
                name = f"ThreatFox.{safe}.UNOFFICIAL"
                lines.append(f"{sha}:{size}:{name}\n" if size else f"{sha}:*:{name}:73\n")
            pathlib.Path(output).write_text("".join(lines), encoding="utf-8")
            return len(lines)
        finally:
            conn.close()
            if mb_conn: mb_conn.close()


def fetch_full(url, key, limiter, timeout=120):
    limiter.check_and_record("threatfox")
    actual = url.format(auth_key=key)
    req = urllib.request.Request(actual, headers={"User-Agent":"Zonendpoint ThreatFox importer/1.0"})
    try:
        with urllib.request.urlopen(req, timeout=timeout) as response:
            data = response.read(MAX_BYTES + 1)
    except Exception as exc:
        raise RuntimeError(mask(exc, key)) from None
    if len(data) > MAX_BYTES: raise ValueError("ThreatFox export exceeds download size limit")
    try:
        with zipfile.ZipFile(io.BytesIO(data)) as z:
            items=[i for i in z.infolist() if not i.is_dir() and i.filename.lower().endswith(".csv")]
            if len(items)!=1 or items[0].file_size > MAX_BYTES: raise ValueError("Unexpected CSV ZIP contents or size")
            raw=z.read(items[0])
            if len(raw)>MAX_BYTES: raise ValueError("Uncompressed CSV exceeds size limit")
            return raw
    except zipfile.BadZipFile:
        raise ValueError("ThreatFox export is not a valid ZIP") from None


def check_api(key, limiter, timeout=30):
    limiter.check_and_record("threatfox")
    req=urllib.request.Request("https://threatfox-api.abuse.ch/api/v1/", data=json.dumps({"query":"get_iocs","days":1}).encode(), headers={"Auth-Key":key,"Content-Type":"application/json"}, method="POST")
    try:
        with urllib.request.urlopen(req,timeout=timeout) as response:
            raw=response.read(4*1024*1024+1)
        if len(raw)>4*1024*1024: raise ValueError("ThreatFox check response exceeds size limit")
        payload=json.loads(raw)
        if payload.get("query_status") not in ("ok","no_result"): raise ValueError("ThreatFox API rejected check request")
        return payload.get("query_status")
    except Exception as exc:
        raise RuntimeError(mask(exc,key)) from None


def main():
    ap=argparse.ArgumentParser()
    ap.add_argument("command", choices=["check","fetch-full","export-hsb","deploy-hsb"])
    ap.add_argument("--key-file", default=DEFAULT_KEY); ap.add_argument("--db",default=DEFAULT_DB)
    ap.add_argument("--rate-limit-file",default=DEFAULT_STATE); ap.add_argument("--output",default="/tmp/threatfox.hsb")
    ap.add_argument("--url",default=DEFAULT_URL); ap.add_argument("--hsb-path",default="/var/lib/clamav/threatfox.hsb")
    ap.add_argument("--malwarebazaar-pending",action="store_true",help="Enforce MalwareBazaar's alternating turn when both feeds are waiting")
    ap.add_argument("--backup-dir",default="/var/backups/clamav-threatfox"); ap.add_argument("--clamav-db-dir",default="/var/lib/clamav")
    ap.add_argument("--clamd-host",default="127.0.0.1"); ap.add_argument("--clamd-port",type=int,default=3310); ap.add_argument("--log-file",default="/var/log/clamav/clamav.log")
    args=ap.parse_args(); key=""
    try:
        if args.command == "check":
            key=mb.load_auth_key(args.key_file); print(f"ThreatFox API check: {check_api(key,RateLimiter(args.rate_limit_file,other_waiting=args.malwarebazaar_pending))}")
        elif args.command == "fetch-full":
            key=mb.load_auth_key(args.key_file); raw=fetch_full(args.url,key,RateLimiter(args.rate_limit_file,other_waiting=args.malwarebazaar_pending))
            with tempfile.NamedTemporaryFile(suffix=".csv",delete=False) as f: f.write(raw); csv_path=f.name
            try:
                db=ThreatDatabase(args.db); count=db.import_csv(csv_path)
                print(f"ThreatFox export imported: rows={count}")
            finally: os.unlink(csv_path)
        elif args.command=="export-hsb":
            count=ThreatDatabase(args.db).export_hsb(args.output); print(f"Exported {count} signatures")
        else:
            db=ThreatDatabase(args.db)
            # Reuse MalwareBazaar's guarded staged deploy via an adapter exposing its expected query.
            class Adapter:
                def _get_connection(self):
                    c=db.connect()
                    mb_db=os.getenv("MALWAREBAZAAR_DB_PATH", "/opt/zonendpoint/malwarebazaar/malwarebazaar.db")
                    if pathlib.Path(mb_db).exists():
                        c.execute("ATTACH DATABASE ? AS mbdb", (mb_db,))
                        c.execute("CREATE TEMP VIEW malware_hashes AS SELECT sha256,file_size,malware AS signature FROM hashes WHERE NOT EXISTS (SELECT 1 FROM mbdb.malware_hashes mb WHERE mb.sha256=hashes.sha256)")
                    else:
                        c.execute("CREATE TEMP VIEW malware_hashes AS SELECT sha256,file_size,malware AS signature FROM hashes")
                    return c
            mb.deploy_hsb(Adapter(),active_hsb_path=args.hsb_path,backup_dir=args.backup_dir,clamav_db_dir=args.clamav_db_dir,clamd_host=args.clamd_host,clamd_port=args.clamd_port,prefix="ThreatFox",allow_wildcard=True)
    except Exception as exc:
        print(f"ERROR: {mask(exc,key)}",file=sys.stderr); return 1
    return 0

if __name__=="__main__": sys.exit(main())
