# Project status — 2026-11-08

Phase: current scan candidate stabilization for an **internal company pilot**. Integration of `vic/ui-compact` (Task V-8) and `vic/cache-reason` (Task V-11 / P1-01) approved by Zon (Task V-12) on branch `vic/integrate-ui-cache`. Zon tested Release Candidate `79f0f59` manually: OK. GitHub main fast-forwarded `2db42a8` → `ed1088c` (Zon approved, Loki pushed). Fix for finding P1-01 (cache threat name preservation) and UI compact layout merged into unified candidate branch for Qeen independent verification.

## Active checkout/snapshot
- Implementation: E:\Zonendpoint-vic-integrate, branch vic/integrate-ui-cache (base ed1088c).
- Integrated branches:
  - `vic/ui-compact` (code SHA `321d32b`): Task V-8 UI compact layout. Moved server status dot next to ZonEndpoint logo (left column, aligned with shield/wordmark); moved Light/Dark theme toggle into scan card header row; no floating controls in window top-right; maintained 44×44px hit targets, aria-labels, and tab focus order. Native mock 17/17 pass.
  - `vic/cache-reason` (code SHA `d91cc29`): Fix finding P1-01 (`scan_cache` preserves threat name on cache hit for INFECTED verdicts). Schema migration `user_version` 1 -> 2 adds `threat_name TEXT`; legacy v1 rows with NULL threat safely treated as cache miss; CLEAN preserved. Rust tests 58/58 pass.
