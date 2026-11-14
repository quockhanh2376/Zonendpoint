#!/usr/bin/env python3
"""
Zonendpoint Hourly abuse.ch (MalwareBazaar + ThreatFox) Feed Update (Task V-42)

Enforces:
1. File lock to prevent concurrent executions.
2. Shared rate limiting (3 requests/sliding 60m window, 400/24h across feeds).
3. Execution order: MalwareBazaar recent export -> ThreatFox recent export.
4. If limit hit / 429 / 403 / network error: skips turn, waits for next hour, NO continuous retry.
5. Ingestion in atomic SQLite transactions, deduplication, updating last_seen/updated_at.
6. If new hashes present: exports corresponding .hsb and safely deploys to clamd:
   - Host available RAM >= 3 GB check.
   - Pre-flight verification on temporary DB with base CVD/CLDs (0 errors, 0 FPs).
   - Backup existing active .hsb files.
   - Atomic replacement (0644 clamav:clamav).
   - Single clamd zRELOAD (timeout 120s).
   - PING -> PONG check.
   - Exact signature count delta verification.
   - Automatic rollback on any failure.
7. If NO new hashes: clamd is NOT reloaded (max 1 reload per hour).
8. Writes run status to /opt/zonendpoint/abusech/last-run.json (Auth-Key NEVER leaked).
"""
import argparse
import datetime
import importlib.util
import json
import logging
import os
import pathlib
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error

HERE = pathlib.Path(__file__).resolve().parent

# Import shared rate limiter
from abusech_rate_limit import RateLimiter, RateLimitError, DEFAULT_STATE

# Import MalwareBazaar module
_mb_spec = importlib.util.spec_from_file_location("zonendpoint_malwarebazaar", HERE / "zonendpoint-malwarebazaar.py")
mb = importlib.util.module_from_spec(_mb_spec)
sys.modules[_mb_spec.name] = mb
_mb_spec.loader.exec_module(mb)

# Import ThreatFox module
_tf_spec = importlib.util.spec_from_file_location("zonendpoint_threatfox", HERE / "zonendpoint-threatfox.py")
tf = importlib.util.module_from_spec(_tf_spec)
sys.modules[_tf_spec.name] = tf
_tf_spec.loader.exec_module(tf)

DEFAULT_KEY_FILE = "/etc/zonendpoint/abusech.key"
DEFAULT_MB_DB = "/opt/zonendpoint/malwarebazaar/malwarebazaar.db"
DEFAULT_TF_DB = "/opt/zonendpoint/threatfox/threatfox.db"
DEFAULT_RATE_LIMIT_FILE = DEFAULT_STATE
DEFAULT_LOCK_FILE = "/opt/zonendpoint/abusech/update.lock"
DEFAULT_LAST_RUN_FILE = "/opt/zonendpoint/abusech/last-run.json"

DEFAULT_MB_RECENT_URL = getattr(mb, "DEFAULT_RECENT_DUMP_URL", "https://mb-api.abuse.ch/v2/files/exports/{auth_key}/recent.csv")
DEFAULT_TF_RECENT_URL = getattr(tf, "DEFAULT_RECENT_URL", "https://threatfox-api.abuse.ch/v2/files/exports/{auth_key}/recent.csv")

DEFAULT_MB_HSB_PATH = "/var/lib/clamav/malwarebazaar.hsb"
DEFAULT_TF_HSB_PATH = "/var/lib/clamav/threatfox.hsb"
DEFAULT_MB_BACKUP_DIR = "/var/backups/clamav-malwarebazaar"
DEFAULT_TF_BACKUP_DIR = "/var/backups/clamav-threatfox"
DEFAULT_TF_RECENT_DUMP = getattr(tf, "DEFAULT_RECENT_DUMP", "/opt/zonendpoint/threatfox/last-recent.csv")

DEFAULT_CLAMAV_DB_DIR = "/var/lib/clamav"
DEFAULT_CLAMD_HOST = "127.0.0.1"
DEFAULT_CLAMD_PORT = 3310
DEFAULT_CLAMD_LOG_FILE = "/var/log/clamav/clamav.log"

MIN_RAM_BYTES = 3 * 1024 * 1024 * 1024  # 3 GB
DEFAULT_RELOAD_TIMEOUT = 120.0

