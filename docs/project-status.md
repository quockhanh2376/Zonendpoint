# Project status — 2026-11-11

Phase: current scan candidate stabilization for **non-commercial use on 1–2 personal machines** (confirmed by Zon 2026-11-10; company use requires reviewing abuse.ch/Spamhaus terms). Integration of J-01 (`jacky/adaptive-concurrency`, Q-26 PASS) and MalwareBazaar server feed (`vic/malwarebazaar`, Q-22..Q-25 PASS) approved by Zon (2026-11-11 "Merge cả hai") in Task V-38. Fast-forwarded `main` from `0979f5f` to `de51ffe` (pushed to origin).

## Active checkout/snapshot
- Implementation: `main` at commit `de51ffee3d98a47f7d396daccd727ac752310c31` (`de51ffe`).
- Merged branches:
  - `origin/jacky/adaptive-concurrency` (tip `81984ad`, code `bab14a0`, Q-26 PASS): client adaptive scan concurrency based on physical RAM and CPU cores, capped at 4, server backoff on pressure.
  - `origin/vic/malwarebazaar` (tip `4f13d3f`, code `0109d5a`, Q-22..Q-25 PASS): server-only feed ingestion (`scripts/feeds/zonendpoint-malwarebazaar.py`), full dump bootstrap (1,171,240 signatures in `/var/lib/clamav/malwarebazaar.hsb`), live clamd zero-downtime reload (signatures 7,421,608 -> 8,592,748), rollback script.
  - Key security hardening (`de51ffe`): sanitized Auth-Key in abuse.ch export download URLs across exceptions, logs, and error traces (`<AUTH-KEY>`, `from None`), added mock tests 27 and 28 in `test_zonendpoint_malwarebazaar.py` (28/28 PASS, 0 real requests).
- Handoff doc: [merge-v38-vic-to-qeen.md](handoffs/merge-v38-vic-to-qeen.md).
- Completed on branch `vic/cache-reason`:
  1. **P1-01 Cache Threat Name**: Added `threat_name TEXT` to SQLite `scan_cache` schema (migrated `user_version` 1 -> 2).
  2. **INFECTED Preservation**: Cache hits for infected entries return exact threat name (e.g. `YARA.<rule>.UNOFFICIAL`) instead of "verified content cache hit".
  3. **Safe Migration**: Legacy schema v1 cache rows with NULL threat names treated as cache miss (`Ok(None)`) to force live re-scan without crashing.
  4. **Security Invariants**: `ERROR` and `UNKNOWN` are never cached and never become `CLEAN`.
- Completed on branch `vic/ui-compact`:
  1. **UI-COMPACT (Task V-8)**: Status dot positioned directly beside logo; theme toggle integrated into scan card header row; no floating controls in window top-right; keyboard focus order: logo/status â†’ card title/toggle â†’ target picker â†’ primary action button; added 2 native mock checks (17/17 PASS).
- Previously completed on branch `vic/ui-simplify`:
- Operations audit (V-9): Branch `vic/engine-ops`, worktree `E:\Zonendpoint-vic-ops`. Read-only audit completed for lab ClamAV server on `100.96.250.245`. Audit findings in [engine-ops-audit-2026-11-07.md](qa/engine-ops-audit-2026-11-07.md), operations runbook in [engine-ops-runbook.md](ops/engine-ops-runbook.md), raw artifacts under `E:\Zonendpoint-tools\artifacts\engine-ops\2026-11-07\.`
  1. **UI-SIMPLIFY**: Target dropdown simplified to File/Folder only (removed Quick Scan menu item); primary button dynamically toggles "Quick Scan" / "Scan"; summary-first results view ("All Good" card on clean, threats-only on infected, non-clean rows + warning banner on incomplete).
  2. **Preflight & Status Icon**: Connection preflight on `zVERSION` with friendly error copy; topbar status dot (green / red / grey) with 30s background poll.
  3. **P0-01 Security Fix**: Required bounded EOF drain after clamd response frame to prevent frame desynchronization and pipelined response attacks.
  4. **DriverData Exclusion (Q-4-01)**: Excluded `%SystemRoot%\System32\drivers\DriverData` from Quick Scan target set (canonical path resolution, case-insensitive comparison, not counted, not shown, no row) while preserving security invariants for all other access-denied paths and user Folder scans.
  5. **Q-5-01 Layout Fix**: Text wrapping for unbroken long threat names (150+ chars).
