#!/usr/bin/env python3
"""
Zonendpoint Automated Sanesecurity Feed Updater (Task V-68)

Automates updates for Sanesecurity ClamAV signature databases:
1. Enforces strict allowlist of Low False-Positive databases (excluding placeholders).
2. Verifies GPG signatures using isolated Sanesecurity keyring with verified fingerprint.
3. Performs conditional downloads (ETag / If-Modified-Since), size limits, and timeout controls.
4. Pre-flights updated databases via isolated clamscan before atomic deployment.
5. Integrates with shared deploy lock (/run/lock/zonendpoint-clamd-deploy.lock) and
   shared reload interval policy (/opt/zonendpoint/reload-state.json).
6. Verifies post-reload clamd health, handles log rotation, and triggers emergency rollback + HOLD on failure.
"""

import os
import sys
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
import urllib.parse
import pathlib
import fcntl
from typing import Optional, Tuple, Dict, Any, List

ALLOWLIST_FILES = [
    "rogue.hdb",
    "malwarehash.hsb",
    "hackingteam.hsb",
    "foxhole_generic.cdb",
    "foxhole_filename.cdb",
    "bofhland_malware_attach.hdb",
    "porcupine.hsb",
    "sigwhitelist.ign2",
    "sanesecurity.ftm",
]

DEFAULT_MIRROR = "https://mirror.seichter.de/sanesecurity/"
PUBKEY_URL = "https://sanesecurity.com/publickey.gpg"
EXPECTED_FINGERPRINT = "890D413C9E6EFD809D0EE892D6F4C53697CD8EE6"

DEFAULT_STATE_DIR = "/opt/zonendpoint/sanesecurity"
DEFAULT_GNUPG_HOME = "/opt/zonendpoint/sanesecurity/gnupg"
DEFAULT_STATE_FILE = "/opt/zonendpoint/sanesecurity/state.json"
DEFAULT_LAST_RUN_FILE = "/opt/zonendpoint/sanesecurity/last-run.json"
DEFAULT_HOLD_FILE = "/opt/zonendpoint/HOLD-sanesecurity"
DEFAULT_LOCK_FILE = os.environ.get("DEPLOY_LOCK_FILE", "/run/lock/zonendpoint-clamd-deploy.lock")
DEFAULT_RELOAD_STATE_FILE = os.environ.get("RELOAD_STATE_FILE", "/opt/zonendpoint/reload-state.json")
DEFAULT_ACTIVE_DIR = "/var/lib/clamav"
DEFAULT_BACKUP_DIR = "/var/backups/clamav-sanesecurity"
DEFAULT_CLAMD_LOG = "/var/log/clamav/clamav.log"

MAX_FILE_BYTES = 20 * 1024 * 1024  # 20 MiB per database file
MAX_PUBKEY_BYTES = 64 * 1024  # 64 KiB for GPG public key (Q-49)
DEFAULT_TIMEOUT = 30.0
DEFAULT_MIN_CHECK_INTERVAL_HOURS = 2.0
DEFAULT_MIN_RELOAD_INTERVAL_HOURS = 6.0
USER_AGENT = "Zonendpoint-Sanesecurity-Updater/1.0 (+https://github.com/quockhanh2376/Zonendpoint)"


def log(msg: str):
    timestamp = time.strftime("%Y-%m-%d %H:%M:%S")
    print(f"[{timestamp}] {msg}", flush=True)


class StrictRedirectHandler(urllib.request.HTTPRedirectHandler):
    """
    Q-49: Rejects redirects that downgrade to non-HTTPS or point to an unauthorized host.
    """
    def __init__(self, allowed_hosts: List[str]):
        super().__init__()
        self.allowed_hosts = [h.lower() for h in allowed_hosts]

    def redirect_request(self, req, fp, code, msg, headers, newurl):
        parsed = urllib.parse.urlparse(newurl)
        is_test_env = os.environ.get("ZONENDPOINT_TEST_ENV") == "1"
        is_local_mock = is_test_env and (parsed.hostname in ("127.0.0.1", "localhost"))
        if not is_local_mock and parsed.scheme.lower() != "https":
            raise ValueError(f"Insecure redirect rejected (must use HTTPS): '{newurl}'")

        if parsed.netloc.lower() not in self.allowed_hosts:
            raise ValueError(
                f"Cross-host redirect rejected: '{newurl}' (target host '{parsed.netloc}' not in allowed {self.allowed_hosts})"
            )
        return super().redirect_request(req, fp, code, msg, headers, newurl)


def build_secure_opener(allowed_hosts: List[str]) -> urllib.request.OpenerDirector:
    """Creates a urllib OpenerDirector enforcing StrictRedirectHandler."""
    return urllib.request.build_opener(StrictRedirectHandler(allowed_hosts))


def _place_hold(hold_file: str):
    """Atomically places HOLD file (Task L-3 / Q-49)."""
    pathlib.Path(hold_file).parent.mkdir(parents=True, exist_ok=True)
    with open(hold_file, "w", encoding="utf-8") as f:
        f.write(time.strftime("%Y-%m-%dT%H:%M:%SZ\n"))
    log(f"Placed HOLD file: {hold_file}")


