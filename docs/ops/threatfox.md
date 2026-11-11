# ThreatFox (abuse.ch) server feed — J-02

**Date:** 2026-11-11 · **Status:** code/mock handoff only; no lab access or live request.

## Terms and feed facts

ThreatFox's Community API is free under abuse.ch fair-use principles. Companies, networks, or individuals with commercial/for-profit needs may require the enhanced commercial API. Zon approved only non-commercial use on 1–2 personal machines; review current abuse.ch/Spamhaus terms before any company deployment. [Community API and terms](https://threatfox.abuse.ch/api/)

ThreatFox shares malware-related IOCs. The documented API supports `sha256`, `md5`, `ip:port`, `domain` and `url` indicators and metadata including malware family, confidence level and first/last-seen. IOCs older than six months are no longer exposed through API/export since 2025-05-01. Some network-format exports are regenerated every five minutes; this product integration remains manually run, without timers. [ThreatFox API](https://threatfox.abuse.ch/api/) · [Export documentation](https://threatfox.abuse.ch/export/)

The current export page documents authenticated full CSV ZIP at `https://threatfox-api.abuse.ch/v2/files/exports/<AUTH-KEY>/full.csv.zip`. Detailed CSV schema is login-gated. The importer resolves field names from the CSV header and supports header aliases; the included fixture is synthetic, based on public API field names. **Column mapping and content still require verification against a real authenticated export before live ingestion.** No API key was used for this research.

## Security and architecture

- Server-only API use. Auth-Key is read from `/etc/zonendpoint/abusech.key`; it is never passed on argv, committed, logged, or sent to clients. The export URL contains the key in its path, so exception/output text is masked as `<AUTH-KEY>`.
- SQLite database: `/opt/zonendpoint/threatfox/threatfox.db`. IOC values are normalized/deduplicated and saved transactionally with IOC type, malware, confidence, first/last-seen, source, and update time. Failed imports roll back.
- SHA-256 signatures only are exported to `/var/lib/clamav/threatfox.hsb`; other IOC types remain metadata for future network features. Existing MalwareBazaar SHA-256 values are excluded from the ThreatFox signature file. Missing sizes use wildcard `*` and FLEVEL 73; names use `ThreatFox.<malware|Unknown>.UNOFFICIAL`.
- ClamAV deployment reuses MalwareBazaar's staged test, available-RAM >=3 GB guard, backup, atomic replacement, zRELOAD, PING/PONG, signature delta check, 120-second reload ceiling, and automatic rollback. Separate rollback command is `/usr/local/bin/zonendpoint-threatfox-rollback.sh`.
- Both scripts use `/opt/zonendpoint/abusech/rate_limit.json`: combined maximum 3 requests per sliding hour and 400 per 24 hours. The old MalwareBazaar timestamp-only state is imported as `malwarebazaar` history. New records include timestamp and source. If both feeds are being run, pass `--threatfox-pending` to MalwareBazaar or `--malwarebazaar-pending` to ThreatFox; this enforces MB → TF → MB turns. When only one feed is running it can use available requests. Reaching a budget stops immediately without waiting/retry.
- No systemd timer/cron, no clamd/freshclam config change, clamd remains `127.0.0.1:3310`; clients consume detections through clamd only.

## Manual commands (lab operator)

```bash
sudo /usr/local/bin/zonendpoint-threatfox.py check
sudo /usr/local/bin/zonendpoint-threatfox.py fetch-full
sudo /usr/local/bin/zonendpoint-threatfox.py export-hsb --output /tmp/threatfox.hsb
sudo /usr/local/bin/zonendpoint-threatfox.py deploy-hsb
sudo /usr/local/bin/zonendpoint-threatfox-rollback.sh
```

These commands were not run on the lab in J-02. Review export headers and run a real `check`, `fetch-full`, temporary database validation, then deployment under the host operator's normal change procedure. `check` and each fetch consume one request from the combined budget.

## Mock verification

Run `python3 scripts/feeds/test_abusech_rate_limit.py`, `python3 scripts/feeds/test_zonendpoint_threatfox.py`, and the MalwareBazaar regression suite `python3 scripts/feeds/test_zonendpoint_malwarebazaar.py`. Execution in the current Windows environment is NOT RUN because Python is unavailable. No live request, SSH, lab test or clamd operation was performed.