logging.basicConfig(
    level=logging.INFO,
    format="[%(asctime)s] [%(levelname)s] %(message)s",
    datefmt="%Y-%m-%d %H:%M:%S",
)
logger = logging.getLogger("zonendpoint-abusech-update")
logger.addFilter(mb.SensitiveKeyFilter())


def mask_key(val: str, key: str = None) -> str:
    return mb.mask_auth_key(val, key)


def is_rate_limit_or_auth_error(exc: Exception) -> bool:
    """Returns True if exception indicates rate limiting (429) or authentication error (401, 403)."""
    if isinstance(exc, RateLimitError) or exc.__class__.__name__ == "RateLimitError":
        return True
    if exc.__class__.__name__ in ("AuthKeyError", "AccountBlockedError"):
        return True
    code = getattr(exc, "code", None) or getattr(exc, "status", None)
    if code in (401, 403, 429):
        return True
    msg = str(exc)
    for c in ("401", "403", "429"):
        if f"HTTP {c}" in msg or f"{c} Unauthorized" in msg or f"{c} Too Many" in msg or f"HTTP error {c}" in msg:
            return True
    return False


def count_hsb_signatures(path: pathlib.Path) -> int:
    """Counts non-empty, non-comment signature lines in a .hsb file."""
    if not path.is_file():
        return 0
    count = 0
    with open(path, "r", encoding="utf-8", errors="replace") as f:
        for line in f:
            stripped = line.strip()
            if stripped and not stripped.startswith("#"):
                count += 1
    return count


def write_last_run(
    status_file: pathlib.Path,
    data: dict,
    auth_key: str = None,
) -> None:
    """Atomically writes last-run.json ensuring zero secret leaks."""
    safe_data = json.loads(json.dumps(data))

    def sanitize(obj):
        if isinstance(obj, dict):
            return {k: sanitize(v) for k, v in obj.items()}
        elif isinstance(obj, list):
            return [sanitize(elem) for elem in obj]
        elif isinstance(obj, str):
            return mask_key(obj, auth_key)
        return obj

    sanitized = sanitize(safe_data)
    status_file.parent.mkdir(parents=True, exist_ok=True)
    temp_file = status_file.with_suffix(".json.tmp")
    try:
        with open(temp_file, "w", encoding="utf-8") as f:
            json.dump(sanitized, f, indent=2)
            f.flush()
            os.fsync(f.fileno())
        try:
            os.chmod(str(temp_file), 0o644)
        except OSError:
            pass
        shutil.move(str(temp_file), str(status_file))
    finally:
        if temp_file.exists():
            temp_file.unlink(missing_ok=True)


def verify_hsbs_on_temp_db(
    hsb_contents: dict,
    clamav_db_dir: str = DEFAULT_CLAMAV_DB_DIR,
    clamscan_bin: str = "clamscan",
) -> dict:
    """
    Validates staged .hsb contents on an isolated temporary DB with base CVD/CLDs.
    hsb_contents: {"malwarebazaar.hsb": content, "threatfox.hsb": content}
    """
    benign_candidates = ["/etc/hosts", "/usr/bin/python3", "/etc/passwd"]
    benign_targets = [p for p in benign_candidates if os.path.exists(p)]
    if not benign_targets:
        benign_targets = [sys.executable]

    with tempfile.TemporaryDirectory(prefix="zon-abusech-tempdb-") as temp_dir:
        temp_dir_p = pathlib.Path(temp_dir)
        clamav_p = pathlib.Path(clamav_db_dir)
        base_db_count = 0
        if clamav_p.is_dir():
            for db_file in clamav_p.iterdir():
                if db_file.is_file() and any(
                    db_file.name.startswith(pfx)
                    for pfx in ("main.", "daily.", "bytecode.")
                ):
                    target_link = temp_dir_p / db_file.name
                    try:
                        os.symlink(str(db_file), str(target_link))
                        base_db_count += 1
                    except Exception:
                        try:
                            shutil.copyfile(str(db_file), str(target_link))
                            base_db_count += 1
                        except Exception as copy_err:
                            logger.warning("Could not link/copy base DB %s: %s", db_file.name, copy_err)

        for fname, content in hsb_contents.items():
            (temp_dir_p / fname).write_text(content, encoding="utf-8")

        logger.info(
            "Running pre-flight verification on temporary DB (base DBs: %d, .hsb files: %d) against %d benign targets...",
            base_db_count,
            len(hsb_contents),
            len(benign_targets),
        )
        try:
            cmd = [clamscan_bin, "-d", str(temp_dir_p)] + benign_targets
            res = subprocess.run(cmd, capture_output=True, text=True)
        except FileNotFoundError:
            logger.warning("'%s' not found on PATH. Pre-flight clamscan verification skipped.", clamscan_bin)
            return {"status": "SKIPPED", "known_viruses": None, "base_dbs": base_db_count}

        if res.returncode != 0:
            logger.error("Temporary DB clamscan failed (exit %d):\nSTDOUT:\n%s\nSTDERR:\n%s", res.returncode, res.stdout, res.stderr)
            raise RuntimeError(f"Temporary DB clamscan failed with exit code {res.returncode}")

        for line in res.stderr.splitlines():
            if "failed to parse or load" in line or "LibClamAV Error" in line:
                raise RuntimeError(f"LibClamAV signature load error: {line}")

        if "Infected files: 0" not in res.stdout:
            raise RuntimeError(f"Benign test targets flagged as infected by .hsb signatures:\n{res.stdout}")

        known_viruses = None
        for line in res.stdout.splitlines():
            m = re.search(r"Known viruses:\s*(\d+)", line)
            if m:
                known_viruses = int(m.group(1))
                break

        logger.info(
            "Temporary DB verification PASSED (0 load errors, 0 False Positives, Known viruses: %s).",
            known_viruses,
        )
        return {
            "status": "PASS",
            "known_viruses": known_viruses,
            "base_dbs": base_db_count,
        }


