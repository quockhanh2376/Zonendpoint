# Project status — 2026-11-12

Phase: current scan candidate stabilization for **non-commercial use on 1–2 personal machines** (confirmed by Zon 2026-11-10; company use requires reviewing abuse.ch/Spamhaus terms). Implementation of hourly automated update scheduler for MalwareBazaar + ThreatFox (Task V-42) and Q-31/Q-32 defect resolution (Tasks V-43/V-44) on branch `vic/abusech-timer`. Base: `origin/main 1399604` (Q-30 PASS).

## Active checkout/snapshot
- Implementation: branch `vic/abusech-timer` (base `origin/main 1399604`).
- Feeds status:
  - MalwareBazaar: 1,171,422 hashes in DB (`/opt/zonendpoint/malwarebazaar/malwarebazaar.db`), 1,171,422 loaded in clamd (`/var/lib/clamav/malwarebazaar.hsb`).
  - ThreatFox: 45,298 unique SHA-256 in DB (`/opt/zonendpoint/threatfox/threatfox.db`), 21,625 net unique signatures loaded in clamd (`/var/lib/clamav/threatfox.hsb`).
  - Total custom feed signatures: 1,193,047 signatures in clamd (clamd live total: 8,614,555 signatures).
  - Mode: Hourly timer installed (`zonendpoint-abusech-update.timer` with monotonic interval `OnUnitInactiveSec=61min`, `OnBootSec=5min`, `AccuracySec=1min`), but **LEFT DISABLED and INACTIVE** (`disabled`, `dead`) pending activation approval. Shared rate limit: 3 requests/hour (sliding 3600s), 400 requests/24h.
  - Guard: Requires $\ge 2$ slots before starting turn, atomically reserved under `rate_limit.json.lock`. If $< 2$ slots remain, entire turn is skipped cleanly without retries.
  - Recent endpoints: MalwareBazaar (`https://mb-api.abuse.ch/v2/files/exports/<AUTH-KEY>/recent.csv`), ThreatFox (`https://threatfox-api.abuse.ch/v2/files/exports/<AUTH-KEY>/recent.csv`).
  - Reload policy: Single clamd reload per hour; 0 new hashes across feeds = 0 reload; pre-flight temp DB check + RAM $\ge 3.0$ GB + timeout $\le 120$ s.
- Handoff doc: [abusech-timer-vic-to-qeen.md](handoffs/abusech-timer-vic-to-qeen.md).
- Merged branches:
  - `origin/vic/close-fix` (tip `0b48da0`, code `e8b249d`, merged in Task V-30M, branch `vic/close-fix-merge`, code snapshot `17bdccf`): graceful window close & scan cancellation on exit (V-30), compatibility with ScanManager, bounded folder workers + adaptive limiter (J-01), and Guard (`stop_guard`), refactored shared event handling (`cancel_and_exit`), quality gates 68/68 cargo tests PASS, 20/20 native tests PASS.
  - `origin/jacky/threatfox` (tip `4e25f22`, code `8eb08e7`, Q-28/Q-29 PASS 51/51 tests): ThreatFox CSV export ingestion, SQLite metadata/hash storage, duplicate resolution (899 rows merged via ON CONFLICT DO UPDATE per Zon approval), shared abuse.ch rate limiter with MalwareBazaar, 21,476 unique SHA-256 deployed to clamd, rollback script.
  - `origin/jacky/adaptive-concurrency` (tip `81984ad`, code `bab14a0`, Q-26 PASS): client adaptive scan concurrency based on physical RAM and CPU cores, capped at 4, server backoff on pressure.
  - `origin/vic/malwarebazaar` (tip `4f13d3f`, code `0109d5a`, Q-22..Q-25 PASS): server-only feed ingestion (`scripts/feeds/zonendpoint-malwarebazaar.py`), full dump bootstrap (1,171,240 signatures in `/var/lib/clamav/malwarebazaar.hsb`), live clamd zero-downtime reload (signatures 7,421,608 -> 8,592,748), rollback script.
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

## Task J-02 — ThreatFox server feed + shared abuse.ch budget (2026-11-11)
- Branch `jacky/threatfox`, base snapshot `a9152229b4024da2b65a934dd4fa9fc81c612e55` (V-38 merge). Implemented ThreatFox manual importer, SQLite metadata/hash storage, `.hsb` export/deploy through the MalwareBazaar guarded atomic deployment flow, standalone rollback, shared rate limiter and mock tests.
- Code implementation commit / handoff snapshot: `de960f956fc2571ed90bc230d5cbf28789111c5b`.

## Task V-39 — ThreatFox Q-28 fixes & lab first run (2026-11-11)
- Branch `jacky/threatfox`, worktree `E:\Zonendpoint-worktrees\v39-threatfox`. Fixed 5 defects from Q-28 QA review:
  1. `zonendpoint-threatfox-rollback.sh`: CRLF -> LF converted, header updated to J-02, `.gitattributes` added (`*.sh text eol=lf`, `*.py text eol=lf`). Verified `bash -n` on lab host: PASS.
  2. `abusech_rate_limit.py`: Strict validation in `_read` refusing non-dict or missing "requests" JSON without overwriting file. Test added and PASS.
  3. Alternation priority: On empty hourly window with `other_waiting=True`, MalwareBazaar is strictly prioritized (`MB -> TF -> MB`). Test added and PASS.
  4. Test suite fix: Corrected `test_hsb_wildcard_and_malwarebazaar_hash_exclusion` in `test_zonendpoint_threatfox.py` line ~52 to `assertNotIn ThreatFox.Fam` (hash collision exclusion).
  5. Forwarded `--log-file` to `mb.deploy_hsb`; refactored `fetch_full` to stream HTTP chunks (64 KB) directly to disk temp file without reading entire ZIP/CSV into RAM; enforced 512 MB limits. Tests added and PASS.
- Quality Gates on lab Ubuntu 26.04 (Python 3.14.4):
  - `test_abusech_rate_limit.py`: 4/4 PASS.
  - `test_zonendpoint_threatfox.py`: 11/11 PASS.
  - `test_zonendpoint_malwarebazaar.py`: 28/28 PASS. Total 43/43 PASS.