def check_safety_guard(host: Optional[str] = None, port: Optional[int] = None, clamdscan_bin: Optional[str] = None):
    """Critical safety check preventing tests from touching live clamd."""
    if os.environ.get("ZONENDPOINT_TEST_ENV") == "1":
        if port == 3310:
            raise AssertionError("SAFETY VIOLATION: Test targeted real clamd port 3310!")
        if clamdscan_bin == "clamdscan":
            raise AssertionError("SAFETY VIOLATION: Test targeted real clamdscan binary!")


def validate_mirror_url(url: str) -> str:
    """Enforces HTTPS for mirror unless local mock in test environment."""
    if not url:
        raise ValueError("Mirror URL cannot be empty")
    is_test_env = os.environ.get("ZONENDPOINT_TEST_ENV") == "1"
    is_local_mock = is_test_env and (url.startswith("http://127.0.0.1") or url.startswith("http://localhost"))
    if not is_local_mock:
        if not url.startswith("https://"):
            raise ValueError(f"Insecure mirror URL rejected (must use HTTPS): '{url}'")
    if not url.endswith("/"):
        url += "/"
    return url


def check_hold_file(hold_file: str = DEFAULT_HOLD_FILE) -> bool:
    """Checks if updater is paused via HOLD file (Task L-3)."""
    p = pathlib.Path(hold_file)
    if p.is_file():
        log(f"Hold file '{hold_file}' present. Skipping Sanesecurity update run.")
        return True
    return False


def check_reload_interval(state_file: str = DEFAULT_RELOAD_STATE_FILE, min_interval_hours: float = DEFAULT_MIN_RELOAD_INTERVAL_HOURS) -> Tuple[bool, float]:
    """Checks if minimum reload interval has elapsed (Task L-8)."""
    p = pathlib.Path(state_file)
    if not p.is_file():
        return True, float("inf")
    try:
        data = json.loads(p.read_text(encoding="utf-8"))
        last_ts = float(data.get("last_reload_timestamp", 0))
        elapsed_hours = (time.time() - last_ts) / 3600.0
        return elapsed_hours >= min_interval_hours, elapsed_hours
    except Exception:
        return True, float("inf")


def record_reload_state(state_file: str = DEFAULT_RELOAD_STATE_FILE, caller: str = "sanesecurity", signatures: Optional[int] = None, forced: bool = False):
    """Atomically records reload timestamp in shared reload state (Task L-8)."""
    p = pathlib.Path(state_file)
    p.parent.mkdir(parents=True, exist_ok=True)
    rec = {
        "last_reload_timestamp": time.time(),
        "caller": caller,
        "signatures": signatures,
        "forced": forced,
    }
    target_dir = str(p.parent)
    fd, tmp_path = tempfile.mkstemp(dir=target_dir, prefix=".reload-state.tmp-")
    try:
        with open(fd, "w", encoding="utf-8") as f:
            json.dump(rec, f, indent=2)
            f.flush()
            os.fsync(f.fileno())
    finally:
        pass
    os.replace(tmp_path, state_file)


def check_min_check_interval(state_file: str, min_interval_hours: float = DEFAULT_MIN_CHECK_INTERVAL_HOURS) -> Tuple[bool, float]:
    """Ensures updater does not poll mirrors too frequently even if timer fires early."""
    p = pathlib.Path(state_file)
    if not p.is_file():
        return True, float("inf")
    try:
        data = json.loads(p.read_text(encoding="utf-8"))
        last_check = float(data.get("last_check_timestamp", 0))
        elapsed_hours = (time.time() - last_check) / 3600.0
        return elapsed_hours >= min_interval_hours, elapsed_hours
    except Exception:
        return True, float("inf")


def load_state(state_file: str) -> Dict[str, Any]:
    p = pathlib.Path(state_file)
    if p.is_file():
        try:
            return json.loads(p.read_text(encoding="utf-8"))
        except Exception as e:
            log(f"Warning: could not parse state file {state_file}: {e}")
    return {"last_check_timestamp": 0.0, "files": {}}


def save_state(state_file: str, state_data: Dict[str, Any]):
    p = pathlib.Path(state_file)
    p.parent.mkdir(parents=True, exist_ok=True)
    target_dir = str(p.parent)
    fd, tmp_path = tempfile.mkstemp(dir=target_dir, prefix=".state.tmp-")
    try:
        with open(fd, "w", encoding="utf-8") as f:
            json.dump(state_data, f, indent=2)
            f.flush()
            os.fsync(f.fileno())
    finally:
        pass
    os.replace(tmp_path, state_file)


def write_last_run(last_run_file: str, data: Dict[str, Any]):
    p = pathlib.Path(last_run_file)
    p.parent.mkdir(parents=True, exist_ok=True)
    target_dir = str(p.parent)
    fd, tmp_path = tempfile.mkstemp(dir=target_dir, prefix=".last-run.tmp-")
    try:
        with open(fd, "w", encoding="utf-8") as f:
            json.dump(data, f, indent=2)
            f.flush()
            os.fsync(f.fileno())
    finally:
        pass
    os.replace(tmp_path, last_run_file)


