# V-39 ThreatFox & Shared Limiter — Vic to Qeen handoff

## Code snapshot
- Branch: `jacky/threatfox`
- Base: `origin/jacky/threatfox` tip `cf0b7f8` (code commit `de960f9`, base `main` `a915222`)
- Worktree: `E:\Zonendpoint-worktrees\v39-threatfox`
- Task: V-39 (Resolve Q-28 review items, live first run on lab server `100.96.250.245`)

## 1. Resolved Q-28 Defects

1. **Rollback Script CRLF & Header Fix**:
   - Converted `scripts/feeds/zonendpoint-threatfox-rollback.sh` to LF line endings.
   - Updated header comment to `# Zonendpoint ThreatFox ClamAV Hash Signature Rollback Script (Task J-02)`.
   - Added root `.gitattributes` enforcing LF (`*.sh text eol=lf`, `*.py text eol=lf`).
   - Verified `bash -n` on lab host: PASS.

2. **Rate Limiter Strict JSON Validation (No Silent Overwrite)**:
   - Modified `abusech_rate_limit.py`: `_read` strictly checks `isinstance(raw, dict)` and `"requests" in raw`, and validates `isinstance(rows, list)`.
   - Any malformed or non-dict JSON raises `ValueError` and refuses requests immediately without modifying or overwriting state file.
   - Added automated test `test_invalid_json_or_missing_requests_refuses_without_overwrite`: PASS.

3. **Alternation Logic Priority on Empty Hourly Window**:
   - Fixed `abusech_rate_limit.py`: when `other_waiting=True` and `hourly` is empty, turn strictly belongs to `malwarebazaar` (`ORDER[0]`). ThreatFox is blocked until MalwareBazaar runs, preserving Zon policy `MB -> TF -> MB`.
   - Added automated test `test_empty_hourly_with_other_waiting_prioritizes_malwarebazaar`: PASS.

4. **Self-Contradictory Test Case in ThreatFox Suite**:
   - Succeeded hash exclusion test `test_hsb_wildcard_and_malwarebazaar_hash_exclusion` in `test_zonendpoint_threatfox.py`: corrected line ~52 to `self.assertNotIn("ThreatFox.Fam.UNOFFICIAL", out.read_text())` as `"a"*64` is excluded due to MalwareBazaar collision.

5. **CLI Parameter Forwarding & Memory-Safe Streaming**:
   - Forwarded `--log-file` to `mb.deploy_hsb(..., log_file=args.log_file)` in `zonendpoint-threatfox.py`. Added test `test_deploy_hsb_passes_log_file`: PASS.
   - Refactored `fetch_full` in `zonendpoint-threatfox.py` to stream HTTP chunks (64 KB) to a temporary ZIP file on disk, validating download size $\le 512$ MB without reading archive into RAM. Extracted CSV is streamed directly to disk, enforcing size limits. Added tests `test_fetch_full_streams_to_temp_file_and_extracts_cleanly` and `test_fetch_full_oversized_download_is_rejected`: PASS.

## 2. Test Execution on Lab Host (`100.96.250.245`, Python 3.14.4)

All three test suites run in temporary directory and passed 100%:
- `test_abusech_rate_limit.py`: **4/4 PASS** (daily cap, legacy migration, strict non-dict rejection, empty-window alternation).
- `test_zonendpoint_threatfox.py`: **11/11 PASS** (header resolution, preamble parsing, atomic rollback, hash exclusion, key masking, 429 fail-fast, streaming chunk download, size limits, log-file forwarding).
- `test_zonendpoint_malwarebazaar.py`: **28/28 PASS** (full regression suite, key sanitization, zero-downtime deployment, rollback).
- Total: **43/43 PASS**.

## 3. Lab Installation & State Migration

- Installed binaries to `/usr/local/bin`:
  - `abusech_rate_limit.py` (755 root:root)
  - `zonendpoint-malwarebazaar.py` (755 root:root, with `de51ffe` key sanitization)
  - `zonendpoint-threatfox.py` (755 root:root)
  - `zonendpoint-threatfox-rollback.sh` (755 root:root)
- Created directories: `/opt/zonendpoint/abusech`, `/opt/zonendpoint/threatfox`, `/var/backups/clamav-threatfox` (755 root:root).
- Migrated legacy state `/opt/zonendpoint/malwarebazaar/rate_limit.json` -> `/opt/zonendpoint/abusech/rate_limit.json`: verified all 6 original timestamps preserved with source tag `malwarebazaar`.

## 4. Real Lab Execution Results (2 Requests Consumed)

1. **`sudo /usr/local/bin/zonendpoint-threatfox.py check`**:
   - Consumed 1 request (1/3 hourly, 7/400 daily).
   - Upstream API result: `ThreatFox API check: ok`.

2. **`sudo /usr/local/bin/zonendpoint-threatfox.py fetch-full`**:
   - Consumed 1 request (2/3 hourly, 8/400 daily).
   - Real CSV header discovered:
     ```text
     ['first_seen_utc', 'ioc_id', 'ioc_value', 'ioc_type', 'threat_type', 'fk_malware', 'malware_alias', 'malware_printable', 'last_seen_utc', 'confidence_level', 'is_compromised', 'reference', 'tags', 'anonymous', 'reporter']
     ```
   - Ingestion status: Safeguard tripped on comment preamble/parsing mismatch. Database `/opt/zonendpoint/threatfox/threatfox.db` kept strictly clean at 0 rows. No unverified signatures ingested into clamd.

3. **ClamAV Engine Invariants**:
   - Live daemon: PID 2245, RSS ~2.35 GB, signature count 8,592,748 (stable, unchanged).
   - Zero clamd.conf / freshclam.conf modifications; clamd remains on `127.0.0.1:3310`.
   - Benign scan verification via tunnel `127.0.0.1:13310`: `README.md`, `package.json`, `project-status.md` scanned clean (`stream: OK`).
   - No cron or systemd timer installed.

4. **Secret Leak Audit**:
   - Ran `sudo sh -c 'grep -rlFf /etc/zonendpoint/abusech.key /opt/zonendpoint /var/lib/clamav /var/backups /var/log /tmp /home/admzon/.bash_history /root/.bash_history'`: returned 0 matches (100% clean).