- Artifacts & handoffs from source branches:
  - UI-COMPACT: NSIS installer `Zonendpoint_0.1.0_x64-setup.exe` (`A29C1ABB19F97E666CA674492029D854A987A3A1C829EDB84FDF42ED6D4092CF`) and standalone `zonendpoint.exe` (`DEA2944DBF1C6203927D5C3328D1BA5D125E259D19D545F3C60D349C4AA22AE2`) at `E:\Zonendpoint-tools\artifacts\321d32b\`. Screenshots (`idle-light.png`, `idle-dark.png`, `all-good-light.png`, `all-good-dark.png`) captured in the same directory. Handoff doc: [ui-compact-vic-to-qeen.md](handoffs/ui-compact-vic-to-qeen.md).
  - CACHE-REASON: NSIS installer `Zonendpoint_0.1.0_x64-setup.exe` (`34E5DE6027FB89029CE951D2D33F68C2007D2354E2965854CBBD6E944F1E0E7E`) and standalone `zonendpoint.exe` (`AE298A429F0B3B8D3EFF6AEB2A694BF3C32092BBF115B65DAD59541AAC0618F7`) at `E:\Zonendpoint-tools\artifacts\d91cc29\`. Handoff doc: [cache-reason-vic-to-qeen.md](handoffs/cache-reason-vic-to-qeen.md).
- Artifacts (Integrated Snapshot 69d8577): NSIS installer `Zonendpoint_0.1.0_x64-setup.exe` (`9B4E8D693E9D47B59C928B2124E0F01A9C17B9A5A74E28515D0A8CC6C052C728`) and standalone `zonendpoint.exe` (`8FC2193767F747E66B71214667DCCEA0E773A2629410E0D7D059EB82B83D913C`) at `E:\Zonendpoint-tools\artifacts\69d8577\`. Handoff doc: [integrate-vic-to-qeen.md](handoffs/integrate-vic-to-qeen.md).
- Completed on branch `vic/cache-reason`:
  1. **P1-01 Cache Threat Name**: Added `threat_name TEXT` to SQLite `scan_cache` schema (migrated `user_version` 1 -> 2).
  2. **INFECTED Preservation**: Cache hits for infected entries return exact threat name (e.g. `YARA.<rule>.UNOFFICIAL`) instead of "verified content cache hit".
  3. **Safe Migration**: Legacy schema v1 cache rows with NULL threat names treated as cache miss (`Ok(None)`) to force live re-scan without crashing.
  4. **Security Invariants**: `ERROR` and `UNKNOWN` are never cached and never become `CLEAN`.
- Completed on branch `vic/ui-compact`:
  1. **UI-COMPACT (Task V-8)**: Status dot positioned directly beside logo; theme toggle integrated into scan card header row; no floating controls in window top-right; keyboard focus order: logo/status → card title/toggle → target picker → primary action button; added 2 native mock checks (17/17 PASS).
- Previously completed on branch `vic/ui-simplify`:
- Operations audit (V-9): Branch `vic/engine-ops`, worktree `E:\Zonendpoint-vic-ops`. Read-only audit completed for lab ClamAV server on `100.96.250.245`. Audit findings in [engine-ops-audit-2026-11-07.md](qa/engine-ops-audit-2026-11-07.md), operations runbook in [engine-ops-runbook.md](ops/engine-ops-runbook.md), raw artifacts under `E:\Zonendpoint-tools\artifacts\engine-ops\2026-11-07\.`
  1. **UI-SIMPLIFY**: Target dropdown simplified to File/Folder only (removed Quick Scan menu item); primary button dynamically toggles "Quick Scan" / "Scan"; summary-first results view ("All Good" card on clean, threats-only on infected, non-clean rows + warning banner on incomplete).
  2. **Preflight & Status Icon**: Connection preflight on `zVERSION` with friendly error copy; topbar status dot (green / red / grey) with 30s background poll.
  3. **P0-01 Security Fix**: Required bounded EOF drain after clamd response frame to prevent frame desynchronization and pipelined response attacks.
  4. **DriverData Exclusion (Q-4-01)**: Excluded `%SystemRoot%\System32\drivers\DriverData` from Quick Scan target set (canonical path resolution, case-insensitive comparison, not counted, not shown, no row) while preserving security invariants for all other access-denied paths and user Folder scans.
  5. **Q-5-01 Layout Fix**: Text wrapping for unbroken long threat names (150+ chars).
- Draft PR: https://github.com/quockhanh2376/Zonendpoint/pull/1 is fully contained in main (fast-forwarded 2db42a8 → ed1088c, Zon approved, Loki pushed) and can be closed on GitHub by Zon. No separate release or git tag has been created. No main merge or public release performed.
- Original source checkout E:\Zonendpoint/main and user changes remain preserved. Previously omitted architecture/licensing specs were restored under the same canonical relative filenames in this branch; architecture has a current addendum and Ed25519 content is unchanged. No alternate-name specs created.

## Current scope/UI
See [v1.0.0-scope.md](v1.0.0-scope.md). Manually initiated Quick scan (13 Windows core files + drivers, excluding `%SystemRoot%\System32\drivers\DriverData`) or picked file/folder Scan; Windows x64 app builds only. English, persisted Light/Dark, Guard OFF. SecConnect/licensing/quarantine deferred. Compact 800x240 UI, 168x162 logo and 13px slogan; Target dropdown has File / Folder only; default and post-scan auto-return to "Quick Scan"; summary-first result view (All Good card on clean, threats-only on infected, non-clean rows + warning on incomplete); server status dot (green / red / grey) with 30s idle poll next to logo; preflight checking `zVERSION` with generic "Cannot connect to ZonEndpoint server" copy; theme toggle inside scan card header row.


## Task V-16 — Server Status Icon & Alignment (2026-11-08)
- Branch: vic/status-icon (base origin/main at 4b8f0f4), worktree E:\Zonendpoint-vic-status.
- Code commit: 87992b0.
- Changes:
  1. **Inline SVG Icons**:
     - connected: checkmark ✓ (--clean-button-text) on green circle (--clean).
     - disconnected: X mark (--clean-button-text) on red circle (--infected).
     - checking: spinning grey ring (--muted arc with --button-border track) with prefers-reduced-motion animation-off override.
     - 44×44px button hit target maintained; title and aria-label preserved and dynamically updated; contrast compliant in Light and Dark themes.
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


## Task V-17 — Fix Status Icon Flicker (Q-14 P2-UX, 2026-11-08)
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
- Release package 0de3208: NSIS 0.1.0 x64 setup (`EE35537DBF8ABCBA2038F68B63A4396568F5D00AE757B2D1EBBC6F9CACD7AECD`) Qeen independent acceptance A–H PASS. Details in [quick-package-0de3208.md](qa/quick-package-0de3208.md) and [quick-package-vic-to-qeen.md](handoffs/quick-package-vic-to-qeen.md).
- LAB-RESILIENCE Tier 1: Rust 37 tests (25 lib, 1 CLI, 11 resilience integration tests), fmt, clippy -D warnings, and Vite frontend build all PASS. Addressed Qeen review items P0-01, P0-02, P1-01, P1-02, P1-03, P2-02. Zero new deps. Tier 2 Node.js zero-dep fault proxy harness provided at scripts/resilience-proxy.mjs. Details in [resilience-vic-to-qeen.md](handoffs/resilience-vic-to-qeen.md).
- UI-SIMPLIFY-PREFLIGHT 8c687b9: Rust 41 tests (29 lib, 1 main, 11 resilience), fmt, clippy -D warnings, Vite build, native mock 14/14 PASS. Details in [ui-simplify-vic-to-qeen.md](handoffs/ui-simplify-vic-to-qeen.md).
- P0-01 SECURITY FIX 0a55516: Rust 46 tests, bounded EOF drain, verified by Qeen in [ui-simplify-0a55516.md](qa/ui-simplify-0a55516.md) (All items PASS, identified Finding Q-4-01).
- QUICK-SCOPE-EXCLUSION 6294506: Rust 51 tests (34 lib, 1 main, 16 resilience), fmt, clippy -D warnings, Vite build, native mock 14/14 PASS. Excluded DriverData from Quick Scan target set while preserving all security invariants for other errors and user folder scans. Verified by Loki in Round 5.
- RELEASE CANDIDATE 79f0f59: Rust 51 tests, fmt, clippy -D warnings, Vite build, native mock 15/15 PASS (including Q-5-01 text wrap check). NSIS installer packaged at `E:\Zonendpoint-tools\artifacts\79f0f59\`. Details in [rc-vic-to-qeen.md](handoffs/rc-vic-to-qeen.md). Zon manual test: OK.
- UI-COMPACT 321d32b: Rust 51 tests, fmt, clippy -D warnings, Vite build, native mock 17/17 PASS (including status dot position next to logo, theme toggle inside card in both themes, keyboard focus order). NSIS installer packaged at `E:\Zonendpoint-tools\artifacts\321d32b\`. Details in [ui-compact-vic-to-qeen.md](handoffs/ui-compact-vic-to-qeen.md).
- CACHE-REASON d91cc29: Rust 58 tests, fmt, clippy -D warnings, Vite build PASS. Preserves threat name on cache hit for INFECTED entries, safe migration from v1 to v2. NSIS installer packaged at `E:\Zonendpoint-tools\artifacts\d91cc29\`. Details in [cache-reason-vic-to-qeen.md](handoffs/cache-reason-vic-to-qeen.md).
- INTEGRATE-UI-CACHE 69d8577: Rust 58 tests (39 lib, 1 main, 18 resilience), fmt, clippy -D warnings, Vite build, native mock suite 17/17 PASS. Integrates UI compact layout and P1-01 threat name cache hit fix. NSIS installer packaged at `E:\Zonendpoint-tools\artifacts\69d8577\`. Details in [integrate-vic-to-qeen.md](handoffs/integrate-vic-to-qeen.md).
- CONNECT-SETUP (V-19): Persistent SSH tunnel supervisor (`scripts/windows/zon-tunnel.ps1`) with exponential backoff (5s -> 60s, reset at 30s stable) and scheduled task install/uninstall scripts (`scripts/windows/install-tunnel-task.ps1`, `scripts/windows/uninstall-tunnel-task.ps1`) running at user logon hidden without admin. Live verified on machine: old ssh pid 12576 stopped -> task ran -> zPING 127.0.0.1:13310 = PONG -> killed child ssh pid 8212 -> auto-reconnected new ssh pid 10272 -> zPING = PONG. Added in-app server configuration UI in `src/App.tsx` & `src/App.css` (compact, Light/Dark compliant, stored in `%LOCALAPPDATA%\ZonEndpoint\config.json`). Enforces private IP rules (loopback, RFC 1918, Tailscale 100.64.0.0/10; rejects public IPs). Priority: `ZONENDPOINT_CLAMD` > saved setting > default `127.0.0.1:13310`. Immediately tests connection upon save. Documented in `docs/ops/client-connect.md`. Quality gates: 62/62 Rust tests PASS, cargo fmt PASS, cargo clippy -D warnings PASS, npm run build PASS, 19/19 native tests PASS.
- Current release 01e7449: NSIS 0.1.0 x64 build, branded setup/uninstaller, install/shortcut/theme persistence, actual installed native Folder+real ClamAV 2-file scan, offline-not-CLEAN cache check, same-version repair, uninstall/data preservation PASS. [Release report](qa/release-installer-01e7449.md), [handoff](handoffs/release-vic-to-qeen.md).

## Limits / next acceptance
PASS is scoped to observed core/native and unsigned NSIS current-user tests, not production readiness. Exact EICAR through Windows remains BLOCKED by Microsoft Defender (Operational 1116/1117, quarantine); app reported error 225, never CLEAN. ClamAV EICAR on Ubuntu and native positive detection via temporary harmless lab signature passed; signature removed/cleanup verified.

### Items requiring Zon's manual test:
1. **File / Folder dialog pick**: Interacting with native Windows File/Folder picker dialogs (automated test is blocked due to Windows UIPI security boundaries).
2. **Auto-return after File / Folder**: Verify target path automatically clears and primary button returns to "Quick Scan" upon scan completion. (Zon tested RC: OK).
3. **Single "All Good" on a file**: Manually scan a single benign file and verify the single-line "All Good" card with green checkmark renders cleanly without error or extra list rows. (Zon tested RC: OK).

Next acceptance: Qeen independent verification of integrated package on branch `vic/integrate-ui-cache` (`69d8577`).

Lab: admzon@100.96.250.245; clamd loopback 3310; normal Windows test tunnel 127.0.0.1:13310. All testing preserved Defender/Sophos policies; no exclusions, public uploads or Internet-exposed clamd.

## Document map / later decisions
[Scope](v1.0.0-scope.md) → [roadmap](roadmap.md) → [technical tasks](implementation-plan.md). [Notebook reconciliation](notebook-review-2026-11-07.md) records nine source groups, policy conflicts and verified primary references; [architecture](zonendpoint_system_architecture_specs.md#12-current-implementation-and-notebook-reconciliation--2026-11-07) has the current-code addendum; [gateway/licensing](zonendpoint_ed25519_secure_gateway_licensing_spec.md) is still Planned/Not implemented. Old [architecture review](architecture-review.md) is explicitly historical.

Open later decisions: pilot distribution/signing, measured budgets, selected feed entitlement/false-positive policy, secure transport/key lifecycle, SQLCipher recovery and AI provider/data policy. Notebook suggestions alone authorize no account setup, external transmission, firewall/AV changes or feature implementation.


## FEED-LAB YARA Forge Staging Evaluation (V-10) — 2026-11-08
Branch `vic/feed-yaraforge` (`E:\Zonendpoint-vic-feed`), base `ed1088c`. Staging evaluation of YARA-Forge Core (`20261103`, 5,133 rules, personal/non-commercial defensive scope) in isolated lab (`~/zon-feedlab/`). Filtered via `scripts/feeds/filter_yara_clamav.py`: 3,765 rules accepted, 1,368 rejected (reasons: empty meta 577, big-endian funcs 374, engine load/subpattern 165, imported modules 174, unsupported modifiers 53, syntax/hex 8, ext vars 8, global/private 5, no condition 4). ClamAV 1.5.4 dynamic load 100% clean (3,765 rules, 0 errors, 0 warnings). Memory overhead +63.4 MiB RSS (YARA-only 95.0 MiB, swap 0MB); load delta +0.14s. Positive control `ZonQA_Benign_Marker` detected correctly (`YARA.ZonQA_Benign_Marker.UNOFFICIAL FOUND`, exit 1). FP scan on `/etc` (889 files), `/usr/bin` (873 files), and `/usr/lib` (27,405 files): 0 False Positives across 29,167 system files. Proposed live enablement and <1min rollback plan under "CẦN ZON DUYỆT". Evidence at `E:\Zonendpoint-tools\artifacts\feedlab\yaraforge\`; report `docs/qa/feedlab-yaraforge-2026-11-08.md`. Live lab clamd (PID 2245), freshclam (PID 2388), configs, and tunnel 13310 preserved untouched. No live change, no merge.