def get_keyring_fingerprints(gnupg_home: str) -> List[str]:
    """Returns list of primary key fingerprints currently present in gnupg_home."""
    cmd = ["gpg", "--homedir", gnupg_home, "--batch", "--with-colons", "--list-keys"]
    res = subprocess.run(cmd, capture_output=True, text=True)
    if res.returncode != 0:
        return []
    fprs: List[str] = []
    is_pub = False
    for line in res.stdout.splitlines():
        if line.startswith("pub:"):
            is_pub = True
        elif is_pub and line.startswith("fpr:"):
            parts = line.split(":")
            if len(parts) >= 10:
                fprs.append(parts[9].strip().upper())
            is_pub = False
    return fprs


def setup_gnupg_keyring(
    gnupg_home: str,
    pubkey_source: Optional[str] = None,
    allowed_hosts: Optional[List[str]] = None,
) -> bool:
    """
    Q-49: Sets up isolated GPG keyring.
    Enforces that:
    1. Keyring must contain ONLY the official Sanesecurity key (EXPECTED_FINGERPRINT).
       Any foreign/unauthorized key present in the keyring causes immediate termination.
    2. Public key download size is strictly bounded by MAX_PUBKEY_BYTES (64 KiB).
    3. Download cannot follow redirects to HTTP or unauthorized hosts.
    """
    os.makedirs(gnupg_home, mode=0o700, exist_ok=True)
    try:
        os.chmod(gnupg_home, 0o700)
    except OSError:
        pass

    # Check existing keys in keyring
    existing_fprs = get_keyring_fingerprints(gnupg_home)
    if existing_fprs:
        unauthorized = [f for f in existing_fprs if f != EXPECTED_FINGERPRINT]
        if unauthorized:
            raise RuntimeError(
                f"GPG keyring '{gnupg_home}' contains unauthorized key(s): {unauthorized}! "
                f"Keyring must ONLY contain official key {EXPECTED_FINGERPRINT}."
            )
        if EXPECTED_FINGERPRINT in existing_fprs:
            return True

    # Import public key
    log(f"Importing Sanesecurity public key into isolated keyring {gnupg_home}...")
    pubkey_data = b""
    if pubkey_source and os.path.exists(pubkey_source):
        sz = os.path.getsize(pubkey_source)
        if sz > MAX_PUBKEY_BYTES:
            raise ValueError(f"Local public key size {sz} exceeds limit {MAX_PUBKEY_BYTES} bytes")
        with open(pubkey_source, "rb") as pf:
            pubkey_data = pf.read(MAX_PUBKEY_BYTES + 1)
            if len(pubkey_data) > MAX_PUBKEY_BYTES:
                raise ValueError(f"Local public key size exceeds limit {MAX_PUBKEY_BYTES} bytes")
    else:
        url = pubkey_source or PUBKEY_URL
        log(f"Fetching GPG public key from {url}...")
        parsed_url = urllib.parse.urlparse(url)
        hosts = allowed_hosts or [parsed_url.netloc]
        opener = build_secure_opener(hosts)
        req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
        with opener.open(req, timeout=DEFAULT_TIMEOUT) as resp:
            final_url = resp.geturl()
            parsed_final = urllib.parse.urlparse(final_url)
            is_test_env = os.environ.get("ZONENDPOINT_TEST_ENV") == "1"
            is_local_mock = is_test_env and (parsed_final.hostname in ("127.0.0.1", "localhost"))
            if not is_local_mock and parsed_final.scheme.lower() != "https":
                raise ValueError(f"Insecure public key final URL (must use HTTPS): '{final_url}'")
            if parsed_final.netloc.lower() not in [h.lower() for h in hosts]:
                raise ValueError(f"Cross-host public key final URL rejected: '{final_url}' (not in {hosts})")

            cl = resp.headers.get("Content-Length")
            if cl and int(cl) > MAX_PUBKEY_BYTES:
                raise ValueError(f"Public key Content-Length {cl} exceeds limit {MAX_PUBKEY_BYTES} bytes")

            while True:
                chunk = resp.read(4096)
                if not chunk:
                    break
                pubkey_data += chunk
                if len(pubkey_data) > MAX_PUBKEY_BYTES:
                    raise ValueError(f"Public key size exceeded limit {MAX_PUBKEY_BYTES} bytes")

    with tempfile.NamedTemporaryFile("wb", delete=False) as tf:
        tf.write(pubkey_data)
        tmp_key = tf.name

    try:
        # Pre-check fingerprint before importing
        cmd_show = ["gpg", "--batch", "--with-colons", "--show-keys", tmp_key]
        res_show = subprocess.run(cmd_show, capture_output=True, text=True)
        found_fpr = None
        for line in res_show.stdout.splitlines():
            if line.startswith("fpr:"):
                parts = line.split(":")
                if len(parts) >= 10:
                    found_fpr = parts[9].strip().upper()
                    break

        if found_fpr != EXPECTED_FINGERPRINT:
            raise RuntimeError(
                f"GPG key fingerprint mismatch! Expected {EXPECTED_FINGERPRINT}, got '{found_fpr}'"
            )

        cmd_import = ["gpg", "--homedir", gnupg_home, "--batch", "--import", tmp_key]
        res_import = subprocess.run(cmd_import, capture_output=True, text=True)
        if res_import.returncode != 0:
            raise RuntimeError(f"GPG key import failed: {res_import.stderr}")

        # Post-check: Keyring must contain ONLY EXPECTED_FINGERPRINT
        post_fprs = get_keyring_fingerprints(gnupg_home)
        unauthorized_post = [f for f in post_fprs if f != EXPECTED_FINGERPRINT]
        if unauthorized_post or EXPECTED_FINGERPRINT not in post_fprs:
            raise RuntimeError(
                f"GPG keyring verification failed after import! Found keys: {post_fprs}, expected ONLY {EXPECTED_FINGERPRINT}"
            )

        log(f"Successfully imported and verified Sanesecurity public key (fingerprint: {EXPECTED_FINGERPRINT}).")
        return True
    finally:
        try:
            os.remove(tmp_key)
        except OSError:
            pass


