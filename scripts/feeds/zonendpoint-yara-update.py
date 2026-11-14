#!/usr/bin/env python3
"""
Zonendpoint Automated Daily YARA-Forge Core Feed Updater (Task V-25)

Automates daily updates for YARAHQ/yara-forge Core ruleset:
1. Queries latest official release from YARAHQ/yara-forge repo; halts if no new version.
2. Downloads core package zip, verifies checksum if available, filters with filter_yara_clamav.py (pruning SUSP_).
3. Pre-flight tests on temporary DB: validates syntax/loading, verifies reasonable rule count,
   confirms benign samples are CLEAN, confirms ZonQA positive control rule detects benign marker.
4. Atomically deploys to /var/lib/clamav/yara-forge-core.yar, reloads clamd, verifies PING -> PONG,
   and retains previous version for rollback.
"""

import sys
import os
import re
import json
import time
import socket
import shutil
import hashlib
import tempfile
import argparse
import subprocess
import urllib.request
import urllib.error

CURRENT_TAG_FILE = "/opt/zonendpoint/yara/current_tag.txt"
DEFAULT_HOLD_FILE = "/opt/zonendpoint/HOLD-yara"
DEFAULT_LOCK_FILE = os.environ.get("DEPLOY_LOCK_FILE", "/run/lock/zonendpoint-clamd-deploy.lock")
FILTER_SCRIPT_PATH = "/opt/zonendpoint/yara/filter_yara_clamav.py"
QA_CONTROL_RULE_PATH = "/opt/zonendpoint/qa/zonqa-control.yar"
ACTIVE_YARA_PATH = "/var/lib/clamav/yara-forge-core.yar"
BACKUP_DIR = "/var/backups/clamav-yara"
BACKUP_YARA_PATH = "/var/backups/clamav-yara/yara-forge-core.yar.bak"
PREVIOUS_TAG_FILE = "/var/backups/clamav-yara/previous_tag.txt"
WORK_DIR = "/opt/zonendpoint/yara/work"
CLAMD_LOG_FILE = "/var/log/clamav/clamav.log"

REPO_API_URL = "https://api.github.com/repos/YARAHQ/yara-forge/releases/latest"
USER_AGENT = "Zonendpoint-Feed-Updater/1.0 (+https://github.com/quockhanh2376/Zonendpoint)"
ZONQA_MARKER = b"ZONQA_BENIGN_MARKER_TEST_STRING_20261108_CLAMAV_LAB\n"

# Reasonable rule count bounds for filtered core ruleset
MIN_RULES = 2500
MAX_RULES = 8000


def log(msg: str):
    timestamp = time.strftime("%Y-%m-%d %H:%M:%S")
    print(f"[{timestamp}] {msg}", flush=True)


def get_current_tag() -> str:
    if os.path.exists(CURRENT_TAG_FILE):
        try:
            with open(CURRENT_TAG_FILE, "r", encoding="utf-8") as f:
                return f.read().strip()
        except Exception as e:
            log(f"Warning: unable to read {CURRENT_TAG_FILE}: {e}")
    return ""


