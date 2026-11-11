# V-40 ThreatFox Real Run & Deployment — Vic to Qeen handoff

## Code snapshot
- Branch: `jacky/threatfox`
- Base: `origin/jacky/threatfox` tip `fefa9db` (Task V-39)
- Code Snapshot SHA: `8eb08e74ca116fbc9cd88913f6d037fa147736b1` (`8eb08e7`)
- Worktree: `E:\Zonendpoint-worktrees\v40-threatfox`
- Task: V-40 (Complete real ThreatFox run, export CSV parser fixes, dump preservation, 0-request import-file, clamd deployment)

## 1. Resolved Defects & Parser Enhancements

1. **Abuse.ch Export CSV Parser Compliance**:
   - Handled `#` comment preambles, mid-file comments, and `#` postamble records.
   - Handled `# ` header prefix and quoted column names with spaces after commas via `csv.DictReader(..., skipinitialspace=True)`.
   - Stripped leading/trailing quotes and spaces defensively on all data values while preserving internal whitespace.
   - Standard IOC type mapping: `sha256_hash` -> `sha256`, `md5_hash` -> `md5`, `sha1_hash` -> `sha1`, `ip:port` -> `ip:port`, `domain` -> `domain`, `url` -> `url`.
   - Malware family resolution: `malware_printable` with fallback to `malware`/`signature`; empty, `"None"`, or whitespace normalized to `"Unknown"`.
   - Confidence level: `confidence_level` converted to integer.
   - Timestamps: `first_seen_utc` and `last_seen_utc`.
   - Wildcard file size `*` and FLEVEL 73 for all ThreatFox hashes.

2. **Dump File Preservation & Offline Replay**:
   - Streamed download saved to `/opt/zonendpoint/threatfox/last-full.csv.zip` (permissions `0600 root:root`, no key in filename or path).
   - Added subcommand `import-file` (`--file` / `--dump-file`) to re-import from local ZIP/CSV consuming 0 API requests.

3. **Rejection Rate Guard**:
   - Transactional import tracks total rows, imported valid rows, and rejected malformed rows.
   - If imported rows = 0 or malformed rejection rate > 5%, the transaction immediately rolls back, halts deployment, and logs `[ALERT-LOKI]`.

4. **Multi-DB Read-Only Concurrency Fix**:
   - Fixed `ATTACH DATABASE` on `/opt/zonendpoint/malwarebazaar/malwarebazaar.db` with `?immutable=1` URI to allow non-root test users (`admzon`) to run hash deduplication checks without triggering SQLite `attempt to write a readonly database` error.

## 2. Test Execution on Lab Host (`100.96.250.245`, Python 3.14.4)

All three test suites run in temporary directory and passed 100%:
- `test_abusech_rate_limit.py`: **4/4 PASS** (daily cap, legacy migration, strict non-dict rejection, empty-window alternation).
- `test_zonendpoint_threatfox.py`: **14/14 PASS** (real format fixture, preamble/quotes/spaces, map ioc_type, dump preservation, import-file 0-request replay, 5% rejection threshold abort, header resolution, atomic rollback, hash exclusion, key masking, 429 fail-fast, streaming chunk download, size limits, log-file forwarding).
- `test_zonendpoint_malwarebazaar.py`: **28/28 PASS** (full regression suite, key sanitization, zero-downtime deployment, rollback).
- Total: **46/46 PASS**.

## 3. Lab Installation & Real Execution

1. **Binary Installed**:
   - `/usr/local/bin/zonendpoint-threatfox.py` (755 root:root) updated with new parser, dump preservation, and `import-file` command.

2. **Real `fetch-full` (1 request used)**:
   - Shared limiter state: 3/3 hourly sliding window, 9/400 daily total across abuse.ch feeds.
   - Dump file preserved: `/opt/zonendpoint/threatfox/last-full.csv.zip` (4,843,512 bytes, mode 0600 root:root).
   - Ingestion breakdown:
     - Total data rows: **135,585**
     - Imported valid rows: **135,585**
     - Rejected rows: **0 (0.00%)**
     - Breakdown by IOC type:
       - `sha256`: 45,041
       - `ip:port`: 29,323
       - `domain`: 29,125
       - `url`: 19,026
       - `md5`: 7,058
       - `sha1`: 6,012
     - SHA-256 total in ThreatFox: 45,041
     - Duplicates with MalwareBazaar excluded: 23,564
     - Net unique deployable SHA-256 for clamd: **21,476**
   - Offline replay (`import-file`): Verified on `/tmp/test_import.db` with 0 requests consumed.

3. **Live ClamAV Deployment (`deploy-hsb`)**:
   - Host available RAM: 11.96 GB (threshold: 3.00 GB) -> PASS.
   - Preflight check on temporary DB: 3,649,734 known viruses, 0 errors, 0 False Positives on benign targets -> PASS.
   - Active file updated: `/var/lib/clamav/threatfox.hsb` (21,476 signatures, mode 0644 clamav:clamav).
   - Zero-downtime clamd reload (`zRELOAD`): completed in 26.01s (limit: 120s).
   - Live clamd signatures: **8,592,748 -> 8,614,224** (delta **+21,476**, exact match).
   - Live clamd RSS: 2,303.66 MB -> 2,305.27 MB (+1.61 MB).
   - Engine health verified: PING -> PONG on `127.0.0.1:3310`.

4. **Live Scanning & Leak Audit Verification**:
   - Benign file scans via tunnel `127.0.0.1:13310`: `package.json`, `README.md`, `Cargo.toml` all return `stream: OK`.
   - Secret leak audit: `sudo sh -c 'grep -rlFf /etc/zonendpoint/abusech.key /opt/zonendpoint /var/lib/clamav /var/backups /var/log /tmp /home/admzon/.bash_history /root/.bash_history'` returned 0 matches (100% clean).
   - Invariants preserved: zero clamd.conf/freshclam.conf modifications, clamd remains on `127.0.0.1:3310`, no cron/timers installed.