def verify_gpg_signature(gnupg_home: str, file_path: str, sig_path: str) -> bool:
    """
    Q-49: Verifies detached GPG signature using --status-fd 1.
    Requires that the signer fingerprint (VALIDSIG) matches EXPECTED_FINGERPRINT exactly.
    """
    cmd = ["gpg", "--homedir", gnupg_home, "--batch", "--status-fd", "1", "--verify", sig_path, file_path]
    res = subprocess.run(cmd, capture_output=True, text=True)
    if res.returncode != 0:
        log(f"GPG verification exit {res.returncode} for {os.path.basename(file_path)}: {res.stderr.strip()}")
        return False

    valid_sig = False
    signer_fpr = None
    for line in res.stdout.splitlines():
        if line.startswith("[GNUPG:] VALIDSIG "):
            parts = line.split()
            if len(parts) >= 3:
                signer_fpr = parts[2].strip().upper()
                primary_fpr = parts[11].strip().upper() if len(parts) >= 12 else ""
                if signer_fpr == EXPECTED_FINGERPRINT or primary_fpr == EXPECTED_FINGERPRINT:
                    valid_sig = True
                    break

    if valid_sig:
        return True

    log(
        f"GPG verification FAILED for {os.path.basename(file_path)}: "
        f"signer fingerprint '{signer_fpr}' does not match expected {EXPECTED_FINGERPRINT}"
    )
    return False


def compute_sha256(filepath: str) -> str:
    h = hashlib.sha256()
    with open(filepath, "rb") as f:
        while True:
            chunk = f.read(65536)
            if not chunk:
                break
            h.update(chunk)
    return h.hexdigest()


def safe_atomic_write(
    target_path: str,
    source_file: str,
    user: str = "clamav",
    group: str = "clamav",
    mode: int = 0o644,
    uid: Optional[int] = None,
    gid: Optional[int] = None,
):
    """Safely copies source_file to target_path using mkstemp, fchown, fchmod, fsync, and replace (Task L-11)."""
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
            target_uid = uid
            target_gid = gid
            if target_uid is None or target_gid is None:
                try:
                    import pwd, grp
                    target_uid = pwd.getpwnam(user).pw_uid
                    target_gid = grp.getgrnam(group).gr_gid
                except Exception:
                    target_uid = None
                    target_gid = None
            if target_uid is not None and target_gid is not None:
                try:
                    os.fchown(fd, target_uid, target_gid)
                except OSError:
                    pass
        try:
            os.fchmod(fd, mode)
        except OSError:
            pass
        os.fsync(fd)
    finally:
        os.close(fd)
    os.replace(tmp_path, target_path)


def download_conditional(
    url: str,
    dest_path: str,
    etag: Optional[str] = None,
    last_modified: Optional[str] = None,
    max_bytes: int = MAX_FILE_BYTES,
    timeout: float = DEFAULT_TIMEOUT,
    allowed_hosts: Optional[List[str]] = None,
    opener: Optional[urllib.request.OpenerDirector] = None,
) -> Tuple[bool, Optional[str], Optional[str]]:
    """
    Downloads file with If-None-Match / If-Modified-Since headers.
    Q-49: Enforces StrictRedirectHandler and validates final URL scheme and host.
    Returns: (is_modified, new_etag, new_last_modified).
    """
    headers = {"User-Agent": USER_AGENT}
    if etag:
        headers["If-None-Match"] = etag
    if last_modified:
        headers["If-Modified-Since"] = last_modified

    parsed_req = urllib.parse.urlparse(url)
    hosts = allowed_hosts or [parsed_req.netloc]
    if opener is None:
        opener = build_secure_opener(hosts)

    req = urllib.request.Request(url, headers=headers)
    try:
        with opener.open(req, timeout=timeout) as resp:
            final_url = resp.geturl()
            parsed_final = urllib.parse.urlparse(final_url)
            is_test_env = os.environ.get("ZONENDPOINT_TEST_ENV") == "1"
            is_local_mock = is_test_env and (parsed_final.hostname in ("127.0.0.1", "localhost"))
            if not is_local_mock and parsed_final.scheme.lower() != "https":
                raise ValueError(f"Insecure final URL (must use HTTPS): '{final_url}'")
            if parsed_final.netloc.lower() not in [h.lower() for h in hosts]:
                raise ValueError(f"Cross-host final URL rejected: '{final_url}' (not in {hosts})")

            new_etag = resp.headers.get("ETag")
            new_lm = resp.headers.get("Last-Modified")
            cl = resp.headers.get("Content-Length")
            if cl and int(cl) > max_bytes:
                raise ValueError(f"Content-Length {cl} exceeds limit {max_bytes} bytes for {url}")

            downloaded = 0
            with open(dest_path, "wb") as out_f:
                while True:
                    chunk = resp.read(65536)
                    if not chunk:
                        break
                    downloaded += len(chunk)
                    if downloaded > max_bytes:
                        raise ValueError(f"Downloaded bytes exceeded limit {max_bytes} for {url}")
                    out_f.write(chunk)
            return True, new_etag, new_lm
    except urllib.error.HTTPError as e:
        if e.code == 304:
            return False, etag, last_modified
        raise