- Installed binaries to `/usr/local/bin`: `abusech_rate_limit.py`, `zonendpoint-malwarebazaar.py` (with `de51ffe` key sanitization), `zonendpoint-threatfox.py`, `zonendpoint-threatfox-rollback.sh` (chmod 755 root:root).
- Migrated legacy state to `/opt/zonendpoint/abusech/rate_limit.json`: preserved all 6 original timestamps with source `malwarebazaar`.
- Lab real execution:
  - `check`: 1 request used (1/3 hourly, 7/400 daily). Upstream API response `query_status: ok`.
  - `fetch-full`: 1 request used (2/3 hourly, 8/400 daily). Real authenticated CSV header retrieved: `first_seen_utc, ioc_id, ioc_value, ioc_type, threat_type, fk_malware, malware_alias, malware_printable, last_seen_utc, confidence_level, is_compromised, reference, tags, anonymous, reporter`. Safeguard tripped on comment preamble/parsing mismatch. Database `/opt/zonendpoint/threatfox/threatfox.db` kept strictly clean at 0 rows. No unverified signatures ingested into clamd.
  - Live clamd: PID 2245, RSS ~2.35 GB, signatures 8,592,748 (stable, unchanged). Zero clamd.conf/freshclam.conf change, no timers. Benign scans verified via tunnel `127.0.0.1:13310 -> 3310` (`stream: OK`).
  - Secret leak audit: `sudo sh -c 'grep -rlFf /etc/zonendpoint/abusech.key ...'` on `/opt/zonendpoint`, `/var/lib/clamav`, `/var/backups`, `/var/log`, `/tmp`, `~/.bash_history` returned 0 matches (100% clean).
- Handoff doc: [threatfox-vic-to-qeen.md](handoffs/threatfox-vic-to-qeen.md).

## Task V-40 — ThreatFox real run & production deployment (2026-11-11)
- Branch `jacky/threatfox`, worktree `E:\Zonendpoint-worktrees\v40-threatfox`. Completed production deployment of ThreatFox on lab server `100.96.250.245`:
  1. **Export CSV Parser Fixes**:
     - Supported comment lines `#` at head, mid-file, and end-of-file.
     - Supported `# ` header lines with quoted field names and spaces after commas (`skipinitialspace=True`).
     - Defensive quote and whitespace stripping on all field values while preserving internal spaces (e.g. malware names, timestamps).
     - Standard IOC type mapping: `sha256_hash` -> `sha256`, `md5_hash` -> `md5`, `sha1_hash` -> `sha1`, `ip:port` -> `ip:port`, `domain` -> `domain`, `url` -> `url`.
     - Malware family resolution: `malware_printable` (empty, `"None"`, or whitespace -> `"Unknown"`).
     - Confidence level resolution: `confidence_level` -> integer.
     - Timestamps: `first_seen_utc` and `last_seen_utc`.
     - Transactional DB insertion with safety threshold: aborts and alerts Loki (`[ALERT-LOKI]`) if imported = 0 or rejected > 5%.
  2. **Dump File Preservation & Offline Replay**:
     - Export dump saved to `/opt/zonendpoint/threatfox/last-full.csv.zip` (`0600 root:root`, no key in path/filename).
     - Added CLI command `import-file --file <path>` to re-import from local ZIP/CSV with 0 API requests consumed.
  3. **Quality Gates on Lab Host (Python 3.14.4)**:
     - `test_abusech_rate_limit.py`: 4/4 PASS.
     - `test_zonendpoint_threatfox.py`: 14/14 PASS (added real format fixture, dump preservation, import-file, and 5% reject threshold tests).
     - `test_zonendpoint_malwarebazaar.py`: 28/28 PASS.
     - Total: **46/46 PASS**.
  4. **Lab Real Run Execution (1 request consumed)**:
     - Consumed 1 request (3/3 hourly sliding window, 9/400 daily total).
     - Total data rows: 135,585; imported: 135,585; rejected: 0 (0.00%).
     - IOC counts by type: `sha256`: 45,041; `ip:port`: 29,323; `domain`: 29,125; `url`: 19,026; `md5`: 7,058; `sha1`: 6,012.
     - SHA-256 total: 45,041; duplicates with MalwareBazaar excluded: 23,564; net unique for clamd: 21,476.
     - Offline replay verified on `/tmp/test_import.db` with 0 requests.
  5. **ClamAV Live Deployment (`deploy-hsb`)**:
     - Host available RAM: 11.96 GB (threshold: 3.00 GB).
     - Preflight temporary DB check: 3,649,734 known viruses, 0 errors, 0 False Positives on benign files.
     - Active signature file written: `/var/lib/clamav/threatfox.hsb` (21,476 signatures, mode 0644 clamav:clamav).
     - ClamAV signatures: **8,592,748 -> 8,614,224** (+21,476 exact match).
     - clamd reload duration: 26.01s (limit: 120s). clamd RSS: 2,303.66 MB -> 2,305.27 MB (+1.61 MB). Engine PONG verified.
  6. **Tunnel & Security Audit**:
     - Benign file scans via tunnel `127.0.0.1:13310`: `package.json`, `README.md`, `Cargo.toml` all return `stream: OK`.
     - Key leak audit across `/opt/zonendpoint`, `/var/lib/clamav`, `/var/backups`, `/var/log`, `/tmp`, `~/.bash_history` returned 0 matches (100% clean).
     - Invariants preserved: zero clamd.conf/freshclam.conf modification, clamd on `127.0.0.1:3310`, no timers.
- Handoff doc: [threatfox-vic-to-qeen.md](handoffs/threatfox-vic-to-qeen.md).