def deploy_combined_hsbs(
    deploy_items: dict,
    clamav_db_dir: str = DEFAULT_CLAMAV_DB_DIR,
    clamd_host: str = DEFAULT_CLAMD_HOST,
    clamd_port: int = DEFAULT_CLAMD_PORT,
    log_file: str = DEFAULT_CLAMD_LOG_FILE,
    clamscan_bin: str = "clamscan",
    reload_timeout: float = DEFAULT_RELOAD_TIMEOUT,
    min_ram_bytes: int = MIN_RAM_BYTES,
) -> dict:
    """
    Deploys staged .hsb files and executes exactly ONE zero-downtime clamd reload.
    deploy_items = {
        "malwarebazaar": {
            "active_path": Path,
            "backup_dir": Path,
            "staged_content": str,
            "new_signatures_delta": int,
        },
        ...
    }
    """
    avail_ram_bytes = mb.get_available_memory_bytes()
    avail_ram_gb = avail_ram_bytes / (1024 ** 3)
    logger.info("Host available memory check: %.2f GB (threshold: %.2f GB)", avail_ram_gb, min_ram_bytes / (1024 ** 3))
    if avail_ram_bytes < min_ram_bytes:
        msg = f"Available host RAM {avail_ram_gb:.2f} GB is below minimum required {min_ram_bytes / (1024**3):.1f} GB threshold. Halting deployment."
        logger.critical("[ALERT-LOKI] %s", msg)
        raise RuntimeError(msg)

    # Pre-flight check on temporary DB
    temp_hsb_contents = {}
    for feed, item in deploy_items.items():
        fname = item["active_path"].name
        temp_hsb_contents[fname] = item["staged_content"]

    verify_hsbs_on_temp_db(temp_hsb_contents, clamav_db_dir=clamav_db_dir, clamscan_bin=clamscan_bin)

    # Backup existing active files
    backups_created = {}
    for feed, item in deploy_items.items():
        active_p = item["active_path"]
        backup_dir = item["backup_dir"]
        backup_dir.mkdir(parents=True, exist_ok=True)
        backup_file = backup_dir / (active_p.name + ".bak")
        if active_p.is_file():
            shutil.copy2(str(active_p), str(backup_file))
            logger.info("Backed up active file '%s' -> '%s'", active_p, backup_file)
            backups_created[feed] = backup_file
        else:
            backups_created[feed] = None

    # Inspect clamd signature count before reload
    log_p = pathlib.Path(log_file) if log_file else None
    log_pos_before = 0
    sig_before = None
    if log_p and log_p.is_file():
        try:
            log_pos_before = log_p.stat().st_size
            sig_before = mb.get_last_logged_signature_count(str(log_p))
            logger.info("clamd signature count before reload: %s (log offset: %d)", sig_before, log_pos_before)
        except Exception as e:
            logger.warning("Could not inspect clamd log '%s': %s", log_p, e)

    # Atomic replacement of active files
    replaced_feeds = []
    try:
        for feed, item in deploy_items.items():
            active_p = item["active_path"]
            tmp_active = active_p.with_suffix(".tmp")
            tmp_active.write_text(item["staged_content"], encoding="utf-8")
            if os.name == "posix":
                try:
                    shutil.chown(str(tmp_active), user="clamav", group="clamav")
                except Exception as e:
                    logger.warning("Could not chown clamav:clamav on '%s': %s", tmp_active, e)
            try:
                os.chmod(str(tmp_active), 0o644)
            except OSError:
                pass
            tmp_active.replace(active_p)
            replaced_feeds.append(feed)
            logger.info("Atomically replaced active file '%s'.", active_p)

        # Trigger single clamd reload via zRELOAD
        logger.info("Triggering clamd reload via zRELOAD socket command...")
        reload_start_time = time.time()
        reload_sent = mb.trigger_clamd_zreload(host=clamd_host, port=clamd_port)
        if not reload_sent:
            raise RuntimeError(f"Failed to send zRELOAD to clamd at {clamd_host}:{clamd_port}")

        # Poll for reload completion and PING -> PONG
        logger.info("Waiting for clamd reload and verifying PING -> PONG (timeout: %.1fs)...", reload_timeout)
        max_wait = reload_timeout
        ping_ok = False
        sig_after = None

        while (time.time() - reload_start_time) < max_wait:
            time.sleep(2.0)
            if log_p and log_p.is_file():
                try:
                    with open(log_p, "r", encoding="utf-8", errors="replace") as lf:
                        lf.seek(log_pos_before)
                        new_log = lf.read()
                        matches = re.findall(r"Database correctly reloaded \((\d+) signatures\)", new_log)
                        if matches:
                            sig_after = int(matches[-1])
                except Exception as e:
                    logger.warning("Error reading clamd log: %s", e)

            if mb.check_clamd_ping(host=clamd_host, port=clamd_port):
                ping_ok = True

            if log_p and log_p.is_file():
                if sig_after is not None and ping_ok:
                    break
            else:
                if ping_ok:
                    break

        reload_duration = round(time.time() - reload_start_time, 2)
        if not ping_ok or (reload_duration > reload_timeout):
            msg = f"clamd reload timed out after {reload_duration:.1f}s (> {reload_timeout:.1f}s threshold) or PING failed! Triggering emergency rollback."
            logger.critical("[ALERT-LOKI] %s", msg)
            raise RuntimeError(msg)

        # Verify signature delta
        expected_total_delta = sum(item["new_signatures_delta"] for item in deploy_items.values())
        if sig_before is not None and sig_after is not None:
            delta = sig_after - sig_before
            if delta != expected_total_delta:
                raise RuntimeError(
                    f"Signature count delta mismatch: expected delta +{expected_total_delta}, "
                    f"got delta +{delta} ({sig_before} -> {sig_after})"
                )
            logger.info("Signature count delta verified: %d -> %d (+%d)", sig_before, sig_after, delta)

        return {
            "reloaded": True,
            "reload_duration": reload_duration,
            "sig_before": sig_before,
            "sig_after": sig_after,
            "expected_delta": expected_total_delta,
        }
    except Exception as exc:
        logger.critical("[ALERT-LOKI] Deployment failure: %s. Initiating emergency rollback...", exc)
        for feed in replaced_feeds:
            item = deploy_items[feed]
            active_p = item["active_path"]
            backup_file = backups_created.get(feed)
            if backup_file and backup_file.is_file():
                logger.info("Restoring '%s' from backup '%s'...", active_p, backup_file)
                shutil.copy2(str(backup_file), str(active_p))
            else:
                logger.info("Removing '%s' (no prior backup)...", active_p)
                active_p.unlink(missing_ok=True)

        logger.info("Sending zRELOAD to restore clamd engine state...")
        mb.trigger_clamd_zreload(host=clamd_host, port=clamd_port)
        time.sleep(2.0)
        mb.check_clamd_ping(host=clamd_host, port=clamd_port)
        raise