def deploy_batch(
    staged_files: List[str],
    active_dir: str,
    backup_dir: str,
    hold_file: str,
    reload_state_file: str,
    last_run_file: str,
    clamdscan_bin: str = "clamdscan",
    clamd_host: str = "127.0.0.1",
    clamd_port: int = 3310,
    user: str = "clamav",
    group: str = "clamav",
    default_mode: int = 0o644,
) -> List[str]:
    """
    Q-49: Atomically deploys all staged files as a batch.
    If backup, installation, or writing fails on any file:
    1. Immediately creates HOLD file.
    2. Restores all files in the batch back to their previous state (preserving mode and owner).
    3. Removes any newly created files that did not previously exist.
    4. Triggers emergency reload.
    5. Writes last-run.json and raises RuntimeError.
    """
    os.makedirs(backup_dir, exist_ok=True)
    os.makedirs(active_dir, exist_ok=True)

    snapshot_manifest: Dict[str, Dict[str, Any]] = {}

    # Step 1: Pre-snapshot all target files in active_dir
    try:
        for staged_path in staged_files:
            fname = os.path.basename(staged_path)
            active_path = os.path.join(active_dir, fname)
            backup_path = os.path.join(backup_dir, f"{fname}.bak")

            if os.path.exists(active_path):
                st = os.stat(active_path)
                mode = st.st_mode & 0o777
                uid = st.st_uid
                gid = st.st_gid
                shutil.copyfile(active_path, backup_path)
                try:
                    os.chmod(backup_path, mode)
                    if os.name == "posix":
                        os.chown(backup_path, uid, gid)
                except OSError:
                    pass
                snapshot_manifest[fname] = {
                    "existed": True,
                    "backup_path": backup_path,
                    "mode": mode,
                    "uid": uid,
                    "gid": gid,
                }
                log(f"Backed up {fname} -> {backup_path} (mode={oct(mode)})")
            else:
                snapshot_manifest[fname] = {
                    "existed": False,
                }
    except Exception as e:
        log(f"ERROR during batch pre-backup phase: {e}. Setting HOLD.")
        _place_hold(hold_file)
        raise RuntimeError(f"Batch pre-backup failed: {e}")

    # Step 2: Deploy files one by one with rollback on error
    deployed_files: List[str] = []
    try:
        for staged_path in staged_files:
            fname = os.path.basename(staged_path)
            active_path = os.path.join(active_dir, fname)
            safe_atomic_write(
                target_path=active_path,
                source_file=staged_path,
                user=user,
                group=group,
                mode=default_mode,
            )
            deployed_files.append(fname)
            log(f"Atomically deployed {fname} -> {active_path}")
        return deployed_files
    except Exception as e:
        log(f"CRITICAL: Batch deployment failed at file '{fname}': {e}. Initiating batch rollback...")
        # 1. Place HOLD file first
        _place_hold(hold_file)

        # 2. Restore all files from snapshot
        for fn, info in snapshot_manifest.items():
            ap = os.path.join(active_dir, fn)
            if info["existed"]:
                bp = info["backup_path"]
                try:
                    safe_atomic_write(
                        target_path=ap,
                        source_file=bp,
                        mode=info["mode"],
                        uid=info.get("uid"),
                        gid=info.get("gid"),
                    )
                    log(f"Batch rollback restored {fn} from {bp} (mode={oct(info['mode'])})")
                except Exception as restore_err:
                    log(f"ERROR restoring {fn} during rollback: {restore_err}")
            else:
                if os.path.exists(ap):
                    try:
                        os.remove(ap)
                        log(f"Batch rollback removed unbacked {fn}")
                    except OSError as rm_err:
                        log(f"Warning removing {ap}: {rm_err}")

        # 3. Trigger emergency clamd reload
        try:
            subprocess.run([clamdscan_bin, "--reload"], capture_output=True, text=True)
            record_reload_state(reload_state_file, caller="sanesecurity_batch_rollback", forced=True)
        except Exception as clamd_err:
            log(f"Warning during batch rollback clamd reload: {clamd_err}")

        write_last_run(last_run_file, {
            "timestamp": time.time(),
            "status": "batch_deploy_failed",
            "error": str(e),
            "failed_file": fname,
            "rollback": "restored"
        })
        raise RuntimeError(f"Batch deployment failed at '{fname}' and was rolled back: {e}")


def run_clamscan(args: List[str], clamscan_bin: str = "clamscan", timeout: float = 120.0) -> subprocess.CompletedProcess:
    return subprocess.run([clamscan_bin] + args, capture_output=True, text=True, timeout=timeout)


