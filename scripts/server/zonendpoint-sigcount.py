#!/usr/bin/env python3
"""
zonendpoint-sigcount.py: ClamAV signature count extractor for ZonEndpoint.

Reads the latest database reload or load line from the clamav-daemon journal,
extracts the active signature count, timestamp, and engine version,
and writes an atomic status file to /var/lib/zonendpoint/status/signatures.json.

Format:
  {"signatures": <int>, "loaded_at": "...", "engine": "ClamAV x.y.z/<daily>"}
"""

import datetime
import json
import os
import re
import subprocess
import sys

OUTPUT_DIR = "/var/lib/zonendpoint/status"
OUTPUT_FILE = os.path.join(OUTPUT_DIR, "signatures.json")
MAX_SIGNATURES = 1_000_000_000
TZ_VN = datetime.timezone(datetime.timedelta(hours=7))

SIGNATURE_REGEX = re.compile(
    r"(?:Database correctly reloaded\s*\(\s*(\d+)\s*signatures\s*\)|Loaded\s+(\d+)\s+signatures)",
    re.IGNORECASE,
)
ENGINE_REGEX = re.compile(r"(ClamAV\s+[0-9.]+\/[0-9]+)", re.IGNORECASE)


def log(msg: str, is_error: bool = False):
    stream = sys.stderr if is_error else sys.stdout
    now = datetime.datetime.now(TZ_VN).strftime("%Y-%m-%dT%H:%M:%S+07:00")
    print(f"[{now}] {msg}", file=stream, flush=True)


def get_engine_version() -> str:
    """Extract ClamAV engine version like 'ClamAV 1.5.4/28182'."""
    for cmd in [["clamd", "--version"], ["clamscan", "--version"]]:
        try:
            res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=5)
            if res.returncode == 0 and res.stdout:
                m = ENGINE_REGEX.search(res.stdout)
                if m:
                    return m.group(1).strip()
                # fallback to first two tokens if pattern differs
                parts = res.stdout.strip().split("/")
                if len(parts) >= 2:
                    return f"{parts[0]}/{parts[1]}".strip()
                return res.stdout.strip().splitlines()[0]
        except Exception:
            continue
    return "ClamAV"


def get_latest_signature_entry():
    """Query systemd journal for clamav-daemon signature events."""
    cmd = [
        "journalctl",
        "-u",
        "clamav-daemon.service",
        "-r",
        "-g",
        "signatures",
        "--no-pager",
        "-n",
        "10",
        "-o",
        "json",
    ]
    try:
        proc = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=10)
    except Exception as e:
        log(f"Failed to run journalctl: {e}", is_error=True)
        return None

    if proc.returncode != 0 or not proc.stdout.strip():
        log("No signature lines found in clamav-daemon journal", is_error=True)
        return None

    lines = [line.strip() for line in proc.stdout.strip().splitlines() if line.strip()]
    # Since journalctl -r outputs newest first, first valid match is the latest
    for raw_line in lines:
        try:
            entry = json.loads(raw_line)
        except Exception:
            continue

        message = entry.get("MESSAGE", "")
        m = SIGNATURE_REGEX.search(message)
        if not m:
            continue

        count_str = m.group(1) or m.group(2)
        try:
            sig_count = int(count_str)
        except (ValueError, TypeError):
            continue

        if sig_count <= 0 or sig_count > MAX_SIGNATURES:
            log(f"Signature count {sig_count} outside valid range 1..{MAX_SIGNATURES}", is_error=True)
            continue

        # Extract timestamp in UTC+7 (Asia/Ho_Chi_Minh)
        ts_str = None
        realtime_ts = entry.get("__REALTIME_TIMESTAMP")
        if realtime_ts:
            try:
                micros = int(realtime_ts)
                dt = datetime.datetime.fromtimestamp(micros / 1_000_000.0, tz=TZ_VN)
                ts_str = dt.strftime("%Y-%m-%dT%H:%M:%S+07:00")
            except Exception:
                ts_str = None

        if not ts_str:
            # Fallback to current UTC+7 time if timestamp parsing fails
            ts_str = datetime.datetime.now(TZ_VN).strftime("%Y-%m-%dT%H:%M:%S+07:00")

        return sig_count, ts_str

    log("Could not find matching signature pattern in journal entries", is_error=True)
    return None


def main():
    result = get_latest_signature_entry()
    if not result:
        log("Signature extraction failed; keeping existing signatures.json if present", is_error=True)
        sys.exit(1)

    sig_count, ts_str = result
    engine_ver = get_engine_version()

    data = {
        "signatures": sig_count,
        "loaded_at": ts_str,
        "engine": engine_ver,
    }

    os.makedirs(OUTPUT_DIR, mode=0o755, exist_ok=True)
    tmp_path = f"{OUTPUT_FILE}.tmp.{os.getpid()}"

    try:
        with open(tmp_path, "w", encoding="utf-8") as f:
            json.dump(data, f, indent=2)
            f.write("\n")
        os.chmod(tmp_path, 0o644)
        os.replace(tmp_path, OUTPUT_FILE)
        log(f"Successfully wrote {OUTPUT_FILE}: {data}")
    except Exception as e:
        log(f"Failed to write status file {OUTPUT_FILE}: {e}", is_error=True)
        if os.path.exists(tmp_path):
            try:
                os.remove(tmp_path)
            except Exception:
                pass
        sys.exit(1)


if __name__ == "__main__":
    main()