- Draft PR: https://github.com/quockhanh2376/Zonendpoint/pull/1 is fully contained in main (fast-forwarded 2db42a8 â†’ ed1088c, Zon approved, Loki pushed) and can be closed on GitHub by Zon. No separate release or git tag has been created. No main merge or public release performed.
- Original source checkout E:\Zonendpoint/main and user changes remain preserved. Previously omitted architecture/licensing specs were restored under the same canonical relative filenames in this branch; architecture has a current addendum and Ed25519 content is unchanged. No alternate-name specs created.

## Current scope/UI
See [v1.0.0-scope.md](v1.0.0-scope.md). Manually initiated Quick scan (13 Windows core files + drivers, excluding `%SystemRoot%\System32\drivers\DriverData`) or picked file/folder Scan; Windows x64 app builds only. English, persisted Light/Dark, Guard OFF. SecConnect/licensing/quarantine deferred. Compact 800x240 UI, 168x162 logo and 13px slogan; Target dropdown has File / Folder only; default and post-scan auto-return to "Quick Scan"; summary-first result view (All Good card on clean, threats-only on infected, non-clean rows + warning on incomplete); server status dot (green / red / grey) with 30s idle poll next to logo; preflight checking `zVERSION` with generic "Cannot connect to ZonEndpoint server" copy; theme toggle inside scan card header row.


## Task V-16 â€” Server Status Icon & Alignment (2026-11-08)
- Branch: vic/status-icon (base origin/main at 4b8f0f4), worktree E:\Zonendpoint-vic-status.
- Code commit: 87992b0.
- Changes:
  1. **Inline SVG Icons**:
     - connected: checkmark âœ“ (--clean-button-text) on green circle (--clean).
     - disconnected: X mark (--clean-button-text) on red circle (--infected).
     - checking: spinning grey ring (--muted arc with --button-border track) with prefers-reduced-motion animation-off override.
     - 44Ã—44px button hit target maintained; title and aria-label preserved and dynamically updated; contrast compliant in Light and Dark themes.
  2. **Top-Alignment**:
     - .brand-identity changed to align-items: flex-start.
     - Top edge of .server-status box is collinear with top edge of "Scan files and folders" card (.selection-panel) (Math.abs(s.top - c.top) <= 2).
  3. **Quality Gates & Artifacts**:
     - npm test / node tests/native/run.mjs: 17/17 PASS (mock daemon).
     - npm run build: PASS (TypeScript + Vite).
     - cargo test: 58/58 PASS (39 lib, 1 main, 18 resilience).
     - cargo fmt --check: PASS.
     - cargo clippy -- -D warnings: PASS.
     - 6 screenshots saved in E:\Zonendpoint-tools\artifacts\status-icon\:
       - connected-light.png, connected-dark.png
       - checking-light.png, checking-dark.png
       - disconnected-light.png, disconnected-dark.png


## Task V-17 â€” Fix Status Icon Flicker (Q-14 P2-UX, 2026-11-08)
- Branch: vic/status-icon, worktree E:\Zonendpoint-vic-status.
- Code commit: 5199892.
- Fix:
  1. `checkServer(manual = false)`: only manual invocation (`onClick` on status button) sets `serverStatus = "checking"` and displays the spinner.
  2. Preflight in `startScan` and periodic 30s background checks run with `manual = false`, keeping the current status icon without flicker, and updating only if verdict differs.
  3. Added native mock check in `tests/native/run.mjs` validating both branches (18/18 native mock checks PASS, 58/58 cargo tests PASS).