def check_clamd_ping(host: str = "127.0.0.1", port: int = 3310, timeout: float = 5.0) -> bool:
    check_safety_guard(host=host, port=port)
    try:
        with socket.create_connection((host, port), timeout=timeout) as s:
            s.sendall(b"PING\n")
            resp = s.recv(1024).decode("utf-8", errors="replace").strip()
            return resp == "PONG"
    except Exception as e:
        log(f"Clamd PING check failed: {e}")
        return False


def preflight_test(staging_files: List[str], clamscan_bin: str = "clamscan") -> bool:
    """Preflight tests newly staged Sanesecurity files on temporary DB (Task L-9)."""
    log("=== Running Pre-flight Validation on Staging Databases (Task L-9) ===")
    with tempfile.TemporaryDirectory(prefix="zon-ss-preflight-") as temp_db:
        for f in staging_files:
            shutil.copy2(f, os.path.join(temp_db, os.path.basename(f)))

        with tempfile.NamedTemporaryFile("wb", delete=False) as empty_tf:
            empty_path = empty_tf.name

        try:
            log(f"Pre-flight loading {len(staging_files)} Sanesecurity databases via clamscan...")
            res = run_clamscan(["--no-summary", "-d", temp_db, empty_path], clamscan_bin=clamscan_bin)
            if res.returncode != 0:
                log(f"Pre-flight clamscan failed with exit code {res.returncode}:\n{res.stderr}\n{res.stdout}")
                return False

            for line in res.stderr.splitlines():
                if "failed to parse or load" in line or "LibClamAV Error" in line:
                    log(f"Pre-flight LibClamAV load error detected: {line}")
                    return False

            log("Pre-flight validation PASS (0 load errors).")
            return True
        finally:
            try:
                os.remove(empty_path)
            except OSError:
                pass


def reload_clamd_and_verify(
    log_file: str,
    clamdscan_bin: str,
    clamd_host: str,
    clamd_port: int,
    reload_timeout: float = 120.0,
) -> bool:
    """Reloads clamd and verifies 'Database correctly reloaded' in log and PING -> PONG."""
    check_safety_guard(host=clamd_host, port=clamd_port, clamdscan_bin=clamdscan_bin)

    log_pos_before = 0
    if log_file and os.path.exists(log_file):
        try:
            log_pos_before = os.path.getsize(log_file)
            log(f"clamd log initial offset: {log_pos_before} on '{log_file}'")
        except Exception as e:
            log(f"Warning: could not get clamd log size: {e}")

    log(f"Triggering zero-downtime clamd reload via {clamdscan_bin} --reload...")
    res = subprocess.run([clamdscan_bin, "--reload"], capture_output=True, text=True)
    if res.returncode != 0:
        log(f"Warning: {clamdscan_bin} --reload exited {res.returncode}: {res.stderr}")

    log(f"Waiting for clamd reload and 'Database correctly reloaded' in log (timeout: {reload_timeout:.1f}s)...")
    start_wait = time.time()
    reload_seen = False
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
            except Exception as e:
                log(f"Warning reading clamd log: {e}")

        if check_clamd_ping(host=clamd_host, port=clamd_port):
            ping_ok = True
        else:
            ping_ok = False

        if log_file and os.path.exists(log_file):
            if reload_seen and ping_ok:
                log("Reload confirmed in log: 'Database correctly reloaded', PING -> PONG OK.")
                return True
        else:
            if ping_ok and (time.time() - start_wait) >= 2.0:
                log("PING -> PONG OK (no clamd log file found/specified).")
                return True

    if ping_ok and not reload_seen and (log_file and os.path.exists(log_file)):
        log("L-5: 'Database correctly reloaded' not seen in log, but PING is OK. Marking as unverified (no rollback).")
        return True

    log("ERROR: Post-reload health check failed!")
    return False


def emergency_rollback(backup_dir: str, active_dir: str, hold_file: str, reload_state_file: str, clamdscan_bin: str, clamd_host: str, clamd_port: int):
    """Executes emergency rollback on post-reload failure and sets HOLD file."""
    log("CRITICAL: Executing emergency rollback and setting HOLD file...")
    _place_hold(hold_file)

    if os.path.exists(backup_dir):
        for fname in os.listdir(backup_dir):
            if fname.endswith(".bak"):
                orig_name = fname[:-4]
                src = os.path.join(backup_dir, fname)
                dst = os.path.join(active_dir, orig_name)
                try:
                    st = os.stat(src)
                    safe_atomic_write(
                        target_path=dst,
                        source_file=src,
                        mode=st.st_mode & 0o777,
                        uid=st.st_uid,
                        gid=st.st_gid,
                    )
                    log(f"Restored {orig_name} from backup (mode={oct(st.st_mode)}).")
                except Exception as e:
                    log(f"Error restoring {orig_name}: {e}")

    subprocess.run([clamdscan_bin, "--reload"], capture_output=True, text=True)
    record_reload_state(reload_state_file, caller="sanesecurity_emergency_rollback", forced=True)


