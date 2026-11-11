# ThreatFox (abuse.ch) server feed — J-02 / V-39 / V-40

**Date:** 2026-11-11 · **Status:** Production live deployed on lab server `100.96.250.245` (Task V-40). Real full export ingested, 21,476 unique SHA-256 signatures deployed to clamd (8,592,748 -> 8,614,224). Tunnel 13310 verified. Zero secret leaks.

## Terms and feed facts

ThreatFox's Community API is free under abuse.ch fair-use principles. Companies, networks, or individuals with commercial/for-profit needs may require the enhanced commercial API. Zon approved only non-commercial use on 1–2 personal machines; review current abuse.ch/Spamhaus terms before any company deployment. [Community API and terms](https://threatfox.abuse.ch/api/)

ThreatFox shares malware-related IOCs. The documented API supports `sha256`, `md5`, `ip:port`, `domain` and `url` indicators and metadata including malware family, confidence level and first/last-seen. IOCs older than six months are no longer exposed through API/export since 2025-05-01. Some network-format exports are regenerated every five minutes; this product integration remains manually run, without timers. [ThreatFox API](https://threatfox.abuse.ch/api/) · [Export documentation](https://threatfox.abuse.ch/export/)

The current export page documents authenticated full CSV ZIP at `https://threatfox-api.abuse.ch/v2/files/exports/<AUTH-KEY>/full.csv.zip`.

### Real Authenticated CSV Schema & Parsing (Task V-40)
The live authenticated export CSV header was verified and parsed on the lab host:
```text
first_seen_utc, ioc_id, ioc_value, ioc_type, threat_type, fk_malware, malware_alias, malware_printable, last_seen_utc, confidence_level, is_compromised, reference, tags, anonymous, reporter
```
- Preamble contains `#` comments; comments can also appear in mid-file or at the bottom.
- Header line begins with `# ` and quoted field names with spaces after commas. Parsed via `csv.DictReader(..., skipinitialspace=True)`.
- Data values are enclosed in double quotes with possible internal spaces (e.g. malware names, timestamps). Leading and trailing quotes/spaces stripped defensively.
- IOC indicator column is `ioc_value` (fallback to `ioc`, `sha256`, etc.).
- IOC types mapped: `sha256_hash` -> `sha256`, `md5_hash` -> `md5`, `sha1_hash` -> `sha1`, `ip:port` -> `ip:port`, `domain` -> `domain`, `url` -> `url`.
- Malware name column is `malware_printable` (empty, `"None"`, or whitespace -> `"Unknown"`).
- Confidence level is `confidence_level` (parsed as integer 0-100).
- Timestamps are `first_seen_utc` and `last_seen_utc`.
- ThreatFox does NOT provide a `file_size` column in this export (all exported file hashes use ClamAV wildcard size `*` with FLEVEL 73).
- Rejection threshold: If imported rows = 0 or malformed rejected rows > 5%, import transaction rolls back, halts immediately without deploying, and logs `[ALERT-LOKI]`.

## Security and architecture

- Server-only API use. Auth-Key is read from `/etc/zonendpoint/abusech.key` (`0600 root:root`); it is never passed on argv, committed, logged, or sent to clients. The export URL contains the key in its path, so exception/output text is masked as `<AUTH-KEY>`.
- Secret leak audit: `sudo sh -c 'grep -rlFf /etc/zonendpoint/abusech.key /opt/zonendpoint /var/lib/clamav /var/backups /var/log /tmp /home/admzon/.bash_history /root/.bash_history'` returned 0 matches (100% clean).
- Preserved Dump File: Downloaded export archive is atomically saved to `/opt/zonendpoint/threatfox/last-full.csv.zip` (`0600 root:root`). The path contains no Auth-Key.
- Offline Re-import: Subcommand `import-file` allows re-importing from the saved dump file (`--file /opt/zonendpoint/threatfox/last-full.csv.zip`) consuming 0 requests if parser or downstream adjustments are needed.
- SQLite database: `/opt/zonendpoint/threatfox/threatfox.db`. IOC values are normalized/deduplicated and saved transactionally with IOC type, malware, confidence, first/last-seen, source, and update time. Failed imports roll back without modifying existing records.
- SHA-256 signatures only are exported to `/var/lib/clamav/threatfox.hsb`; other IOC types remain metadata for future network features. Existing MalwareBazaar SHA-256 values are excluded from the ThreatFox signature file. Missing sizes use wildcard `*` and FLEVEL 73; names use `ThreatFox.<malware|Unknown>.UNOFFICIAL`.
- ClamAV deployment reuses MalwareBazaar's staged test, available-RAM >=3 GB guard, backup, atomic replacement, zRELOAD, PING/PONG, signature delta check, 120-second reload ceiling, and automatic rollback. Separate rollback command is `/usr/local/bin/zonendpoint-threatfox-rollback.sh`.
- Shared rate limiter: `/opt/zonendpoint/abusech/rate_limit.json`: combined maximum 3 requests per sliding hour (3600s) and 400 per 24 hours. Old MalwareBazaar timestamp history migrated (6 requests preserved). When both feeds are waiting, MalwareBazaar has priority on empty hourly window, followed by alternation MB → TF → MB. If rate limit is hit, scripts stop immediately without retry or waiting.
- No systemd timer/cron, no clamd/freshclam config change, clamd remains `127.0.0.1:3310`; clients consume detections through clamd only via SSH tunnel `127.0.0.1:13310 -> 3310`.

## Manual commands (lab operator)

```bash
sudo /usr/local/bin/zonendpoint-threatfox.py check
sudo /usr/local/bin/zonendpoint-threatfox.py fetch-full
sudo /usr/local/bin/zonendpoint-threatfox.py import-file --file /opt/zonendpoint/threatfox/last-full.csv.zip
sudo /usr/local/bin/zonendpoint-threatfox.py export-hsb --output /tmp/threatfox.hsb
sudo /usr/local/bin/zonendpoint-threatfox.py deploy-hsb
sudo /usr/local/bin/zonendpoint-threatfox-rollback.sh
```

## Lab Verification & Live Run (Task V-40)

1. **Automated Test Suites (Lab Ubuntu 26.04 / Python 3.14.4)**:
   - `test_abusech_rate_limit.py`: 4/4 PASS
   - `test_zonendpoint_threatfox.py`: 14/14 PASS (added preamble/quotes/postamble real format fixture, dump file preservation, import-file 0-request replay, and 5% rejection threshold tests)
   - `test_zonendpoint_malwarebazaar.py`: 28/28 PASS
   - Total: **46/46 PASS**.

2. **Lab Real Execution Results**:
   - `fetch-full`: 1 request used (3/3 hourly sliding window, 9/400 daily total).
   - Preserved dump file: `/opt/zonendpoint/threatfox/last-full.csv.zip` (4,843,512 bytes, mode 0600 root:root).
   - Ingestion breakdown:
     - Total data rows: 135,585
     - Imported valid rows: 135,585
     - Rejected rows (malformed): 0 (0.00% <= 5% threshold)
     - Breakdown by IOC type:
       - `sha256`: 45,041
       - `ip:port`: 29,323
       - `domain`: 29,125
       - `url`: 19,026
       - `md5`: 7,058
       - `sha1`: 6,012
     - SHA-256 hashes in ThreatFox: 45,041
     - Duplicates with MalwareBazaar excluded: 23,564
     - Net unique deployable SHA-256 for clamd: 21,476
   - Offline replay verification (`import-file`): Tested on `/tmp/test_import.db` using saved dump file; 135,585 rows imported cleanly with 0 requests consumed.

3. **Live ClamAV Deployment (`deploy-hsb`)**:
   - Preflight check: Temp DB with base databases verified cleanly (3,649,734 known viruses, 0 errors, 0 False Positives on benign targets).
   - Available host RAM check: 11.96 GB (threshold: 3.00 GB) -> PASS.
   - clamd PID 2245 RSS before reload: 2,303.66 MB.
   - Active file updated: `/var/lib/clamav/threatfox.hsb` (21,476 signatures, mode 0644 clamav:clamav).
   - clamd reload: `zRELOAD` -> PONG in 26.01s (limit: 120s).
   - clamd signatures: **8,592,748 -> 8,614,224** (delta **+21,476**, exact match).
   - clamd PID 2245 RSS after reload: 2,305.27 MB (+1.61 MB).
   - Engine status: PONG (127.0.0.1:3310).

4. **Live Verification via Tunnel 13310**:
   - Scanned benign files via loopback tunnel `127.0.0.1:13310`:
     - `package.json`: `stream: OK`
     - `README.md`: `stream: OK`
     - `Cargo.toml`: `stream: OK`

5. **Secret Leak Audit**:
   - Ran `sudo sh -c 'grep -rlFf /etc/zonendpoint/abusech.key /opt/zonendpoint /var/lib/clamav /var/backups /var/log /tmp /home/admzon/.bash_history /root/.bash_history'`: returned 0 matches (100% clean).