def run_hourly_update(args) -> int:
    start_time = time.time()
    now_iso = datetime.datetime.now(datetime.timezone.utc).isoformat()

    status_record = {
        "timestamp": now_iso,
        "duration_seconds": 0.0,
        "requests": {
            "malwarebazaar": 0,
            "threatfox": 0,
            "total": 0,
        },
        "new_hashes": {
            "malwarebazaar": 0,
            "threatfox": 0,
            "total": 0,
        },
        "reloaded": False,
        "signatures_before": None,
        "signatures_after": None,
        "reload_duration_seconds": 0.0,
        "error": None,
    }

    last_run_p = pathlib.Path(args.last_run_file)
    lock_p = pathlib.Path(args.lock_file)
    lock_p.parent.mkdir(parents=True, exist_ok=True)

    # 1. File lock to prevent concurrent executions
    lock_fd = None
    try:
        lock_fd = open(str(lock_p), "a+")
        try:
            import fcntl
            fcntl.flock(lock_fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except (BlockingIOError, IOError):
            logger.info("Another update process is currently running (lock '%s' held). Exiting without overlapping.", lock_p)
            return 0
    except Exception as e:
        logger.warning("Could not establish lock on '%s': %s", lock_p, e)

    auth_key = None
    limiter = None
    try:
        # Load Auth-Key safely
        try:
            auth_key = mb.load_auth_key(args.key_file)
            mb.SensitiveKeyFilter.register_key(auth_key)
        except Exception as e:
            msg = f"Failed to load Auth-Key from {args.key_file}: {mask_key(e)}"
            logger.error(msg)
            status_record["error"] = msg
            return 1

        limiter = RateLimiter(args.rate_limit_file)

        # Guard: verify and reserve 2 slots under shared limiter lock before starting turn
        try:
            stat = limiter.get_status()
            logger.info(
                "Shared abuse.ch rate limit status before guard: %d/%d hourly (sliding 60m), %d/%d daily (24h)",
                stat["hourly_used"], stat["hourly_max"], stat["daily_used"], stat["daily_max"]
            )
            res_token = limiter.reserve_slots(count=2, sources=("malwarebazaar", "threatfox"))
            logger.info(
                "Guard passed: reserved 2 slots for MalwareBazaar + ThreatFox (token: %s) under shared limiter lock.",
                res_token
            )
        except RateLimitError as e:
            msg = f"{e}"
            logger.warning("Guard skipped turn: %s", msg)
            status_record["error"] = msg
            return 0
        except Exception as e:
            msg = f"Guard rate limit inspection failed: {mask_key(e, auth_key)}"
            logger.error(msg)
            status_record["error"] = msg
            return 0

        # 2. Step 1: MalwareBazaar recent update
        mb_db = mb.MalwareDatabase(args.mb_db)
        mb_count_before = mb_db.get_hash_count()
        mb_client = mb.MalwareBazaarClient(
            base_url=getattr(args, "mb_base_url", mb.DEFAULT_BASE_URL),
            key_file=args.key_file,
            rate_limiter=limiter,
            timeout=args.timeout,
        )

        mb_error = None
        mb_temp_csv = None
        mb_temp_zip = None
        try:
            logger.info("Step 1: Fetching MalwareBazaar recent additions (export URL)...")
            mb_temp_csv, mb_temp_zip = mb_client.fetch_recent_export(
                recent_url=args.mb_url,
                timeout=args.timeout,
                max_retries=0,
            )
            status_record["requests"]["malwarebazaar"] = 1
            status_record["requests"]["total"] += 1

            mb_stats = mb_db.import_dump_csv(
                csv_path=mb_temp_csv,
                source="malwarebazaar_recent",
                update_last_seen=True,
            )
            mb_count_after = mb_db.get_hash_count()
            mb_new_hashes = max(0, mb_count_after - mb_count_before)
            status_record["new_hashes"]["malwarebazaar"] = mb_new_hashes
            logger.info(
                "MalwareBazaar update completed: total_lines=%d, inserted=%d, new_hashes=%d, total_in_db=%d",
                mb_stats["total_dump_lines"], mb_stats["inserted"], mb_new_hashes, mb_count_after
            )
        except (RateLimitError, urllib.error.HTTPError, urllib.error.URLError, TimeoutError, Exception) as exc:
            mb_error = mask_key(exc, auth_key)
            logger.warning("MalwareBazaar update failed: %s. Skipping without retry per Zon policy.", mb_error)
            status_record["error"] = f"MalwareBazaar: {mb_error}"
            if is_rate_limit_or_auth_error(exc):
                logger.warning(
                    "MalwareBazaar encountered rate limit or auth error (401/403/429): %s. Terminating run immediately without calling ThreatFox per Zon policy.",
                    mb_error,
                )
                status_record["reloaded"] = False
                return 0
        finally:
            if mb_temp_csv and mb_temp_csv.is_file():
                mb_temp_csv.unlink(missing_ok=True)
            if mb_temp_zip and mb_temp_zip.is_file():
                mb_temp_zip.unlink(missing_ok=True)

        # 3. Step 2: ThreatFox recent update
        tf_db = tf.ThreatDatabase(args.tf_db)
        tf_hashes_before = tf_db.get_hash_count()
        tf_error = None
        tf_temp_csv = None
        try:
            logger.info("Step 2: Fetching ThreatFox recent additions (export URL)...")
            tf_temp_csv = tf.fetch_recent(
                url=args.tf_url,
                key=auth_key,
                limiter=limiter,
                dump_path=getattr(args, "tf_recent_dump", None),
                timeout=args.timeout,
            )
            status_record["requests"]["threatfox"] = 1
            status_record["requests"]["total"] += 1

            tf_imported = tf_db.import_csv(tf_temp_csv)
            tf_hashes_after = tf_db.get_hash_count()
            tf_new_hashes = max(0, tf_hashes_after - tf_hashes_before)
            status_record["new_hashes"]["threatfox"] = tf_new_hashes
            logger.info(
                "ThreatFox update completed: imported_rows=%d, new_hashes=%d, total_in_db=%d",
                tf_imported, tf_new_hashes, tf_hashes_after
            )
        except (RateLimitError, urllib.error.HTTPError, urllib.error.URLError, TimeoutError, Exception) as exc:
            tf_error = mask_key(exc, auth_key)
            logger.warning("ThreatFox update failed: %s. Skipping without retry per Zon policy.", tf_error)
            status_record["error"] = (
                f"{status_record['error']}; ThreatFox: {tf_error}" if status_record["error"] else f"ThreatFox: {tf_error}"
            )
        finally:
            if tf_temp_csv and os.path.exists(tf_temp_csv):
                # Don't delete if it is the preserved dump
                if getattr(args, "tf_recent_dump", None) and tf_temp_csv == str(args.tf_recent_dump):
                    pass
                elif tf_temp_csv.endswith("last-recent.csv"):
                    pass
                else:
                    try:
                        os.unlink(tf_temp_csv)
                    except OSError:
                        pass

        status_record["new_hashes"]["total"] = (
            status_record["new_hashes"]["malwarebazaar"] + status_record["new_hashes"]["threatfox"]
        )

        # 4. Check whether to export and deploy to clamd
        mb_active_p = pathlib.Path(args.mb_hsb_path)
        tf_active_p = pathlib.Path(args.tf_hsb_path)

        mb_active_count = count_hsb_signatures(mb_active_p)
        tf_active_count = count_hsb_signatures(tf_active_p)

        mb_db_total = mb_db.get_hash_count()
        tf_net_unique = tf_db.get_net_deployable_count(mb_db=args.mb_db)

        mb_new_sigs = max(0, mb_db_total - mb_active_count)
        tf_new_sigs = max(0, tf_net_unique - tf_active_count)

        logger.info(
            "Signature evaluation: MalwareBazaar (active: %d, db: %d, new: %d), ThreatFox (active: %d, net_unique: %d, new: %d)",
            mb_active_count, mb_db_total, mb_new_sigs, tf_active_count, tf_net_unique, tf_new_sigs
        )

        if mb_new_sigs == 0 and tf_new_sigs == 0:
            logger.info("No new signatures to deploy across feeds. Skipping clamd reload (clamd reloads at most once per hour).")
            status_record["reloaded"] = False
            status_record["duration_seconds"] = round(time.time() - start_time, 2)
            write_last_run(last_run_p, status_record, auth_key)
            return 0

        # Build staging content for feeds that have new signatures
        deploy_items = {}
        if mb_new_sigs > 0:
            logger.info("Generating staging .hsb for MalwareBazaar (%d signatures)...", mb_db_total)
            mb_content, _ = mb.generate_hsb_content(mb_db, prefix="MalwareBazaar", allow_wildcard=True)
            deploy_items["malwarebazaar"] = {
                "active_path": mb_active_p,
                "backup_dir": pathlib.Path(args.mb_backup_dir),
                "staged_content": mb_content,
                "new_signatures_delta": mb_new_sigs,
            }

        if tf_new_sigs > 0:
            logger.info("Generating staging .hsb for ThreatFox (%d net unique signatures)...", tf_net_unique)
            with tempfile.NamedTemporaryFile("w+", suffix=".hsb", delete=False) as tf_tmp:
                tf_tmp_path = tf_tmp.name
            try:
                tf_db.export_hsb(tf_tmp_path)
                tf_content = pathlib.Path(tf_tmp_path).read_text(encoding="utf-8")
            finally:
                if os.path.exists(tf_tmp_path):
                    os.unlink(tf_tmp_path)

            deploy_items["threatfox"] = {
                "active_path": tf_active_p,
                "backup_dir": pathlib.Path(args.tf_backup_dir),
                "staged_content": tf_content,
                "new_signatures_delta": tf_new_sigs,
            }

        # Safe deployment with single clamd reload
        deploy_result = deploy_combined_hsbs(
            deploy_items=deploy_items,
            clamav_db_dir=args.clamav_db_dir,
            clamd_host=args.clamd_host,
            clamd_port=args.clamd_port,
            log_file=args.log_file,
            clamscan_bin=args.clamscan_bin,
            reload_timeout=args.reload_timeout,
            min_ram_bytes=args.min_ram_bytes,
        )

        status_record["reloaded"] = deploy_result["reloaded"]
        status_record["signatures_before"] = deploy_result["sig_before"]
        status_record["signatures_after"] = deploy_result["sig_after"]
        status_record["reload_duration_seconds"] = deploy_result["reload_duration"]
        logger.info(
            "SUCCESS: Combined deployment completed! clamd signatures %s -> %s in %.2fs",
            deploy_result["sig_before"], deploy_result["sig_after"], deploy_result["reload_duration"]
        )
        return 0

    except Exception as exc:
        msg = f"Hourly update encountered an unhandled error: {mask_key(exc, auth_key)}"
        logger.error(msg)
        status_record["error"] = msg
        return 1
    finally:
        if limiter is not None:
            try:
                limiter.cancel_unconsumed_reservations()
            except Exception as e:
                logger.warning("Could not cancel unconsumed reservations: %s", e)
        status_record["duration_seconds"] = round(time.time() - start_time, 2)
        write_last_run(last_run_p, status_record, auth_key)
        if lock_fd:
            try:
                import fcntl
                fcntl.flock(lock_fd, fcntl.LOCK_UN)
            except Exception:
                pass
            try:
                lock_fd.close()
            except Exception:
                pass


def parse_args():
    p = argparse.ArgumentParser(description="Zonendpoint Hourly abuse.ch Feed Update (Task V-42)")
    p.add_argument("--key-file", default=DEFAULT_KEY_FILE)
    p.add_argument("--mb-db", default=DEFAULT_MB_DB)
    p.add_argument("--tf-db", default=DEFAULT_TF_DB)
    p.add_argument("--rate-limit-file", default=DEFAULT_RATE_LIMIT_FILE)
    p.add_argument("--lock-file", default=DEFAULT_LOCK_FILE)
    p.add_argument("--last-run-file", default=DEFAULT_LAST_RUN_FILE)

    p.add_argument("--mb-url", default=DEFAULT_MB_RECENT_URL)
    p.add_argument("--tf-url", default=DEFAULT_TF_RECENT_URL)
    p.add_argument("--tf-recent-dump", default=DEFAULT_TF_RECENT_DUMP)

    p.add_argument("--mb-hsb-path", default=DEFAULT_MB_HSB_PATH)
    p.add_argument("--tf-hsb-path", default=DEFAULT_TF_HSB_PATH)
    p.add_argument("--mb-backup-dir", default=DEFAULT_MB_BACKUP_DIR)
    p.add_argument("--tf-backup-dir", default=DEFAULT_TF_BACKUP_DIR)

    p.add_argument("--clamav-db-dir", default=DEFAULT_CLAMAV_DB_DIR)
    p.add_argument("--clamd-host", default=DEFAULT_CLAMD_HOST)
    p.add_argument("--clamd-port", type=int, default=DEFAULT_CLAMD_PORT)
    p.add_argument("--log-file", default=DEFAULT_CLAMD_LOG_FILE)
    p.add_argument("--clamscan-bin", default="clamscan")

    p.add_argument("--timeout", type=float, default=120.0)
    p.add_argument("--reload-timeout", type=float, default=DEFAULT_RELOAD_TIMEOUT)
    p.add_argument("--min-ram-bytes", type=int, default=MIN_RAM_BYTES)
    return p.parse_args()


if __name__ == "__main__":
    sys.exit(run_hourly_update(parse_args()))