def parse_args(argv: Optional[List[str]] = None):
    parser = argparse.ArgumentParser(description="Zonendpoint Sanesecurity Feed Updater")
    parser.add_argument("--mirror", default=DEFAULT_MIRROR, help=f"Sanesecurity mirror URL (default: {DEFAULT_MIRROR})")
    parser.add_argument("--public-key", default=None, help="Path or URL to Sanesecurity GPG public key")
    parser.add_argument("--state-dir", default=DEFAULT_STATE_DIR, help="Directory for state and gnupg keyring")
    parser.add_argument("--gnupg-home", default=DEFAULT_GNUPG_HOME, help="GPG homedir for verification")
    parser.add_argument("--state-file", default=DEFAULT_STATE_FILE, help="Path to state.json")
    parser.add_argument("--last-run-file", default=DEFAULT_LAST_RUN_FILE, help="Path to last-run.json")
    parser.add_argument("--hold-file", default=DEFAULT_HOLD_FILE, help="Path to HOLD-sanesecurity file")
    parser.add_argument("--lock-file", default=DEFAULT_LOCK_FILE, help="Path to deploy lock file")
    parser.add_argument("--reload-state-file", default=DEFAULT_RELOAD_STATE_FILE, help="Path to reload-state.json")
    parser.add_argument("--active-dir", default=DEFAULT_ACTIVE_DIR, help="Active ClamAV db directory")
    parser.add_argument("--backup-dir", default=DEFAULT_BACKUP_DIR, help="Backup directory for rollback")
    parser.add_argument("--log-file", default=DEFAULT_CLAMD_LOG, help="Path to clamd.log")
    parser.add_argument("--clamdscan-bin", default="clamdscan", help="Binary for clamdscan")
    parser.add_argument("--clamscan-bin", default="clamscan", help="Binary for clamscan preflight")
    parser.add_argument("--clamd-host", default="127.0.0.1", help="Host for clamd PING check")
    parser.add_argument("--clamd-port", type=int, default=3310, help="Port for clamd PING check")
    parser.add_argument("--min-check-interval", type=float, default=DEFAULT_MIN_CHECK_INTERVAL_HOURS, help="Min hours between check runs")
    parser.add_argument("--min-reload-interval", type=float, default=DEFAULT_MIN_RELOAD_INTERVAL_HOURS, help="Min hours between clamd reloads")
    parser.add_argument("--force-check", action="store_true", help="Bypass min check interval")
    parser.add_argument("--force-reload", action="store_true", help="Bypass min reload interval")
    parser.add_argument("--check-only", action="store_true", help="Check for updates without downloading or deploying")
    return parser.parse_args(argv)


