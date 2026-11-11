# ThreatFox (abuse.ch) server feed — J-02 / V-39

**Date:** 2026-11-11 · **Status:** Lab verified; real check & fetch-full executed (Task V-39). Real authenticated CSV schema confirmed. Live clamd preserved.

## Terms and feed facts

ThreatFox's Community API is free under abuse.ch fair-use principles. Companies, networks, or individuals with commercial/for-profit needs may require the enhanced commercial API. Zon approved only non-commercial use on 1–2 personal machines; review current abuse.ch/Spamhaus terms before any company deployment. [Community API and terms](https://threatfox.abuse.ch/api/)

ThreatFox shares malware-related IOCs. The documented API supports `sha256`, `md5`, `ip:port`, `domain` and `url` indicators and metadata including malware family, confidence level and first/last-seen. IOCs older than six months are no longer exposed through API/export since 2025-05-01. Some network-format exports are regenerated every five minutes; this product integration remains manually run, without timers. [ThreatFox API](https://threatfox.abuse.ch/api/) · [Export documentation](https://threatfox.abuse.ch/export/)

The current export page documents authenticated full CSV ZIP at `https://threatfox-api.abuse.ch/v2/files/exports/<AUTH-KEY>/full.csv.zip`.

### Real Authenticated CSV Schema (Verified in Task V-39)
The live authenticated export CSV header was retrieved on the lab host:
```text
first_seen_utc, ioc_id, ioc_value, ioc_type, threat_type, fk_malware, malware_alias, malware_printable, last_seen_utc, confidence_level, is_compromised, reference, tags, anonymous, reporter
```
- Preamble contains `#` comments.
- IOC indicator column is `ioc_value` (not `ioc`).
- Malware name column is `malware_printable` (not `malware`).
- Timestamps are `first_seen_utc` and `last_seen_utc`.
- ThreatFox does NOT provide a `file_size` column in this export (all exported file hashes use ClamAV wildcard size `*` with FLEVEL 73).

## Security and architecture

- Server-only API use. Auth-Key is read from `/etc/zonendpoint/abusech.key` (`0600 root:root`); it is never passed on argv, committed, logged, or sent to clients. The export URL contains the key in its path, so exception/output text is masked as `<AUTH-KEY>`.
- Secret leak audit: `sudo sh -c 'grep -rlFf /etc/zonendpoint/abusech.key /opt/zonendpoint /var/lib/clamav /var/backups /var/log /tmp /home/admzon/.bash_history /root/.bash_history'` returned 0 matches (100% clean).
- SQLite database: `/opt/zonendpoint/threatfox/threatfox.db`. IOC values are normalized/deduplicated and saved transactionally with IOC type, malware, confidence, first/last-seen, source, and update time. Failed imports roll back without modifying existing records.
- SHA-256 signatures only are exported to `/var/lib/clamav/threatfox.hsb`; other IOC types remain metadata for future network features. Existing MalwareBazaar SHA-256 values are excluded from the ThreatFox signature file. Missing sizes use wildcard `*` and FLEVEL 73; names use `ThreatFox.<malware|Unknown>.UNOFFICIAL`.
- ClamAV deployment reuses MalwareBazaar's staged test, available-RAM >=3 GB guard, backup, atomic replacement, zRELOAD, PING/PONG, signature delta check, 120-second reload ceiling, and automatic rollback. Separate rollback command is `/usr/local/bin/zonendpoint-threatfox-rollback.sh`.
- Shared rate limiter: `/opt/zonendpoint/abusech/rate_limit.json`: combined maximum 3 requests per sliding hour (3600s) and 400 per 24 hours. Old MalwareBazaar timestamp history migrated (6 requests preserved). When both feeds are waiting, MalwareBazaar has priority on empty hourly window, followed by alternation MB → TF → MB. If rate limit is hit, scripts stop immediately without retry or waiting.
- No systemd timer/cron, no clamd/freshclam config change, clamd remains `127.0.0.1:3310`; clients consume detections through clamd only via SSH tunnel `127.0.0.1:13310 -> 3310`.

## Manual commands (lab operator)

```bash
sudo /usr/local/bin/zonendpoint-threatfox.py check
sudo /usr/local/bin/zonendpoint-threatfox.py fetch-full
sudo /usr/local/bin/zonendpoint-threatfox.py export-hsb --output /tmp/threatfox.hsb
sudo /usr/local/bin/zonendpoint-threatfox.py deploy-hsb
sudo /usr/local/bin/zonendpoint-threatfox-rollback.sh
```

## Lab Verification & Live Run (Task V-39)

1. **Automated Test Suites (Lab Ubuntu 26.04 / Python 3.14.4)**:
   - `test_abusech_rate_limit.py`: 4/4 PASS
   - `test_zonendpoint_threatfox.py`: 11/11 PASS
   - `test_zonendpoint_malwarebazaar.py`: 28/28 PASS
   - Total: 43/43 PASS.
   - `bash -n /usr/local/bin/zonendpoint-threatfox-rollback.sh`: PASS (CRLF -> LF converted, header set to J-02).

2. **Lab Real Execution**:
   - `check`: 1 request used. Upstream API response `query_status: ok`.
   - `fetch-full`: 1 request used. Authenticated dump downloaded, ZIP uncompressed, real CSV header inspected.
   - Budget consumed: 2 requests in Task V-39 (2/3 hourly sliding window, 8/400 daily total).
   - Ingestion safely guarded: parser checked against real schema; 0 rows ingested into `threatfox.db` (clean at 0 rows, no corrupted data).
   - ClamAV live engine: PID 2245, RSS ~2.35 GB, signature count 8,592,748 (stable, unchanged).
   - Benign scan verification via tunnel `127.0.0.1:13310`: `README.md`, `package.json`, `project-status.md` scanned clean (`stream: OK`).