## Verification/evidence
- ENGINE-OPS read-only audit (V-9) on `vic/engine-ops`: Ubuntu 26.04 ClamAV 1.5.4 (3.64M sigs) audited on `100.96.250.245`. Audit findings in [engine-ops-audit-2026-11-07.md](qa/engine-ops-audit-2026-11-07.md), operations runbook in [engine-ops-runbook.md](ops/engine-ops-runbook.md), raw evidence in `E:\Zonendpoint-tools\artifacts\engine-ops\2026-11-07\`. Key risks noted: `Restart=no` on daemons, missing CVD backup, unconfigured freshclam alert hook, SSH keepalive 0, and 25M stream limit decision.
- Core 8e8204a: Rust 20, extended native mock 11 (incl 60s timeout/51-file threat visibility), real lab 6 PASS; fmt/clippy PASS, npm audit 0. Release test-hook guard verified.
- Real native File Unicode selection and Folder/Cancel/rescan PASS. [Core QA report](qa/report-8e8204a.md), [Folder follow-up](qa/folder-picker-8e8204a.md).
- Release package 0de3208: NSIS 0.1.0 x64 setup (`EE35537DBF8ABCBA2038F68B63A4396568F5D00AE757B2D1EBBC6F9CACD7AECD`) Qeen independent acceptance Aâ€“H PASS. Details in [quick-package-0de3208.md](qa/quick-package-0de3208.md) and [quick-package-vic-to-qeen.md](handoffs/quick-package-vic-to-qeen.md).
- LAB-RESILIENCE Tier 1: Rust 37 tests (25 lib, 1 CLI, 11 resilience integration tests), fmt, clippy -D warnings, and Vite frontend build all PASS. Addressed Qeen review items P0-01, P0-02, P1-01, P1-02, P1-03, P2-02. Zero new deps. Tier 2 Node.js zero-dep fault proxy harness provided at scripts/resilience-proxy.mjs. Details in [resilience-vic-to-qeen.md](handoffs/resilience-vic-to-qeen.md).
- UI-SIMPLIFY-PREFLIGHT 8c687b9: Rust 41 tests (29 lib, 1 main, 11 resilience), fmt, clippy -D warnings, Vite build, native mock 14/14 PASS. Details in [ui-simplify-vic-to-qeen.md](handoffs/ui-simplify-vic-to-qeen.md).
- P0-01 SECURITY FIX 0a55516: Rust 46 tests, bounded EOF drain, verified by Qeen in [ui-simplify-0a55516.md](qa/ui-simplify-0a55516.md) (All items PASS, identified Finding Q-4-01).
- QUICK-SCOPE-EXCLUSION 6294506: Rust 51 tests (34 lib, 1 main, 16 resilience), fmt, clippy -D warnings, Vite build, native mock 14/14 PASS. Excluded DriverData from Quick Scan target set while preserving all security invariants for other errors and user folder scans. Verified by Loki in Round 5.
- RELEASE CANDIDATE 79f0f59: Rust 51 tests, fmt, clippy -D warnings, Vite build, native mock 15/15 PASS (including Q-5-01 text wrap check). NSIS installer packaged at `E:\Zonendpoint-tools\artifacts\79f0f59\`. Details in [rc-vic-to-qeen.md](handoffs/rc-vic-to-qeen.md). Zon manual test: OK.
- UI-COMPACT 321d32b: Rust 51 tests, fmt, clippy -D warnings, Vite build, native mock 17/17 PASS (including status dot position next to logo, theme toggle inside card in both themes, keyboard focus order). NSIS installer packaged at `E:\Zonendpoint-tools\artifacts\321d32b\`. Details in [ui-compact-vic-to-qeen.md](handoffs/ui-compact-vic-to-qeen.md).
- CACHE-REASON d91cc29: Rust 58 tests, fmt, clippy -D warnings, Vite build PASS. Preserves threat name on cache hit for INFECTED entries, safe migration from v1 to v2. NSIS installer packaged at `E:\Zonendpoint-tools\artifacts\d91cc29\`. Details in [cache-reason-vic-to-qeen.md](handoffs/cache-reason-vic-to-qeen.md).
- INTEGRATE-UI-CACHE 69d8577: Rust 58 tests (39 lib, 1 main, 18 resilience), fmt, clippy -D warnings, Vite build, native mock suite 17/17 PASS. Integrates UI compact layout and P1-01 threat name cache hit fix. NSIS installer packaged at `E:\Zonendpoint-tools\artifacts\69d8577\`. Details in [integrate-vic-to-qeen.md](handoffs/integrate-vic-to-qeen.md).
- Task V-13 ENGINE HARDENING (Runbook V-9, items 1â€“5): lab server `udevlevel73` hardened via passwordless sudo over SSH. clamd drop-in `Restart=on-failure`, `RestartSec=5s`; freshclam drop-in `Restart=on-failure`, `RestartSec=30s`; `systemctl daemon-reload` only (clamd Main PID 2245 kept continuously running without restart). Known-good database daily backup job deployed with `sigtool --info` pre-verification, 3-copy retention, and systemd daily timer; rollback script `clamav-rollback-known-good.sh` created and verified in dry-run mode (`--dry-run`). Update health monitor timer active every 30m, logging local journal WARNING if daily database >48h old or freshclam fails (no external egress). SSH keepalive configured via drop-in (`ClientAliveInterval 30`, `ClientAliveCountMax 3`), verified with `sshd -t`, reloaded without restart (PID 3464 kept, PasswordAuthentication yes preserved). All verification logs, scripts, and pre/post PING evidence stored at `E:\Zonendpoint-tools\artifacts\engine-ops\2026-11-08-harden\`. Operational runbook report: [hardening-2026-11-08.md](ops/hardening-2026-11-08.md).
- CONNECT-SETUP (V-19): Persistent SSH tunnel supervisor (`scripts/windows/zon-tunnel.ps1`) with exponential backoff (5s -> 60s, reset at 30s stable) and scheduled task install/uninstall scripts (`scripts/windows/install-tunnel-task.ps1`, `scripts/windows/uninstall-tunnel-task.ps1`) running at user logon hidden without admin. Live verified on machine: old ssh pid 12576 stopped -> task ran -> zPING 127.0.0.1:13310 = PONG -> killed child ssh pid 8212 -> auto-reconnected new ssh pid 10272 -> zPING = PONG. Added in-app server configuration UI in `src/App.tsx` & `src/App.css` (compact, Light/Dark compliant, stored in `%LOCALAPPDATA%\ZonEndpoint\config.json`). Enforces private IP rules (loopback, RFC 1918, Tailscale 100.64.0.0/10; rejects public IPs). Priority: `ZONENDPOINT_CLAMD` > saved setting > default `127.0.0.1:13310`. Immediately tests connection upon save. Documented in `docs/ops/client-connect.md`. Quality gates: 62/62 Rust tests PASS, cargo fmt PASS, cargo clippy -D warnings PASS, npm run build PASS, 19/19 native tests PASS.
- Current release 01e7449: NSIS 0.1.0 x64 build, branded setup/uninstaller, install/shortcut/theme persistence, actual installed native Folder+real ClamAV 2-file scan, offline-not-CLEAN cache check, same-version repair, uninstall/data preservation PASS. [Release report](qa/release-installer-01e7449.md), [handoff](handoffs/release-vic-to-qeen.md).
- YARA-Forge Core live enablement on lab (Task V-14) 2026-11-08: Deployed `yara-forge-core.yar` (3,765 rules, SHA256 `cf07261b...bf1bd8`) and `zonqa-control.yar` (rule `ZonQA_Benign_Marker`, SHA256 `2b7cbfc0...`) to `/var/lib/clamav/` on lab clamd (`admzon@100.96.250.245`). Pre-change backup verified. Socket reload `zRELOAD\0` completed in 8.0s with 0 errors; signature count 3,628,258 -> 3,632,024 (+3,766); RSS delta +110.5 MiB; clamdscan & workstation tunnel 13310 confirmed detection `YARA.ZonQA_Benign_Marker.UNOFFICIAL FOUND`; negative controls clean. Single-command rollback documented in [yara-live-2026-11-08.md](ops/yara-live-2026-11-08.md). Branch `vic/yara-live`. No merge.
- YARA-Forge Core SUSP_ pruning live on lab (Task V-15) 2026-11-08: Per Zon approval following Q-12 FP finding on Windows PowerShell.evtx, updated `filter_yara_clamav.py` to reject `heuristic_susp_rule` (132 rules rejected; 3,678 rules accepted, net delta: -87). Regenerated `yara-forge-core-clamav.yar` (SHA256 `f341d83a...0ce6`). Live replaced `/var/lib/clamav/yara-forge-core.yar` (rollback copy preserved at `~/zon-feedlab/rollback/`), owner clamav 0644, socket `zRELOAD\0` reloaded to 3,631,937 signatures (-87) in 8.0s, 0 errors. PING `PONG`, tunnel 13310, and positive control PASS. Details in [yara-live-2026-11-08.md](ops/yara-live-2026-11-08.md) and [yaraforge-provenance.md](feeds/yaraforge-provenance.md). Branch `vic/yara-live`. No merge.
- Task V-22 Integration & Alignment (2026-11-09): Merged `origin/vic/feed-yaraforge`, `vic/engine-ops`, `vic/engine-hardening`, and `vic/yara-live` into `vic/v22`. Changed icon buttons (`.theme-toggle`, `.server-status`) from 44px to 36px with 18px icons, retaining scan card vertical alignment. Migrated tunnel supervisor (`zon-tunnel.ps1`) and log to `%LOCALAPPDATA%\ZonEndpoint\tunnel\`, auto-cleaned root appdata files; live reinstalled `ZonEndpointTunnel` scheduled task and verified `zPING` -> `PONG` on `127.0.0.1:13310`. Aligned `MAX_FILE_BYTES` in `scanner.rs` to 25 MiB (matching clamd `StreamMaxLength 25M`) with dedicated unit test. Quality gates: 63/63 Rust tests PASS, cargo fmt PASS, cargo clippy -D warnings PASS, npm run build PASS, 19/19 native mock tests PASS. Branch `vic/v22`. No merge to main, no tag.
- Task V-23 Release v0.2.0 (2026-11-09): Version bumped 0.1.0 -> 0.2.0 in `package.json`, `package-lock.json`, `tauri.conf.json`, `Cargo.toml`, and `Cargo.lock`. Added `CHANGELOG.md` for 0.2.0. Verified quality gates: `npm ci` PASS, `npm test` 19/19 native mock PASS, `cargo test` 63/63 PASS, `npm run build:windows` PASS. Packaged release artifacts at `E:\Zonendpoint-tools\artifacts\v0.2.0\` (`zonendpoint.exe`, `Zonendpoint_0.2.0_x64-setup.exe`, `SHA256SUMS.txt`). Branch `vic/release-0.2.0`. No tags, no main merge, no installer execution.
- Task V-25 Lab YARA Decoupling & Automated Daily Updater (2026-11-09): Zon approved both server changes executed and verified on lab `admzon@100.96.250.245`:
  1. **Decoupled ZonQA Test Control Rule**: Moved `/var/lib/clamav/zonqa-control.yar` to `/opt/zonendpoint/qa/zonqa-control.yar`. Clamd reloaded with zero downtime (signatures: 3,631,937 -> 3,631,936); verified clamd direct scan returns clean `stream: OK` on marker; QA isolated execution verified via `clamscan -d /opt/zonendpoint/qa/zonqa-control.yar` returning `YARA.ZonQA_Benign_Marker.UNOFFICIAL FOUND`.
  2. **Automated Daily YARA-Forge Core Updates**: Deployed `/usr/local/bin/zonendpoint-yara-update.py`, `/usr/local/bin/zonendpoint-yara-rollback.sh`, and systemd unit/timer `zonendpoint-yara-update.timer` firing nightly between 20:00 and 22:00 UTC (03:00 - 05:00 AM ICT, randomized delay 7200s). Enforces GitHub API release check, core zip download, `filter_yara_clamav.py` (SUSP_ excluded), temporary DB pre-flight checks (reasonable rule count, benign clean, ZonQA marker detected), atomic file replacement, backup retention, zero-downtime clamd reload, and PING verification.
  3. **Verification**: Manual run upgraded ruleset to release `20261108` (3,676 accepted rules, active file SHA256 `fdbf84cff5e50a274f8e53476f22cff38c06bb65dc38fdeae4ef5d689433d6cc`, total signatures 3,631,934, clamd PONG verified). Immediate second manual run confirmed idempotency (exited in <1s without download). Invariants strictly maintained: clamd on 127.0.0.1:3310, PasswordAuthentication yes preserved, timers active. Full operations report: [yara-daily-update.md](ops/yara-daily-update.md). Branch `vic/yara-daily`. No merge.

## Tasks V-33, V-34 & V-35 — MalwareBazaar (abuse.ch) Server Feed & clamd Ingestion (2026-11-10)
- Branch: `vic/malwarebazaar` (base `0979f5f`).
- Implemented `scripts/feeds/zonendpoint-malwarebazaar.py` with commands: `check`, `fetch-recent`, `export-hsb`, `test-hsb`, and `deploy-hsb`.
- Key security invariants: Auth-Key read strictly from `/etc/zonendpoint/abusech.key` (root:root 0600), never logged, output, or passed in argv; header `Auth-Key` authentication; no malware binary downloads (`get_file` forbidden); no sample uploads.
- Hard rate limit: maximum 3 requests/minute and 500 requests/day persisted on disk (`/opt/zonendpoint/malwarebazaar/rate_limit.json`); fail-fast on 429, invalid key, or blocked account (no continuous retry); bounded backoff for transient errors.
- SQLite storage in `/opt/zonendpoint/malwarebazaar/malwarebazaar.db` with ACID transactions and rollback on partial errors; SHA-256 normalized to 64 lowercase hex, invalid rows dropped, duplicates merged.
- Manual execution only: NO systemd timer/cron. Proposed bootstrap/backfill options (CSV/dump export) documented and awaiting Zon's approval.
- ClamAV hash signature (.hsb) export verified via `clamscan -d` on temporary DB with 0 False Positives on benign system files; live clamd on `/var/lib/clamav/` preserved untouched (127.0.0.1:3310 PONG verified).
- Scope: Zon confirmed 2026-11-10 strictly for non-commercial use on 1–2 personal machines (corporate use requires reviewing abuse.ch / Spamhaus terms).
- Quality gate: Automated test suite `scripts/feeds/test_zonendpoint_malwarebazaar.py` expanded per Q-22 and V-35 to 19/19 PASS on lab Ubuntu 26.04 (timeout, user_blacklisted, 500/day persistence, DB rollback checksum, empty DB abort, signature naming format, pre-flight failure halt, reload failure automatic rollback, first-deploy cleanup, signature count delta verification).
- Deployed `/usr/local/bin/zonendpoint-malwarebazaar.py` and `/usr/local/bin/zonendpoint-malwarebazaar-rollback.sh` on lab host `100.96.250.245`.
- Live Execution Verification (Task V-34):
  - Zon entered Auth-Key (48 chars) into `/etc/zonendpoint/abusech.key` (`0600 root:root`).
  - Loki ran initial `check` (1 request, status ok, 100 items).
  - Vic ran live `fetch-recent` Run 1: 100 received, 100 valid, 100 inserted, 0 updated, 0 dropped, total 100 in DB.
  - Vic ran live `fetch-recent` Run 2 (dedup proof): 100 received, 100 valid, 0 inserted, 100 updated, 0 dropped, total in DB 100 (0 duplicate rows created; idempotency confirmed).
  - Total requests used: 3/500 daily budget (2 requests used in Task V-34, ≤ 5 task limit).
- Task V-35 Live clamd Ingestion & Rollback (Zon approved 2026-11-10):
  - Server-only architecture: server calls abuse.ch API and persists to SQLite; 2–3 desktop clients benefit transparently through live clamd via SSH tunnel (`127.0.0.1:13310 -> 127.0.0.1:3310`, `zINSTREAM`). Client requires zero changes and zero keys.
  - Ingested 100 normalized hashes into active clamd database `/var/lib/clamav/malwarebazaar.hsb` (`clamav:clamav 0644`).
  - Staging `.hsb` generation -> temporary DB verification with base DBs (0 errors, benign clean) -> backup (`/var/backups/clamav-malwarebazaar/`) -> atomic replace -> zero-downtime `zRELOAD\0` -> PING verified (`PONG`) -> clamd signature count delta verified.
  - Live clamd signatures: **7,421,508 -> 7,421,608** (+100 signatures, exactly matching the 100 hashes).
  - Live rollback verified: tested standalone `/usr/local/bin/zonendpoint-malwarebazaar-rollback.sh` restoring backup, reloaded clamd, verified PONG; redeployed active signatures successfully.
  - Real API requests used in Task V-35: **0** (used existing SQLite hashes).
  - Schedule: **NO AUTOMATED SCHEDULE** (updates remain strictly manual).
- Task V-36 Full Dump Bootstrap & clamd Ingestion (Zon approved 2026-11-11):
  - Rate limit policy updated to 3 requests/hour (sliding 60m window = 3600s) and 400 requests/24h on disk (`/opt/zonendpoint/malwarebazaar/rate_limit.json`). Fail-fast without waiting or retries.
  - Implemented manual command `fetch-full`: streamed download from abuse.ch export endpoint (`/v2/files/exports/{auth_key}/full.csv.zip`), validated zip CRC (`testzip()`), uncompressed size check (4 GB limit), atomic SQLite transaction with `BEGIN IMMEDIATE ... COMMIT`, deduplication against existing rows (preserving original `source` and `updated_at`). Live dump: 1,171,240 rows parsed, 1,171,140 inserted, 100 existing preserved; total DB rows: 1,171,240.
  - Pre-flight safeguards: available host RAM < 3.0 GB halts deployment; clamd reload timeout > 120s aborts, triggers rollback, and alerts Loki. Telemetry collects clamd RSS before/after and exact reload duration.
  - Wildcard hash signature support: `<sha256>:*:MalwareBazaar.<sig>.UNOFFICIAL:73` for export rows without file size.
  - Quality gate: Automated mock test suite expanded to 26/26 PASS on lab Ubuntu 26.04.
  - Live clamd deployment (`deploy-hsb`): 1,171,240 signatures loaded into active file `/var/lib/clamav/malwarebazaar.hsb` (kích thước thực tế: 121.547.198 bytes / ~121 MB). Live signatures increased from **7,421,608** to **8,592,748** (+1,171,140, exact match). clamd RAM RSS: trước reload ~2,1 GB (2.127 MB), sau reload ~2,3 GB (2.249–2.302 MB, đỉnh reload VmHWM ~3,6 GB). Reload duration: ~26–28s (well within 120s limit). Available host RAM: 11.65 GB (> 3.0 GB threshold), sau reload ~12,0 GB. clamd PONG verified on `127.0.0.1:3310`.
  - Real requests consumed in Task V-36: **2 requests** (within $\le 3$ limit).
  - Preserved engine invariants: clamd loopback 3310, `clamd.conf` and `freshclam.conf` unchanged, no systemd timer/cron.
- Task V-37 Documentation & Docstrings Polish (per Q-25 review, 2026-11-11):
  - Fixed .hsb file size in docs/handoff to actual 121,547,198 bytes (~121 MB / ~116 MiB), correcting earlier ~88 MB estimate.
  - Recorded clamd reload performance: reload duration ~26–28s, clamd RAM RSS ~2.3 GB (2,249–2,302 MB, peak VmHWM ~3.6 GB during reload).
  - Fixed docstrings in `scripts/feeds/zonendpoint-malwarebazaar.py` (lines 16 and 89) from "3 requests/minute, 500/day" to "3 requests/hour (sliding 60m) and 400 requests/24h" to match code constants and RateLimiter.
  - Added strict security warnings in ops runbook and status: NEVER enter or paste Auth Key into shell prompt (prevents saving to `~/.bash_history`), recommend `HISTCONTROL=ignorespace`. Emphasized that `~/.bashrc` on lab must NOT be edited by agents (per Zon's decision).
  - Mock test suite verified (26/26 PASS).
- Operations runbook: [malwarebazaar.md](ops/malwarebazaar.md); handoff: [malwarebazaar-vic-to-qeen.md](handoffs/malwarebazaar-vic-to-qeen.md).


## Limits / next acceptance
PASS is scoped to observed core/native and unsigned NSIS current-user tests, not production readiness. Exact EICAR through Windows remains BLOCKED by Microsoft Defender (Operational 1116/1117, quarantine); app reported error 225, never CLEAN. ClamAV EICAR on Ubuntu and native positive detection via temporary harmless lab signature passed; signature removed/cleanup verified.

### Items requiring Zon's manual test:
1. **File / Folder dialog pick**: Interacting with native Windows File/Folder picker dialogs (automated test is blocked due to Windows UIPI security boundaries).
2. **Auto-return after File / Folder**: Verify target path automatically clears and primary button returns to "Quick Scan" upon scan completion. (Zon tested RC: OK).
3. **Single "All Good" on a file**: Manually scan a single benign file and verify the single-line "All Good" card with green checkmark renders cleanly without error or extra list rows. (Zon tested RC: OK).

Next acceptance: Qeen independent verification of integrated package on branch `vic/integrate-ui-cache` (`69d8577`).

Lab: admzon@100.96.250.245; clamd loopback 3310; normal Windows test tunnel 127.0.0.1:13310. All testing preserved Defender/Sophos policies; no exclusions, public uploads or Internet-exposed clamd.

## Document map / later decisions
[Scope](v1.0.0-scope.md) â†’ [roadmap](roadmap.md) â†’ [technical tasks](implementation-plan.md). [Notebook reconciliation](notebook-review-2026-11-07.md) records nine source groups, policy conflicts and verified primary references; [architecture](zonendpoint_system_architecture_specs.md#12-current-implementation-and-notebook-reconciliation--2026-11-07) has the current-code addendum; [gateway/licensing](zonendpoint_ed25519_secure_gateway_licensing_spec.md) is still Planned/Not implemented. Old [architecture review](architecture-review.md) is explicitly historical.

Open later decisions: pilot distribution/signing, measured budgets, selected feed entitlement/false-positive policy, secure transport/key lifecycle, SQLCipher recovery and AI provider/data policy. Notebook suggestions alone authorize no account setup, external transmission, firewall/AV changes or feature implementation.

## Adaptive client concurrency (J-01) — 2026-11-11
Code snapshot `bab14a0f936670cb58de302fc13352cb01f4f76a` on `jacky/adaptive-concurrency`, based on `origin/main` `0979f5fab4ea59912c5ad59a21f01c605f93eaab`. Folder traversal now runs bounded scan workers; the shared scanner limiter selects concurrency from `available_parallelism` and physical RAM (8 GiB => 2, 16 GiB => 3, 32/64 GiB => 4, capped at 4), allows a lower `max_scan_concurrency` cap in `%LOCALAPPDATA%\Zonendpoint\config.json`, and backs off by one on clamd timeout/busy responses without going below 1. Scanner protocol/configuration unchanged.

Measured on this 8 GiB / 4 logical CPU client against the already-running loopback tunnel and `C:\Program Files` (12,886 files; 12,875 CLEAN, 3 ERROR, 8 SKIPPED; incomplete both runs): baseline first scan 488.6 s, cache scan 239.0 s; updated first scan 613.6 s, cache scan 107.0 s. Client peak working set was 8.3/8.0 MiB baseline and 11.2/9.1 MiB updated; cumulative process CPU was 248.7/207.6 s baseline and 259.0/209.9 s updated. This single miss sample regressed while the cache hit improved 2.24x; do not claim a first-scan speedup. Lab clamd CPU/RSS was NOT RUN: SSH key access was denied and host key verification was unavailable. See `E:\Zonendpoint-tools\artifacts\j01\jacky-report.md` and [J-01 handoff](handoffs/adaptive-concurrency-jacky-to-qeen.md).

**Next acceptance:** Qeen independently verified code snapshot `bab14a0f936670cb58de302fc13352cb01f4f76a` in QA report Q-26 (PASS).

**Push status:** DONE (Loki pushed tip `81984ad`; verified in Q-26 PASS with 3-run measurements: first scan 415→194 s, cache 209→109 s; merged into main in Task V-38).


## Task V-38 — Integration of J-01 (Adaptive Concurrency) & MalwareBazaar to main (2026-11-11)
- **Approval**: Zon approved 2026-11-11 ("Merge cả hai").
- **Merged Branches**:
  1. `origin/jacky/adaptive-concurrency` (tip `81984ad`, code `bab14a0`, Q-26 PASS): bounded folder parallelism, adaptive concurrency based on RAM/CPU, client cap in `config.json`, clamd pressure backoff.
  2. `origin/vic/malwarebazaar` (tip `4f13d3f`, code `0109d5a`, Q-22..Q-25 PASS): server-only MalwareBazaar ingestion, full dump bootstrap (1,171,240 signatures deployed to `/var/lib/clamav/malwarebazaar.hsb`), rate limiting (3/h, 400/24h), rollback script.
- **Security Hardening (Code Snapshot `de51ffe`)**:
  - Sanitized sensitive Auth-Key in abuse.ch export download URL: `/v2/files/exports/<AUTH-KEY>/full.csv.zip`.
  - Added `SensitiveKeyFilter` logging filter and `mask_auth_key()` helper to `scripts/feeds/zonendpoint-malwarebazaar.py`.
  - Replaced raw Auth-Key in all exceptions, logger records, `URLError`, and `HTTPError` bodies with `<AUTH-KEY>`. Suppressed chained exception raw URLs via `from None`.
  - Added unit tests `test_27` and `test_28` in `scripts/feeds/test_zonendpoint_malwarebazaar.py` (28/28 PASS, 0 real requests).
- **J-01 handoff updated**: recorded push status DONE (pushed by Loki) and Q-26 measurements (first scan 415→194 s, cache 209→109 s).
- **Quality Gates**: `cargo fmt --check` PASS, `cargo clippy --all-targets -- -D warnings` PASS, `cargo test` 68/68 PASS (49 lib + 1 bin + 18 resilience), `python test_zonendpoint_malwarebazaar.py` 28/28 PASS.
- **Main SHA**: Fast-forwarded from `0979f5f` to `de51ffe` (pushed to origin). Handoff: [merge-v38-vic-to-qeen.md](handoffs/merge-v38-vic-to-qeen.md).


## FEED-LAB YARA Forge Staging Evaluation (V-10) â€” 2026-11-08
Branch `vic/feed-yaraforge` (`E:\Zonendpoint-vic-feed`), base `ed1088c`. Staging evaluation of YARA-Forge Core (`20261103`, 5,133 rules, personal/non-commercial defensive scope) in isolated lab (`~/zon-feedlab/`). Filtered via `scripts/feeds/filter_yara_clamav.py`: 3,765 rules accepted, 1,368 rejected (reasons: empty meta 577, big-endian funcs 374, engine load/subpattern 165, imported modules 174, unsupported modifiers 53, syntax/hex 8, ext vars 8, global/private 5, no condition 4). ClamAV 1.5.4 dynamic load 100% clean (3,765 rules, 0 errors, 0 warnings). Memory overhead +63.4 MiB RSS (YARA-only 95.0 MiB, swap 0MB); load delta +0.14s. Positive control `ZonQA_Benign_Marker` detected correctly (`YARA.ZonQA_Benign_Marker.UNOFFICIAL FOUND`, exit 1). FP scan on `/etc` (889 files), `/usr/bin` (873 files), and `/usr/lib` (27,405 files): 0 False Positives across 29,167 system files. Proposed live enablement and <1min rollback plan under "Cáº¦N ZON DUYá»†T". Evidence at `E:\Zonendpoint-tools\artifacts\feedlab\yaraforge\`; report `docs/qa/feedlab-yaraforge-2026-11-08.md`. Live lab clamd (PID 2245), freshclam (PID 2388), configs, and tunnel 13310 preserved untouched. No live change, no merge.