def main():
    args = parse_args()
    check_safety_guard(host=args.clamd_host, port=args.clamd_port, clamdscan_bin=args.clamdscan_bin)

    # 1. HOLD file check
    if check_hold_file(args.hold_file):
        write_last_run(args.last_run_file, {
            "timestamp": time.time(),
            "status": "hold",
            "message": f"Hold file present at {args.hold_file}"
        })
        sys.exit(0)

    # 2. Acquire shared deploy lock
    os.makedirs(os.path.dirname(args.lock_file), exist_ok=True)
    lock_fd = None
    try:
        lock_fd = open(args.lock_file, "w")
        fcntl.flock(lock_fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except (IOError, OSError) as e:
        log(f"ERROR: Another clamd deploy/update process is currently running (lock '{args.lock_file}' held). Exiting code 3.")
        write_last_run(args.last_run_file, {
            "timestamp": time.time(),
            "status": "locked",
            "error": str(e)
        })
        sys.exit(3)

    try:
        # Validate mirror URL
        mirror_url = validate_mirror_url(args.mirror)
        mirror_host = urllib.parse.urlparse(mirror_url).netloc
        allowed_hosts = [mirror_host]
        if args.public_key:
            pk_host = urllib.parse.urlparse(args.public_key).netloc
            if pk_host and pk_host not in allowed_hosts:
                allowed_hosts.append(pk_host)
        else:
            def_pk_host = urllib.parse.urlparse(PUBKEY_URL).netloc
            if def_pk_host and def_pk_host not in allowed_hosts:
                allowed_hosts.append(def_pk_host)

        # 3. Min check interval
        if not args.force_check and not args.check_only:
            can_check, elapsed = check_min_check_interval(args.state_file, args.min_check_interval)
            if not can_check:
                log(f"Minimum check interval of {args.min_check_interval:.1f}h not elapsed ({elapsed:.2f}h elapsed). Skipping.")
                write_last_run(args.last_run_file, {
                    "timestamp": time.time(),
                    "status": "check_deferred",
                    "elapsed_hours": elapsed
                })
                sys.exit(0)

        # 4. Setup GPG keyring and verify fingerprint (Q-49: max size 64KiB & strict host check)
        setup_gnupg_keyring(args.gnupg_home, pubkey_source=args.public_key, allowed_hosts=allowed_hosts)

        state = load_state(args.state_file)
        files_state = state.get("files", {})

        log(f"Checking Sanesecurity updates against mirror: {mirror_url}")
        if args.check_only:
            log("Running in check-only mode. Exiting.")
            sys.exit(0)

        # 5. Download and verify in temporary staging directory
        staged_files: List[str] = []
        updated_file_names: List[str] = []
        unchanged_file_names: List[str] = []
        failed_gpg_files: List[str] = []

        with tempfile.TemporaryDirectory(prefix="zon-ss-staging-") as staging_dir:
            for fname in ALLOWLIST_FILES:
                file_url = mirror_url + fname
                sig_url = mirror_url + fname + ".sig"

                dest_file = os.path.join(staging_dir, fname)
                dest_sig = os.path.join(staging_dir, fname + ".sig")

                fstate = files_state.get(fname, {})
                prev_etag = fstate.get("etag")
                prev_lm = fstate.get("last_modified")

                try:
                    is_mod, new_etag, new_lm = download_conditional(
                        file_url, dest_file, etag=prev_etag, last_modified=prev_lm,
                        allowed_hosts=[mirror_host]
                    )
                    if not is_mod:
                        unchanged_file_names.append(fname)
                        continue

                    # File modified upstream, download corresponding .sig
                    download_conditional(
                        sig_url, dest_sig, max_bytes=1024 * 1024,
                        allowed_hosts=[mirror_host]
                    )

                    # GPG verification (Q-49: signer fingerprint must match EXPECTED_FINGERPRINT via --status-fd 1)
                    if not verify_gpg_signature(args.gnupg_home, dest_file, dest_sig):
                        log(f"WARNING: GPG verification failed for {fname}. Rejecting update, retaining existing copy.")
                        failed_gpg_files.append(fname)
                        continue

                    staged_files.append(dest_file)
                    updated_file_names.append(fname)
                    files_state[fname] = {
                        "etag": new_etag,
                        "last_modified": new_lm,
                        "sha256": compute_sha256(dest_file),
                        "size": os.path.getsize(dest_file),
                        "verified_at": time.time(),
                    }
                except Exception as e:
                    log(f"Error fetching/verifying {fname}: {e}")
                    failed_gpg_files.append(fname)

            log(f"Summary: {len(updated_file_names)} updated, {len(unchanged_file_names)} unchanged, {len(failed_gpg_files)} rejected/failed.")

            # If nothing was updated, record state and finish
            if not updated_file_names:
                state["last_check_timestamp"] = time.time()
                save_state(args.state_file, state)
                write_last_run(args.last_run_file, {
                    "timestamp": time.time(),
                    "status": "no_updates",
                    "updated": [],
                    "unchanged": unchanged_file_names,
                    "failed": failed_gpg_files
                })
                log("No updates to deploy. Finished.")
                sys.exit(0)

            # 6. Preflight test on staged files
            if not preflight_test(staged_files, clamscan_bin=args.clamscan_bin):
                log("ERROR: Pre-flight validation failed. Aborting deployment.")
                write_last_run(args.last_run_file, {
                    "timestamp": time.time(),
                    "status": "preflight_failed",
                    "updated": updated_file_names,
                    "error": "Pre-flight validation failed"
                })
                sys.exit(1)

            # 7. Atomic batch deployment & backup (Q-49: all-or-nothing rollback + HOLD on error)
            deployed_files = deploy_batch(
                staged_files=staged_files,
                active_dir=args.active_dir,
                backup_dir=args.backup_dir,
                hold_file=args.hold_file,
                reload_state_file=args.reload_state_file,
                last_run_file=args.last_run_file,
                clamdscan_bin=args.clamdscan_bin,
                clamd_host=args.clamd_host,
                clamd_port=args.clamd_port,
            )

            # 8. Reload check and execution
            reload_deferred = False
            if not args.force_reload:
                can_reload, elapsed_h = check_reload_interval(args.reload_state_file, args.min_reload_interval)
                if not can_reload:
                    log(f"L-8: Deferring clamd reload: only {elapsed_h:.2f}h elapsed (min interval: {args.min_reload_interval:.1f}h).")
                    reload_deferred = True

            if not reload_deferred:
                ok = reload_clamd_and_verify(
                    log_file=args.log_file,
                    clamdscan_bin=args.clamdscan_bin,
                    clamd_host=args.clamd_host,
                    clamd_port=args.clamd_port,
                )
                if not ok:
                    emergency_rollback(
                        backup_dir=args.backup_dir,
                        active_dir=args.active_dir,
                        hold_file=args.hold_file,
                        reload_state_file=args.reload_state_file,
                        clamdscan_bin=args.clamdscan_bin,
                        clamd_host=args.clamd_host,
                        clamd_port=args.clamd_port,
                    )
                    write_last_run(args.last_run_file, {
                        "timestamp": time.time(),
                        "status": "rollback_triggered",
                        "error": "Clamd post-reload health check failed"
                    })
                    sys.exit(2)

                record_reload_state(args.reload_state_file, caller="sanesecurity", forced=args.force_reload)

            state["last_check_timestamp"] = time.time()
            state["files"] = files_state
            save_state(args.state_file, state)

            status_str = "reload_deferred" if reload_deferred else "ok"
            write_last_run(args.last_run_file, {
                "timestamp": time.time(),
                "status": status_str,
                "updated": updated_file_names,
                "unchanged": unchanged_file_names,
                "failed": failed_gpg_files,
                "reload_deferred": reload_deferred
            })
            log(f"SUCCESS: Sanesecurity update completed with status '{status_str}'.")

    finally:
        if lock_fd:
            try:
                fcntl.flock(lock_fd, fcntl.LOCK_UN)
                lock_fd.close()
            except Exception:
                pass


if __name__ == "__main__":
    main()