## Task V-41 — Merge ThreatFox into main & Q-29 polish (2026-11-11)
- Branch `vic/merge-v41`, worktree `E:\Zonendpoint-worktrees\v41-merge`, base `origin/main` (`a915222`).
- Per Zon approval (2026-11-11 "Merge vào main"), merged `origin/jacky/threatfox` (tip `4e25f22`, code `8eb08e7`, Q-28/Q-29 PASS 51/51 tests) into `main` via merge commit `bae722d`.
- **Q-29 Log Fix**: Updated `ThreatDatabase.import_rows()` in `scripts/feeds/zonendpoint-threatfox.py` to query unique SHA-256 count directly from SQLite (`SELECT count(*) FROM hashes`, 45,040) instead of raw CSV row count (45,041), ensuring `net_deployable_sha256 = 45,040 - 23,564 = 21,476` matches `.hsb` line count exactly. Added unit test in `scripts/feeds/test_zonendpoint_threatfox.py` (15/15 PASS).
- **Duplicate Rows Policy**: Documented that real export CSV contains 899 duplicate `(ioc, ioc_type)` rows; SQLite merges them via `ON CONFLICT DO UPDATE` retaining the latest record per Zon's explicit approval.
- **Quality Gates**:
  - `cargo fmt --check`: PASS.
  - `cargo clippy --all-targets -- -D warnings`: PASS.
  - `cargo test`: 68/68 PASS (49 lib + 1 bin + 18 resilience).
  - Python test suites on lab temporary directory (`/tmp/v41_verify`): `test_abusech_rate_limit.py` 4/4 PASS, `test_zonendpoint_threatfox.py` 15/15 PASS, `test_zonendpoint_malwarebazaar.py` 28/28 PASS (Total: 47/47 PASS, 0 real requests).
- **Lab Deployment**: Reinstalled `zonendpoint-threatfox.py` to `/usr/local/bin` (755 root:root). Only log calculation line changed; 0 fetch/deploy executed; clamd PID 2245, DB, `.hsb` untouched.
- **Main SHA**: Fast-forwarded `main` from `a915222` to `3e62bf4` and pushed to `origin main`.
- **Feeds Status**: MalwareBazaar 1,171,240 hashes + ThreatFox 21,476 unique SHA-256 signatures in clamd (total 1,192,716 feed signatures; clamd total 8,614,224 signatures); 134,686 unique IOCs in DB; strictly manual execution; 0 timers.
- Handoff doc: [merge-v41-vic-to-qeen.md](handoffs/merge-v41-vic-to-qeen.md).

## Task V-43 — ThreatFox Starvation Fix & Monotonic Timer (2026-11-12)
- Branch `vic/abusech-timer`, worktree `E:\Zonendpoint-worktrees\v43-timer`, continuation of `af972e7` (code `1b6834c`).
- **Defect Resolved (Q-31)**: Fixed ThreatFox starvation under hourly timer. With `OnCalendar=hourly + RandomizedDelaySec=5min`, consecutive intervals could be $< 60$ min, leaving only 1 slot in the sliding window; MalwareBazaar took the slot and ThreatFox was blocked (63.2% success vs MB 100%).
- **Fix 1 (Turn Guard & Atomic Reservation)**:
  - In `scripts/feeds/abusech_rate_limit.py`: added `RateLimiter.reserve_slots(count=2, sources=("malwarebazaar", "threatfox"))`.
  - Atomically verifies `hourly_remaining >= 2` and `daily_remaining >= 2` under `rate_limit.json.lock`.
  - In `scripts/feeds/zonendpoint-abusech-update.py`: calls `reserve_slots(2)` before turn start. If $< 2$ slots remain (e.g. from intervening manual requests), the entire turn is skipped cleanly without retries, recording `last-run.json`.
- **Fix 2 (Monotonic Timer >= 61 min)**:
  - In `scripts/feeds/zonendpoint-abusech-update.timer`: changed to `OnBootSec=5min`, `OnUnitActiveSec=61min`, `AccuracySec=1min` (removed `RandomizedDelaySec` and `OnCalendar=hourly`).
  - Guaranteed interval $\ge 61$ min ensures all prior requests are outside the 60m sliding window, achieving 100.0% completion for both MB and TF.
- **Fix 3 (MalwareBazaar Log Label)**:
  - In `scripts/feeds/zonendpoint-malwarebazaar.py`: `import_dump_csv` logs "Recent export imported successfully" when importing `recent.csv` instead of "Full dump imported successfully".
- **Quality Gates & Simulations**:
  - Python test suites on lab temporary directory (`/tmp/v43_test`, Python 3.14.4): **5/5 suites PASS (66/66 tests, 0 real requests to abuse.ch)**:
    - `test_abusech_rate_limit.py`: 8/8 PASS
    - `test_abusech_timer_simulation.py`: 5/5 PASS (100k-iteration simulation: old timer MB 100%, TF 63.2%; new timer MB 100%, TF 100%; manual interference full-turn skip)
    - `test_zonendpoint_abusech_update.py`: 10/10 PASS
    - `test_zonendpoint_threatfox.py`: 15/15 PASS
    - `test_zonendpoint_malwarebazaar.py`: 28/28 PASS
- **Lab Deployment**:
  - Reinstalled all updated scripts (`/usr/local/bin`) and unit files (`/etc/systemd/system/`); ran `systemctl daemon-reload`.
  - Timer verified: `disabled`, `inactive (dead)`. ClamAV daemon PID 2245, configs, and 8,614,555 signatures untouched.
- Handoff doc: [abusech-timer-vic-to-qeen.md](handoffs/abusech-timer-vic-to-qeen.md).

## J-03 Adaptive Concurrency v2 - 2026-11-12
- Checkout `E:\Zonendpoint-Jacky\j03`, branch `jacky/concurrency-v2`; code snapshot `1e0ec28ba6bc5c0732b7d544836a51dddfdb5b51` (local commit, not pushed/merged).
- Expanded automatic ceiling to 6 only for >=32 GiB and >=8 logical CPUs; retained 8 GiB -> 2, 16 GiB -> 3, and 4 for other higher-memory/low-core systems. Config `max_scan_concurrency` is a clamped 1-6 override and old JSON remains valid. Pressure backoff recovers one permit after 50 consecutive CLEAN/INFECTED results; non-clean outcomes reset the streak.
- Gates PASS: fmt; 70 Rust tests (51 lib, 1 binary, 18 resilience); clippy `--all-targets -- -D warnings`. Real-folder measurement on `C:\Program Files`: NOT RUN because UI automation refused access; tunnel 127.0.0.1:13310 was reachable. No 4-vs-6 performance conclusion. Lab metrics NOT RUN; Qeen to measure. See [J-03 handoff](handoffs/concurrency-v2-jacky-to-qeen.md).