def fetch_latest_release() -> dict:
    req = urllib.request.Request(
        REPO_API_URL,
        headers={
            "User-Agent": USER_AGENT,
            "Accept": "application/vnd.github.v3+json",
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            data = resp.read().decode("utf-8")
            return json.loads(data)
    except urllib.error.URLError as e:
        raise RuntimeError(f"Failed to query GitHub release API: {e}")


def download_file(url: str, dest_path: str):
    log(f"Downloading from {url} to {dest_path}...")
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(req, timeout=120) as resp, open(dest_path, "wb") as out:
        shutil.copyfileobj(resp, out)
    log(f"Downloaded {os.path.getsize(dest_path):,} bytes.")


def compute_sha256(file_path: str) -> str:
    h = hashlib.sha256()
    with open(file_path, "rb") as f:
        while chunk := f.read(65536):
            h.update(chunk)
    return h.hexdigest()


def find_checksum_in_release(release: dict, asset_name: str) -> str:
    # 1. Look for asset named {asset_name}.sha256 or checksums.txt
    for a in release.get("assets", []):
        aname = a.get("name", "")
        if aname == f"{asset_name}.sha256" or aname in ("checksums.txt", "SHA256SUMS.txt"):
            dl_url = a.get("browser_download_url")
            if dl_url:
                try:
                    req = urllib.request.Request(dl_url, headers={"User-Agent": USER_AGENT})
                    with urllib.request.urlopen(req, timeout=30) as resp:
                        content = resp.read().decode("utf-8", errors="replace")
                        for line in content.splitlines():
                            if asset_name in line:
                                parts = line.split()
                                if len(parts) >= 1 and len(parts[0]) == 64:
                                    return parts[0].lower()
                            elif len(line.strip()) == 64 and aname.endswith(".sha256"):
                                return line.strip().lower()
                except Exception as e:
                    log(f"Warning: could not fetch checksum asset {aname}: {e}")

    # 2. Check release body for 64-char hex hash
    body = release.get("body", "")
    for line in body.splitlines():
        if asset_name in line:
            m = re.search(r"\b([a-fA-F0-9]{64})\b", line)
            if m:
                return m.group(1).lower()

    return ""


def extract_core_rules(zip_path: str, extract_dir: str) -> str:
    import zipfile
    with zipfile.ZipFile(zip_path, "r") as zf:
        # Find yara-rules-core.yar or any .yar in packages/core/
        target_name = None
        for name in zf.namelist():
            if name.endswith("yara-rules-core.yar") or (name.startswith("packages/core/") and name.endswith(".yar")):
                target_name = name
                break
        if not target_name:
            raise RuntimeError(f"Could not find core .yar rule file inside {zip_path}")
        
        extracted_path = zf.extract(target_name, extract_dir)
        log(f"Extracted {target_name} ({os.path.getsize(extracted_path):,} bytes).")
        return extracted_path


def run_clamscan(args: list) -> subprocess.CompletedProcess:
    return subprocess.run(["clamscan"] + args, capture_output=True, text=True, timeout=120)


def check_clamd_ping(host: str = "127.0.0.1", port: int = 3310, timeout: float = 5.0) -> bool:
    try:
        with socket.create_connection((host, port), timeout=timeout) as s:
            s.sendall(b"PING\n")
            resp = s.recv(1024).decode("utf-8", errors="replace").strip()
            return resp == "PONG"
    except Exception as e:
        log(f"Clamd PING check failed: {e}")
        return False


def verify_on_temp_db(staging_yar: str, accepted_count: int):
    log("=== Running Pre-flight Validation on Temporary DB ===")
    
    # 1. Sanity check rule count
    if not (MIN_RULES <= accepted_count <= MAX_RULES):
        raise RuntimeError(
            f"Accepted rule count {accepted_count} outside reasonable bounds [{MIN_RULES}, {MAX_RULES}]."
        )
    log(f"Rule count sanity check PASS ({accepted_count} rules within [{MIN_RULES}, {MAX_RULES}]).")

    with tempfile.TemporaryDirectory(prefix="zon-yara-test-") as temp_db:
        temp_yar = os.path.join(temp_db, "yara-forge-core.yar")
        shutil.copyfile(staging_yar, temp_yar)

        # 2. Syntax and load test via clamscan
        log("Testing rule load via clamscan on benign /etc/hosts...")
        res = run_clamscan(["-d", temp_yar, "/etc/hosts"])
        if res.returncode != 0:
            raise RuntimeError(f"clamscan syntax/load test failed with code {res.returncode}:\n{res.stderr}\n{res.stdout}")
        
        # Verify no LibClamAV errors
        for line in res.stderr.splitlines():
            if "failed to parse or load" in line or "LibClamAV Error" in line:
                raise RuntimeError(f"clamscan reported LibClamAV load error: {line}")
        log("Rule load test PASS (0 load errors).")

        # 3. Benign samples verification (must be CLEAN)
        benign_targets = ["/etc/hosts", "/usr/bin/python3", "/etc/passwd"]
        existing_targets = [p for p in benign_targets if os.path.exists(p)]
        log(f"Scanning {len(existing_targets)} benign system files with temporary DB...")
        res = run_clamscan(["-d", temp_yar] + existing_targets)
        if res.returncode != 0:
            raise RuntimeError(
                f"Benign sample scan failed (possible False Positive):\n{res.stdout}\n{res.stderr}"
            )
        if "Infected files: 0" not in res.stdout:
            raise RuntimeError(f"Benign sample scan reported infection:\n{res.stdout}")
        log("Benign samples test PASS (0 False Positives).")

        # 4. ZonQA positive control rule verification (loaded temporarily)
        if not os.path.exists(QA_CONTROL_RULE_PATH):
            raise RuntimeError(f"ZonQA control rule not found at {QA_CONTROL_RULE_PATH}")
        
        qa_dest = os.path.join(temp_db, "zonqa-control.yar")
        shutil.copyfile(QA_CONTROL_RULE_PATH, qa_dest)

        with tempfile.NamedTemporaryFile(mode="wb", delete=False) as tf:
            tf.write(ZONQA_MARKER)
            marker_file = tf.name

        try:
            log("Testing ZonQA positive control rule on temporary DB...")
            res = run_clamscan(["-d", temp_db, marker_file])
            if res.returncode != 1:
                raise RuntimeError(
                    f"ZonQA marker test expected exit code 1 (INFECTED), got {res.returncode}:\n{res.stdout}"
                )
            if "YARA.ZonQA_Benign_Marker.UNOFFICIAL FOUND" not in res.stdout:
                raise RuntimeError(
                    f"ZonQA marker rule did not trigger expected signature name:\n{res.stdout}"
                )
            if "Infected files: 1" not in res.stdout:
                raise RuntimeError(f"ZonQA marker scan did not report 1 infected file:\n{res.stdout}")
            log("ZonQA positive control marker detection PASS.")
        finally:
            if os.path.exists(marker_file):
                os.unlink(marker_file)

    log("All temporary DB pre-flight checks PASSED successfully.")


def safe_atomic_write(
    target_path: str,
    source_file: str,
    user: str = "clamav",
    group: str = "clamav",
    mode: int = 0o644,
):
    """
    Safely copies source_file to target_path using mkstemp, fchown, fchmod, fsync, and replace (Task L-11).
    """
    target_dir = os.path.dirname(target_path)
    os.makedirs(target_dir, exist_ok=True)
    fd, tmp_path = tempfile.mkstemp(
        dir=target_dir,
        prefix=f".{os.path.basename(target_path)}.tmp-",
    )
    try:
        with open(source_file, "rb") as sf:
            while True:
                chunk = sf.read(65536)
                if not chunk:
                    break
                os.write(fd, chunk)
        if os.name == "posix":
            try:
                import pwd, grp
                uid = pwd.getpwnam(user).pw_uid
                gid = grp.getgrnam(group).gr_gid
                os.fchown(fd, uid, gid)
            except Exception:
                pass
        try:
            os.fchmod(fd, mode)
        except OSError:
            pass
        os.fsync(fd)
    finally:
        os.close(fd)
    os.replace(tmp_path, target_path)


def deploy_and_reload(
    staging_yar: str,
    new_tag: str,
    accepted_count: int,
    log_file: str = CLAMD_LOG_FILE,
    reload_timeout: float = 120.0,
):
    log("=== Deploying Filtered Ruleset Atomically ===")
    os.makedirs(BACKUP_DIR, exist_ok=True)

    # 1. Backup current active file and tag
    if os.path.exists(ACTIVE_YARA_PATH):
        log(f"Backing up current {ACTIVE_YARA_PATH} -> {BACKUP_YARA_PATH}...")
        shutil.copyfile(ACTIVE_YARA_PATH, BACKUP_YARA_PATH)
        current_tag = get_current_tag()
        if current_tag:
            with open(PREVIOUS_TAG_FILE, "w", encoding="utf-8") as f:
                f.write(current_tag + "\n")
            log(f"Recorded previous release tag: {current_tag}")

    # 2. Record clamd log offset before reload for reload line detection
    log_pos_before = 0
    if log_file and os.path.exists(log_file):
        try:
            log_pos_before = os.path.getsize(log_file)
            log(f"clamd log initial offset: {log_pos_before} on '{log_file}'")
        except Exception as e:
            log(f"Warning: could not get initial clamd log size: {e}")

    # 3. Atomic replacement via secure temp file (Task L-11)
    safe_atomic_write(ACTIVE_YARA_PATH, source_file=staging_yar, user="clamav", group="clamav", mode=0o644)
    log(f"Atomically updated {ACTIVE_YARA_PATH}.")

    # 4. Reload clamd
    log("Triggering zero-downtime clamd reload via clamdscan --reload...")
    res = subprocess.run(["clamdscan", "--reload"], capture_output=True, text=True)
    if res.returncode != 0:
        log(f"Warning: clamdscan --reload exited {res.returncode}: {res.stderr}")

    # 5. Wait for reload line 'Database correctly reloaded' with timeout and log rotation handling
    log(f"Waiting for clamd reload and 'Database correctly reloaded' in log (timeout: {reload_timeout:.1f}s)...")
    start_wait = time.time()
    reload_seen = False
    sig_count_logged = None
    ping_ok = False

    while (time.time() - start_wait) < reload_timeout:
        time.sleep(1.0)
        if log_file and os.path.exists(log_file):
            try:
                cur_size = os.path.getsize(log_file)
                if cur_size < log_pos_before:
                    log(f"Log rotation detected on '{log_file}' ({cur_size} < {log_pos_before}). Resetting read offset to 0.")
                    log_pos_before = 0
                with open(log_file, "r", encoding="utf-8", errors="replace") as lf:
                    lf.seek(log_pos_before)
                    chunk = lf.read()
                    if "Database correctly reloaded" in chunk:
                        reload_seen = True
                        matches = re.findall(r"Database correctly reloaded \((\d+) signatures\)", chunk)
                        if matches:
                            sig_count_logged = int(matches[-1])
            except Exception as e:
                log(f"Warning reading clamd log: {e}")

        if check_clamd_ping():
            ping_ok = True
        else:
            ping_ok = False

        if log_file and os.path.exists(log_file):
            if reload_seen and ping_ok:
                log(f"Reload confirmed in log: {sig_count_logged or 'unknown'} signatures, PING -> PONG OK.")
                break
        else:
            if ping_ok and (time.time() - start_wait) >= 2.0:
                log("PING -> PONG OK (no clamd log file specified/found).")
                break

    health_ok = ping_ok and (reload_seen if (log_file and os.path.exists(log_file)) else True)
    if not health_ok:
        log("CRITICAL: Clamd health check failed post-reload! Initiating emergency rollback...")
        if os.path.exists(BACKUP_YARA_PATH):
            safe_atomic_write(ACTIVE_YARA_PATH, source_file=BACKUP_YARA_PATH, user="clamav", group="clamav", mode=0o644)
            subprocess.run(["clamdscan", "--reload"], capture_output=True, text=True)
            log("Restored previous ruleset from backup.")
        # Rollback branch does NOT modify CURRENT_TAG_FILE (remains old verified tag)
        raise RuntimeError("Post-reload health check failed! Reverted to backup.")

    # 6. Update current tag ONLY after health check succeeds
    os.makedirs(os.path.dirname(CURRENT_TAG_FILE), exist_ok=True)
    with open(CURRENT_TAG_FILE, "w", encoding="utf-8") as f:
        f.write(new_tag + "\n")
    log(f"Successfully recorded new release tag: '{new_tag}' in {CURRENT_TAG_FILE}")

    final_hash = compute_sha256(ACTIVE_YARA_PATH)
    log("=" * 60)
    log(f"SUCCESS: YARA-Forge Core update completed successfully!")
    log(f"  Release Tag:    {new_tag}")
    log(f"  Accepted Rules: {accepted_count:,}")
    log(f"  Active File:    {ACTIVE_YARA_PATH}")
    log(f"  SHA256:         {final_hash}")
    log(f"  clamd Status:   PONG (127.0.0.1:3310 connected)")
    log("=" * 60)


def run_update(args):
    log("Starting Zonendpoint YARA-Forge Core updater...")
    current_tag = get_current_tag()
    log(f"Current installed release tag: '{current_tag}'")

    # Step 1: Query latest release
    log(f"Querying latest release from {REPO_API_URL}...")
    release = fetch_latest_release()
    latest_tag = release.get("tag_name", "").strip()
    if not latest_tag:
        raise RuntimeError("GitHub release response did not contain a valid 'tag_name'")

    log(f"Latest upstream release tag: '{latest_tag}'")

    if not args.force and current_tag == latest_tag:
        log(f"Installed release '{current_tag}' is already up-to-date with upstream. No update needed. Halting.")
        sys.exit(0)

    if args.check_only:
        log(f"New release '{latest_tag}' is available. (--check-only set, halting).")
        sys.exit(0)

    # Step 2: Download core ruleset
    core_asset = None
    for a in release.get("assets", []):
        if a.get("name") == "yara-forge-rules-core.zip":
            core_asset = a
            break

    if not core_asset:
        raise RuntimeError(f"Asset 'yara-forge-rules-core.zip' not found in release '{latest_tag}'")

    dl_url = core_asset["browser_download_url"]
    expected_size = core_asset.get("size")
    expected_checksum = find_checksum_in_release(release, "yara-forge-rules-core.zip")

    os.makedirs(WORK_DIR, exist_ok=True)
    zip_dest = os.path.join(WORK_DIR, f"yara-forge-rules-core-{latest_tag}.zip")
    download_file(dl_url, zip_dest)

    # Verify checksum if available
    download_sha256 = compute_sha256(zip_dest)
    log(f"Downloaded zip SHA256: {download_sha256}")
    if expected_checksum:
        log(f"Verifying against upstream checksum: {expected_checksum}...")
        if download_sha256.lower() != expected_checksum.lower():
            raise RuntimeError(
                f"Checksum mismatch for {zip_dest}! Expected {expected_checksum}, got {download_sha256}"
            )
        log("Checksum verification PASS.")
    else:
        log("No upstream checksum provided for core zip; recorded local SHA256.")

    # Extract core rules
    extract_dir = os.path.join(WORK_DIR, f"extracted-{latest_tag}")
    os.makedirs(extract_dir, exist_ok=True)
    raw_yar = extract_core_rules(zip_dest, extract_dir)

    # Filter with filter_yara_clamav.py
    staging_dir = os.path.join(WORK_DIR, f"staging-{latest_tag}")
    os.makedirs(staging_dir, exist_ok=True)
    staging_yar = os.path.join(staging_dir, "yara-forge-core.yar")
    staging_report = os.path.join(staging_dir, "filter-report.json")

    log(f"Filtering rules via {FILTER_SCRIPT_PATH} (excluding SUSP_ heuristics & running validation loop)...")
    filter_cmd = [
        sys.executable,
        FILTER_SCRIPT_PATH,
        "-i", raw_yar,
        "-o", staging_yar,
        "-r", staging_report,
        "--validate"
    ]
    proc = subprocess.run(filter_cmd, capture_output=True, text=True)
    if proc.returncode != 0:
        raise RuntimeError(f"Filter script failed with code {proc.returncode}:\n{proc.stdout}\n{proc.stderr}")

    with open(staging_report, "r", encoding="utf-8") as f:
        report_data = json.load(f)

    accepted_count = report_data.get("accepted_count", 0)
    rejected_count = report_data.get("rejected_count", 0)
    log(f"Filter completed: {accepted_count:,} rules accepted, {rejected_count:,} rejected.")

    # Extra verification: ensure NO accepted rules have SUSP_
    for name in report_data.get("accepted_rule_names", []):
        if "SUSP_" in name:
            raise RuntimeError(f"CRITICAL: Found SUSP_ heuristic rule '{name}' in accepted ruleset!")
    log("Verified: 0 SUSP_ heuristic rules in accepted ruleset.")

    # Step 3: Pre-flight on temporary DB
    verify_on_temp_db(staging_yar, accepted_count)

    # Step 4: Atomic deploy & reload
    if args.dry_run:
        log("DRY-RUN completed successfully. Active database was NOT modified.")
        sys.exit(0)

    deploy_and_reload(
        staging_yar,
        latest_tag,
        accepted_count,
        log_file=args.log_file,
        reload_timeout=args.reload_timeout,
    )

    # Cleanup work directory
    try:
        shutil.rmtree(extract_dir, ignore_errors=True)
        shutil.rmtree(staging_dir, ignore_errors=True)
        if os.path.exists(zip_dest):
            os.remove(zip_dest)
        log("Cleaned up temporary work files.")
    except Exception as e:
        log(f"Warning: error cleaning work files: {e}")
    return 0


def main():
    parser = argparse.ArgumentParser(description="Zonendpoint YARA-Forge Core Automated Daily Updater")
    parser.add_argument("--force", action="store_true", help="Force update even if tag matches current")
    parser.add_argument("--check-only", action="store_true", help="Check for updates without downloading")
    parser.add_argument("--dry-run", action="store_true", help="Download and test, but do not replace active DB or reload clamd")
    parser.add_argument("--hold-file", default=DEFAULT_HOLD_FILE, help=f"Hold file path to skip update when rollback active (default: {DEFAULT_HOLD_FILE})")
    parser.add_argument("--log-file", default=CLAMD_LOG_FILE, help=f"Path to clamd log file (default: {CLAMD_LOG_FILE})")
    parser.add_argument("--reload-timeout", type=float, default=120.0, help="Timeout in seconds for clamd reload (default: 120.0)")
    parser.add_argument("--lock-file", default=DEFAULT_LOCK_FILE, help=f"Shared deploy lock file path (default: {DEFAULT_LOCK_FILE})")
    args = parser.parse_args()

    # L-3: Check for hold file created by rollback
    hold_file = getattr(args, "hold_file", DEFAULT_HOLD_FILE)
    if os.path.isfile(hold_file):
        log(f"Hold file '{hold_file}' exists (created by rollback). Skipping YARA update per Zon policy. Remove file to resume automatic updates.")
        return 0

    # L-6: Shared flock to prevent concurrent executions with abusech/rollback/clamd deploy
    lock_file = getattr(args, "lock_file", DEFAULT_LOCK_FILE)
    try:
        os.makedirs(os.path.dirname(lock_file), exist_ok=True)
    except Exception as e:
        log(f"Could not create parent directory for lock file '{lock_file}': {e}. Aborting.")
        return 1

    lock_fd = None
    try:
        lock_fd = open(lock_file, "a+")
        try:
            import fcntl
            fcntl.flock(lock_fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except (BlockingIOError, OSError) as e:
            log(f"Another clamd deploy/update process is currently running (lock '{lock_file}' held: {e}). Skipping turn.")
            try:
                lock_fd.close()
            except Exception:
                pass
            return 1
    except Exception as e:
        log(f"Could not establish lock on '{lock_file}': {e}. Aborting.")
        if lock_fd:
            try:
                lock_fd.close()
            except Exception:
                pass
        return 1

    try:
        return run_update(args)
    finally:
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


if __name__ == "__main__":
    try:
        ret = main()
        if ret is not None and ret != 0:
            sys.exit(ret)
    except Exception as err:
        log(f"FATAL ERROR: {err}")
        sys.exit(1)