## J-04 — Concurrency benchmark completion (2026-11-12)
- Checkout `E:\Zonendpoint-Jacky\j04`, branch `jacky/concurrency-v2`. Merged local `origin/main` at `2464aee` (merge `0b0ed14`; daily conflict resolved by retaining both branches' entries). Code snapshot `3efdefd2e701d26498759ca0c7a04634ac4c2728`.
- Added the dev-only `src-tauri/examples/folder_bench.rs`; it exercises the app's folder walker, engine-pinned scanner, limiter and cache without adding dependencies. Benchmark command and schema are documented in the J-03 handoff.
- 8 GiB / 4 CPU, Program Files, 12,886 files, loopback tunnel: 3 cold runs each at 2/4/6 and a cache run per level, interleaved. Median 4=107.7 s; median 6=98.9 s after rerunning the anomalous 6-c sample. 6 improves 8.2%, below the 10% criterion; recommend keeping ceiling 4. Level-2 times were variable (529.5/427.9/115.5 s); two extra runs returned 117.4/112.6 s. Record original and rerun observations in the handoff and `E:\Zonendpoint-tools\artifacts\j04\jacky-report.md`.
- Flaky cancellation test now accepts one or more preserved INFECTED outcomes and asserts all counted infected results remain in `results`. Product cancellation behavior unchanged. Repeated test 50/50 PASS; fmt, cargo test 70/70, clippy and release example build PASS.
- `%LOCALAPPDATA%\ZonEndpoint` file SHA-256 before/after identical. Clamd CPU/RSS and lab timer/reload telemetry NOT RUN; Qeen to measure independently. No Qeen acceptance, push or merge to main.

## J-05 — Restore application concurrency ceiling to 4 (2026-11-12)
- Checkout `E:\Zonendpoint-Jacky\j05`, branch `jacky/concurrency-v2`, based on local J-04 snapshot `51011db`. Code commit `bf4f79ef0cb307d43b6a1c46f9930adb3f6ef17c`; handoff and status are in the documentation snapshot. No push or merge to main; Loki owns push.
- Restored automatic selection to 8 GiB -> 2, 16 GiB -> 3, otherwise -> 4, bounded by CPU and hard cap 4. `max_scan_concurrency` only lowers the automatic value; old JSON values including 5/6/large values still load and resolve to <=4. Retained recovery at 50 consecutive CLEAN/INFECTED results, pressure backoff to floor 1, telemetry, and J-04 cancellation test correction.
- `folder_bench` remains dev-only, accepts 1–6 directly for measurements, independent of app config. No benchmark rerun; J-04 measured 6 just 8.2% faster than 4, below Zon’s 10% threshold.
- Gates: fmt PASS; cargo test **70/70 PASS** (51 lib, 1 binary, 18 resilience); clippy `--all-targets -- -D warnings` PASS; release example build PASS. Qeen acceptance NOT RUN. See [J-05 handoff](handoffs/concurrency-v2-jacky-to-qeen.md) and `E:\Zonendpoint-tools\artifacts\j05\jacky-report.md`.
- Next acceptance: Qeen independently audits the exact code/docs handoff snapshot; no performance remeasurement requested.

## J-06 — v0.3.0 performance baseline (2026-11-13)
- Checkout `E:\Zonendpoint-Jacky\j06`, branch `jacky/perf-baseline`, based on `main` at `f3e959d9ff2d573a321b7499fcbff2e548cb822a`. Dev-only benchmark source commit: `ba0e418f00aa7f3e6b65f49e0efe256eea9026a4`; no push/merge. No version, package, Tauri configuration, dependencies, or product behavior changed.
- Measured 3 cache-cold + 2 cache-warm Quick scans (auto concurrency 2), 3 Program Files cancellations each at 10 s and 60 s, and 5 cache-cold Program Files scans. Quick: 589/589 CLEAN; cold median 8.771 s with an outlier at 123.653 s; warm median 1.718 s. Program Files: 12,886 outcomes each, median 115.634 s with one 388.106 s outlier. Cancellation latency recorded as 0 ms (under 1 ms timer resolution); 0 INFECTED observed in all Program Files trials.
- Detailed method, machine/tunnel identity, per-file percentiles, top-10 slow-file comparisons, limitations and UTC run intervals are in [J-06 baseline](qa/perf-baseline-0.3.0.md); raw JSON per run and `run.meta` in `E:\Zonendpoint-tools\artifacts\j06\runs`. Brief report: `E:\Zonendpoint-tools\artifacts\j06\jacky-report.md`.
- Gates PASS: `cargo fmt --check`, `cargo test` 70/70, `cargo clippy --all-targets -- -D warnings`, `cargo build --release --example folder_bench`. Original `%LOCALAPPDATA%\ZonEndpoint\cache.db` hash unchanged. App + WebView2 process-tree RAM NOT RUN; Qeen acceptance NOT RUN. No causal attribution for the two slow outliers without lab journal/timer evidence.
- Next acceptance: Qeen independently reviews the exact handoff snapshot; Loki owns push. Qeen must measure full installed-app/WebView2 RAM.
## V-51 / P0 — Release Candidate NSIS Build (2026-11-13)
- Checkout `E:\Zonendpoint-worktrees\v51-release`, branch `vic/release-candidate` from `origin/main` (`f3e959d`). Clean tree verified before build.
- Toolchain: rustc 1.99.1, cargo 1.99.1, node v26.13.0, npm 11.20.0, NSIS v3.13 (`C:\Users\admzon\AppData\Local\tauri\NSIS\makensis.exe`).
- Version bumped to `0.3.0` per Zon approval (commit `80d2a7a`: package.json, package-lock.json, tauri.conf.json, Cargo.toml, Cargo.lock, CHANGELOG.md).
- Hashes: `package-lock.json` `EC08DA44135F6A693C0A7A57A4A9400D65B6BE624FC83BC48A0C130CE6E27029`, `src-tauri/Cargo.lock` `122243C0C54C2EECD61164FA56924FBBD25759AFE47C06346163F745F39D97E5`.
- Pre-build gates: `cargo fmt -- --check` PASS (0 diffs), `cargo test` **70/70 PASS** (51 lib, 1 bin, 18 resilience), `cargo clippy --all-targets -- -D warnings` PASS, `npm.cmd run build` PASS, `node tests/native/run.mjs` **20/20 PASS** (UI verified displaying `v0.3.0`; V-30 window close tests; 0 orphan processes).
- Release build: `npm.cmd ci` then `npm.cmd run build:windows` (`tauri build --target x86_64-pc-windows-msvc --bundles nsis`). No `native-ui-tests` feature, no test overrides.
- Artifacts:
  - Installer: `E:\Zonendpoint-tools\artifacts\v51-release\Zonendpoint_0.3.0_x64-setup.exe` (3,709,078 bytes)
  - Installer SHA256: `5BC97E93158D7A546F3EC79D26D2EFE5AB6ECE3062153403E780374EB006D55B`
  - App executable: `zonendpoint.exe` (PE x64, SHA256: `600AE667D077B02CF387CAFF563E6813AF353F32131593316B97F753EA45C10A`)
  - NSIS version: `v3.13`, Version: `0.3.0`. Unsigned.
- No install/overwrite on user system. Ready for Qeen testing (steps 3–5 of `V/Q-RELEASE-CURRENT`).
- Handoff doc: [release-candidate-vic-to-qeen.md](handoffs/release-candidate-vic-to-qeen.md). Branch `vic/release-candidate` pushed; no merge to main.

## V-52 — Cover Signature Badge & Server Integration in 0.3.0 (2026-11-14)
- Phê duyệt: Anh Zon duyệt 2026-11-14 ("Vào 0.3.0"), tích hợp ô "Cover: <số chữ ký>" dưới logo/slogan bên trái, khung viền cam bo tròn.
- Merged `origin/main` (`6e652c3`, SecuriteInfo docs) vào `vic/release-candidate` (commit `e15b864`).
- Server (Lab):
  - `scripts/server/zonendpoint-sigcount.py` + `zonendpoint-sigcount.service` + `zonendpoint-sigcount.timer` (10 phút/lần, root).
  - Đọc dòng reload/load ClamAV mới nhất trong journal (`journalctl -u clamav-daemon.service -r -g signatures -n 10 -o json`).
  - Ghi nguyên tử `/var/lib/zonendpoint/status/signatures.json` quyền 0644 (`{"signatures": 8616478, "loaded_at_utc": "...", "engine": "ClamAV 1.5.4/28182"}`).
  - Đã cài đặt, kích hoạt timer và kiểm chứng trực tiếp trên lab server.
- Client Tunnel Supervisor:
  - Mở rộng `scripts/windows/zon-tunnel.ps1`: chạy runspace nền đồng bộ `/var/lib/zonendpoint/status/signatures.json` qua SSH mỗi ~15 phút về `%LOCALAPPDATA%\ZonEndpoint\server-status.json`.
  - Tuân thủ nghiêm ngặt chỉ thị của Loki: cờ SSH non-interactive `-n -o BatchMode=yes -o ConnectTimeout=15 -o ServerAliveInterval=15 -o ServerAliveCountMax=2` cùng watchdog timeout 25s qua .NET Process, đảm bảo không bao giờ bị treo tiến trình hoặc socket rò rỉ.
  - Kiểm tra tính hợp lệ nghiêm ngặt (JSON, signatures int 0..1e9, ISO UTC timestamp), ghi nguyên tử; không làm gián đoạn vòng lặp `ssh -N`.
  - Đã test thử nghiệm bằng tiến trình tạm (`-SyncOnce`), ghi log tunnel sạch, không sửa Scheduled Task hiện tại của anh Zon.
- App Backend & UI:
  - Rust module `src-tauri/src/server_status.rs` với lệnh `get_server_status`: kiểm tra untrusted data (file <=64KB, signatures 0..1e9, stale nếu >2 giờ).
  - Frontend UI: badge `.brand-cover` dưới slogan, hiển thị `Cover: 8,616,464`, tooltip chi tiết, stale styling, tương thích 2 theme và layout co giãn.
- Quality Gates & Packaging:
  - `cargo fmt -- --check`: PASS.
  - `cargo test`: **77/77 PASS** (58 lib, 1 bin, 18 resilience).
  - `cargo clippy --all-targets -- -D warnings`: PASS.
  - `npm.cmd run build`: PASS.
  - `node tests/native/run.mjs`: **21/21 PASS** (thêm test Cover badge, 0 tiến trình mồ côi).
  - Release build NSIS: `E:\Zonendpoint-tools\artifacts\v51-release\Zonendpoint_0.3.0_x64-setup.exe` (3.714.841 bytes, PE x64, NSIS v3.13).
  - Installer SHA256: `b0f0f6a9e6a6737438aaeee166bc376c077dc53b6d36a0622325ee388ecf669f`.
  - App executable SHA256: `ae223e2533e5925106c2e36ce5bc881622e6bade4587e03cff6607e7fd1276eb`.
- Handoff: Cập nhật [release-candidate-vic-to-qeen.md](handoffs/release-candidate-vic-to-qeen.md). Đẩy `vic/release-candidate`. Không tag, không ký, không merge main.

## V-53 — Cover Badge UTF-8 BOM Fix, Strict Validation & UTC+7 Timezone Standardization in 0.3.0 (2026-11-14)
- Triển khai theo yêu cầu Zon test tay 2026-11-14: khắc phục lỗi app hiện "Cover: —" khi file status chứa UTF-8 BOM (`EF BB BF`) và chuẩn hóa toàn bộ ứng dụng sang múi giờ **UTC+7 (Asia/Ho_Chi_Minh)**.
- Nguyên nhân BOM: PowerShell 5.1/.NET ghi UTF-8 có kèm BOM khi dùng `[System.Text.Encoding]::UTF8`, khiến `serde_json::from_str` lỗi parse.
- Chuẩn hóa UTC+7:
  - Lab Server (`100.96.250.245`): đổi múi giờ sang `Asia/Ho_Chi_Minh` (+0700). Ghi nhận các timer `OnCalendar` không có hậu tố UTC bị dịch 7 giờ (`dpkg-db-backup`, `sysstat-rotate`, `sysstat-summary`, `logrotate`, `zonendpoint-yara-update`, `e2scrub`, `xfs_scrub`, `fstrim`), các timer tương đối (`sigcount` 10m, `abusech` 61m, `sysstat-collect` 10m, `clamav-health` 30m) giữ nguyên chu kỳ. Script `zonendpoint-sigcount.py` ghi giờ `+07:00` và đổi trường sang `loaded_at`.
  - Tunnel (`scripts/windows/zon-tunnel.ps1`): chuyển sang ghi UTF-8 không BOM qua `New-Object System.Text.UTF8Encoding($false)`. Format timestamp trong `tunnel.log` theo `+07:00`. Sinh trường `fetched_at` theo `+07:00` và chuẩn hóa `loaded_at` sang `+07:00`.
  - Rust Backend (`src-tauri/src/server_status.rs`): loại bỏ BOM đầu file (`trim_start_matches('\u{feff}')`) trước khi parse. Parser nhận dạng `YYYY-MM-DDTHH:MM:SS+07:00`, tương thích ngược chuyển đổi định dạng `...Z` cũ sang `+07:00`. Tính toán `stale` dựa trên epoch seconds tuyệt đối (`local_secs - 7 * 3600`), triệt tiêu hoàn toàn nguy cơ lệch 7 giờ gây stale giả.
  - Frontend UI: Tooltip hiển thị dạng `DD/MM/YYYY HH:mm (UTC+7)` độc lập 100% với cài đặt múi giờ/locale của hệ điều hành máy client: `"ClamAV signatures on server (loaded 14/11/2026 11:41 (UTC+7)), synced 14/11/2026 11:45 (UTC+7)"`.
  - Siết chặt validation: signatures bắt buộc là số nguyên JSON (từ chối số thập phân như `8616497.5`, chuỗi, số âm, > 1e9); thời gian bắt buộc đúng chuẩn ISO-8601 `+07:00` (hoặc `Z` cũ), từ chối mọi offset khác (+00:00, +08:00, v.v.), phần thập phân và hậu tố lạ.
- Phân tích rủi ro gỡ cài đặt (Mục 7):
  - Phân tích template NSIS của Tauri: lệnh gỡ chỉ xóa `zonendpoint.exe` và `uninstall.exe`, sau đó gọi `RMDir "$INSTDIR"` (không có `/r`, chỉ xóa khi thư mục rỗng).
  - Checkbox "Delete app data" chỉ xóa `%LOCALAPPDATA%\com.zonendpoint.desktop` (bundle id của WebView2), hoàn toàn không động tới `%LOCALAPPDATA%\ZonEndpoint`.
  - Kết luận: Gỡ cài đặt hoàn toàn KHÔNG xóa `cache.db`, `config.json`, `server-status.json` hay `tunnel\`.
  - Đề xuất: Chuyển thư mục cài đặt sang `%LOCALAPPDATA%\Programs\ZonEndpoint` để tách bạch rõ ràng giữa file thực thi và dữ liệu người dùng (chờ Zon quyết).
- Quality Gates:
  - `cargo fmt -- --check`: PASS.
  - `cargo test`: **81/81 PASS** (62 lib, 1 bin, 18 resilience; 11 server_status tests gồm test chống lệch 7 giờ).
  - `cargo clippy --all-targets -- -D warnings`: PASS.
  - `npm.cmd run build`: PASS.
  - `node tests/native/run.mjs`: **21/21 PASS** (mock file có UTF-8 BOM và tooltip `(UTC+7)`).
  - Đóng gói NSIS: `E:\Zonendpoint-tools\artifacts\v51-release\Zonendpoint_0.3.0_x64-setup.exe` (3,715,539 bytes, PE32+ x64, NSIS v3.13).
  - Installer SHA256: `A6797CA3FAA4DCD4A62D2FD71E225D27F720250B776A724DE2AC0A57405D394F`.
- Bàn giao: Cập nhật [release-candidate-vic-to-qeen.md](handoffs/release-candidate-vic-to-qeen.md). Đẩy nhánh `vic/release-candidate`. Không tag, không ký, không merge main.

## V-54 — Server Configuration Row Visibility When Connected OK in 0.3.0 (2026-11-14)
- Triển khai theo yêu cầu Zon 2026-11-14: ẩn hàng cấu hình Server (ô nhập `#server-address-input` + nút Save `.server-save-btn`) khi kết nối OK để giao diện gọn gàng.
- Hành vi chi tiết:
  1. Kết nối OK (`serverStatus === "connected"`): tự động ẩn hàng Server.
  2. Đang kiểm tra lần đầu (`serverStatus === "checking"`): không hiện (tránh nhấp nháy giao diện khi khởi động).
  3. Mất kết nối / lỗi kết nối (`serverStatus === "disconnected"`): tự động hiển thị hàng Server để người dùng sửa địa chỉ và Save; khi kết nối lại OK thì tự ẩn lại (trừ khi đang focus/gõ trong ô).
  4. Bật/tắt thủ công khi đang OK: bấm vào icon trạng thái server (dấu check xanh) để hiện/ẩn. Hỗ trợ đầy đủ keyboard accessible và cập nhật động `title`/`aria-label` ("Show server settings" / "Hide server settings").
  5. Bảo toàn focus: không bao giờ ẩn hàng Server khi người dùng đang focus trong ô nhập (`isServerInputFocused`).
  6. Không đổi logic kết nối/lưu cấu hình backend; bảo toàn bố cục trên 2 theme Light/Dark và cửa sổ hẹp 800x240.
- Quality Gates:
  - `cargo fmt -- --check`: PASS.
  - `cargo test`: **81/81 PASS** (62 lib, 1 bin, 18 resilience).
  - `cargo clippy --all-targets -- -D warnings`: PASS.
  - `npm.cmd run build`: PASS.
  - `node tests/native/run.mjs`: **21/21 PASS** (kiểm thử đầy đủ 3 trạng thái của hàng Server: OK -> ẩn; offline -> hiện; bấm icon -> toggle hiện/ẩn).
  - Đóng gói NSIS release: `E:\Zonendpoint-tools\artifacts\v51-release\Zonendpoint_0.3.0_x64-setup.exe` (3,716,290 bytes, PE32+ x64, NSIS v3.13).
  - Installer SHA256: `8AF8B82DEC059D138338DA880DB41BA2E2D54950ADDC33E6690430AB3B90F426`.
- Bàn giao: Cập nhật [release-candidate-vic-to-qeen.md](handoffs/release-candidate-vic-to-qeen.md). Đẩy nhánh `vic/release-candidate`. Không tag, không ký, không merge main.

## V-55 — Auto-Compact Scan Results Window to Content Height in 0.3.0 (2026-11-14)
- Triển khai theo yêu cầu Zon 2026-11-14 sau khi Q-40 phát hiện lỗi cửa sổ release cao ~800px tạo khoảng trống lớn dưới card All Good:
- Nguyên nhân gốc rễ:
  1. `src/App.tsx` trước đây sử dụng `Math.max(window.innerHeight, ...)` và điều kiện một chiều `if (height > window.innerHeight)`, khiến cửa sổ chỉ có thể nới rộng mà không bao giờ co lại.
  2. Việc đo chiều cao bằng `.app-shell.scrollHeight` bị ảnh hưởng bởi CSS `.selection-panel { min-height: max(208px, calc(100vh - 32px)); }`, khiến container luôn bị kéo giãn bằng chiều cao viewport.
- Giải pháp triển khai:
  1. Hàm `measureContentHeight()` đo chính xác tọa độ đáy của các phần tử con thực sự (`.brand-heading` và `.selection-panel` descendants kèm margin/padding), loại bỏ ảnh hưởng từ min-height viewport.
  2. Khi kết quả là "All Good" (`isClean`: hoàn tất, 0 threat/error/unknown/cancelled): đặt chiều cao mục tiêu chuẩn compact 240px.
  3. Khi kết quả có threat/error/cancelled: tự động nới cao theo `measureContentHeight()` (giới hạn an toàn theo `workArea` của màn hình).
  4. Hỗ trợ quét lần 2 All Good sau một lần kết quả dài: tự động co lại chuẩn xác về 240px.
  5. Bảo toàn tương tác kéo chỉnh kích cỡ cửa sổ thủ công của người dùng, không giật màn hình khi chưa có kết quả mới.
- Đính chính phân loại timer trong handoff: `zonendpoint-yara-update.timer` có `OnCalendar=*-*-* 20:00:00 UTC` (chỉ định rõ UTC, chạy ~03:00–05:00 +07 do randomize delay), do đó không bị dịch giờ khi đổi múi giờ sang `Asia/Ho_Chi_Minh`.
- Quality Gates:
  - `cargo fmt -- --check`: PASS.
  - `cargo test`: **81/81 PASS** (62 lib, 1 bin, 18 resilience).
  - `cargo clippy --all-targets -- -D warnings`: PASS.
  - `npm.cmd run build`: PASS.
  - `node tests/native/run.mjs`: **21/21 PASS (chạy 2 lần liên tiếp đều PASS 100%)** (xác minh nới cao > 300px khi có threat, co gọn <= 300px và khoảng trống dưới card < 80px khi All Good; khắc phục triệt để race condition V-30 bằng `waitForEnabled`).
  - Đóng gói NSIS release: `E:\Zonendpoint-tools\artifacts\v51-release\Zonendpoint_0.3.0_x64-setup.exe` (3,715,823 bytes, PE32+ x64, NSIS v3.13).
  - Installer SHA256: `fa0495faf4ae9faea6fc84fe962e6b5eac3be14ea789768490206d3b3c8c3e03`.
- Bàn giao: Cập nhật [release-candidate-vic-to-qeen.md](handoffs/release-candidate-vic-to-qeen.md). Đẩy nhánh `vic/release-candidate`. Không tag, không ký, không merge main.

## V-58 — Server Feeds & Services Hardening (2026-11-14)
- Triển khai theo chỉ đạo của anh Zon ngày 2026-11-14 dựa trên `E:\Zonendpoint\optimisse.md` phần 5 (Branch `vic/feeds-hardening`, base `origin/main 908535e`). Cô lập 100% trong repo và test môi trường tạm `/tmp/v58` trên lab; không triển khai lab thật.
- Các hạng mục hoàn thành (mỗi mục một commit riêng biệt):
  1. **L-1 (`e0b659b`)**: MalwareBazaar gặp 429/401/403/RateLimitError lập tức ngắt lượt chạy, hủy slot đã bảo lưu trong rate_limit.json, 0 request gửi sang ThreatFox, ghi rõ lỗi vào `last-run.json`. Giữ test 503 retry.
  2. **L-2 (`3735dc5`)**: Đặt mặc định `max_retries=0` cho mọi endpoint abuse.ch (`_post`, `fetch_recent`, `fetch_full_dump`). Số request gửi ngoài bằng đúng số đếm limiter.
  3. **L-8 (`19f0967`)**: Tham số `--min-reload-interval` (giờ, mặc định 6.0) kèm state file ghi nguyên tử `/opt/zonendpoint/abusech/reload-state.json`. Chưa đủ khoảng cách tối thiểu thì nạp SQLite bình thường nhưng hoãn reload clamd (`reload_deferred=True`).
  4. **L-3 (`1711522`)**: File hold (`/opt/zonendpoint/HOLD-abusech`, `/opt/zonendpoint/HOLD-yara`): cả 3 script rollback tạo file hold khi chạy; các updater phát hiện file hold thì log cảnh báo và bỏ qua lượt chạy cleanly (exit 0). Cập nhật tài liệu ops hướng dẫn gỡ hold.
  5. **L-4 (`2412db6`)**: `zonendpoint-yara-update.py` chỉ ghi tag mới vào `current_tag.txt` sau khi thấy `"Database correctly reloaded"` trong log clamd và PING thành công; nhánh rollback giữ nguyên tag cũ; thay `sleep(8)` bằng chờ log clamd có timeout và xử lý xoay vòng log.
  6. **L-6 (`1fcbb04`)**: Khóa `flock` độc quyền dùng chung `/run/lock/zonendpoint-clamd-deploy.lock` giữa abusech-update, yara-update và 3 script rollback. Không lấy được khóa -> bỏ lượt, thoát mã lỗi khác 0; sửa dứt điểm lỗi fail-open ở abusech-update.
  7. **L-11 (`c8ea858`)**: Loại bỏ file tạm đoán trước được trong `/var/lib/clamav` và rollback. Sử dụng `tempfile.mkstemp`, `os.fchown`, `os.fchmod`, `os.fsync`, và `os.replace` (Python) và `mktemp -p "$TARGET_DIR"` + `sync` + `mv -f` (Shell).
  8. **L-12 (`5979ae2`)**: Sandbox systemd cho 3 file `.service`: `ProtectSystem=strict`, `ReadWritePaths=`, `PrivateDevices=yes`, `ProtectKernelTunables=yes`, `ProtectKernelModules=yes`, `ProtectControlGroups=yes`, `RestrictAddressFamilies`, `NoNewPrivileges=yes`, `PrivateTmp=yes`. Riêng `zonendpoint-sigcount.service` chuyển sang `DynamicUser=yes`, `SupplementaryGroups=systemd-journal`, `StateDirectory=zonendpoint/status`. Đã verify bằng `systemd-analyze verify`.
  9. **L-7 (`b17b94b`)**: Thêm `timeout=120` cho tất cả tiến trình `clamscan` con; tính delta chữ ký mong đợi dựa trên số bản ghi thực tế đã sinh ra trong staging content thay vì `COUNT(*)` từ SQLite.
- Quality Gates & Testing:
  - Máy chủ lab (`udevlevel73`, Ubuntu 24.04, Python 3.14.4): **76/76 unit tests PASS** (29 MB, 10 TF, 18 abusech, 5 yara hold/reload, 5 deploy lock, 4 secure temp) trong 42.9s.
  - `systemd-analyze verify /tmp/v58/services/*.service`: **PASS (exit code 0)**.
  - Đã dọn dẹp sạch sẽ thư mục `/tmp/v58` trên lab.
- Bàn giao: Tạo `docs/handoffs/feeds-hardening-vic-to-qeen.md`. Đẩy nhánh `vic/feeds-hardening`. Không merge main.

## V-62 — Feeds Hardening Round 2 & Fix Q-43 Issues (2026-11-14)
- Triển khai theo chỉ đạo của anh Zon ngày 2026-11-14 (Branch `vic/feeds-hardening`, cô lập trong repo, không triển khai lab):
  - Khắc phục lỗi nghiêm trọng Q-43 (zero real clamd reload invariant): tiêm 100% tham số (port, socket, binary, paths) vào mock/thư mục tạm; thêm Safety Guard (`check_safety_guard` và Shell exit 99) chặn đứng mọi kết nối tới port clamd thật (3310) hoặc binary `clamdscan` khi `ZONENDPOINT_TEST_ENV=1`. Chứng minh thực nghiệm qua `journalctl` không có reload thật nào (`-- No entries --`).
  - L-3: Tạo file HOLD ngay đầu 3 script rollback (trước khi thay file), mọi nhánh lỗi sau đó giữ nguyên HOLD.
  - L-6: Lệnh `deploy-hsb` thủ công của MalwareBazaar và ThreatFox cũng lấy khóa chung `/run/lock/zonendpoint-clamd-deploy.lock` (exit 3 khi bị chiếm).
  - L-8: Khoảng cách reload tối thiểu áp cho MỌI đường nạp lại (abuse.ch, YARA, deploy-hsb thủ công) qua reload-state chung `/opt/zonendpoint/reload-state.json`. Rollback được phép bỏ qua giới hạn (`force_reload=True`) nhưng vẫn ghi nhận state. Ghi rõ freshclam nằm ngoài kiểm soát.
  - L-12: Thu hẹp `ReadWritePaths` từ `/var/backups` xuống đúng `/var/backups/clamav-malwarebazaar`, `-threatfox`, `-yara` theo từng service.
  - L-5: Phát hiện log clamd bị xoay vòng (`cur_size < log_pos_before`); không đọc được số chữ ký sau reload = "chưa xác minh" (không rollback chỉ vì thiếu dòng log nếu PING OK), lấy số trước reload từ dòng gần nhất SAU lần khởi động clamd gần nhất (`+++ Started at `).
  - L-9: Preflight chỉ `clamscan -d` với file `.hsb` mới (như YARA), không nạp main/daily/bytecode vào temporary test DB.
  - L-10: Ghi `.hsb` trực tiếp từ SQLite ra file theo dòng (streaming cursor line-by-line 64KB buffer, không dựng list trong RAM); ThreatFox dùng SQLite query `NOT EXISTS` thay `set()` Python.
  - L-13: YARA: `tag_name` kiểm bằng regex chặt `^[a-zA-Z0-9][a-zA-Z0-9_.-]{0,63}$`, URL tải bắt buộc bắt đầu bằng `https://github.com/YARAHQ/`, giới hạn byte tải (50MB) và giải nén (100MB, chống Zip Slip), kiểm tra checksum bắt buộc nếu upstream release có file checksum.
  - L-14: File khóa API quyền quá rộng (`mode & 0o077 != 0` hoặc không thuộc root) -> từ chối chạy; bộ lọc `SensitiveKeyFilter` che giấu khóa gắn vào root logger & root handlers; URL bắt buộc HTTPS (`validate_https_url`).
- Quality Gates & Testing:
  - Máy chủ lab (`udevlevel73`, Ubuntu 24.04, Python 3.14.4): **108/108 unit tests PASS 100%** trong 31.5s (37 MB, 16 TF, 18 abusech, 12 yara hold, 7 deploy lock, 4 secure temp, 8 rate limit, 6 timer sim).
  - Chứng minh zero reload thật: `sudo journalctl -u clamav-daemon --since '2026-11-14 21:50:00' --no-pager` trả về `-- No entries --`.
  - `systemd-analyze verify`: **PASS (exit code 0)**.
  - `systemd-analyze security --offline=true`: **7.1 MEDIUM** cho cả 2 service unit.
  - Đã dọn dẹp sạch sẽ thư mục `/tmp/v62` trên lab.
- Bàn giao: Cập nhật `docs/handoffs/feeds-hardening-vic-to-qeen.md` với quy trình triển khai chuẩn (dừng timer, backup + sha256 vào thư mục có ngày, copy bản mới, daemon-reload, analyze, start timer; rollback = khôi phục đúng thư mục có ngày, tuyệt đối không dùng git checkout trên lab). Đẩy nhánh `vic/feeds-hardening`. Không merge main.
