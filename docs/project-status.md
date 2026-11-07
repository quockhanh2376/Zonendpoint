# Project status — 2026-11-07

Phase: current scan candidate stabilization for an **internal company pilot**. Zon tested Release Candidate `79f0f59` manually: OK. GitHub main fast-forwarded `2db42a8` → `ed1088c` (Zon approved, Loki pushed). Active implementation: Task V-8 (Zon UI request: compact layout).

## Active checkout/snapshot
- Implementation: E:\Zonendpoint-vic-compact, branch vic/ui-compact (base ed1088c).
- Latest product code on main: ed1088c08c7f6a0944178c67eaf619925c00143c (`ed1088c`), containing RC `79f0f59` (Zon manual test OK). Resolved finding Q-5-01 by adding `overflow-wrap: anywhere;` to `.results small` in `src/App.css` (threat name text wrapping); added native mock test verifying `scrollWidth <= clientWidth` in `.results` in both Light and Dark themes. All quality gates pass (51/51 Rust tests, fmt/clippy pass, npm build pass, native mock 15/15 pass).
- Completed on branch `vic/ui-simplify`:
  1. **UI-SIMPLIFY**: Target dropdown simplified to File/Folder only (removed Quick Scan menu item); primary button dynamically toggles "Quick Scan" / "Scan"; summary-first results view ("All Good" card on clean, threats-only on infected, non-clean rows + warning banner on incomplete).
  2. **Preflight & Status Icon**: Connection preflight on `zVERSION` with friendly error copy; topbar status dot (green / red / grey) with 30s background poll.
  3. **P0-01 Security Fix**: Required bounded EOF drain after clamd response frame to prevent frame desynchronization and pipelined response attacks.
  4. **DriverData Exclusion (Q-4-01)**: Excluded `%SystemRoot%\System32\drivers\DriverData` from Quick Scan target set (canonical path resolution, case-insensitive comparison, not counted, not shown, no row) while preserving security invariants for all other access-denied paths and user Folder scans.
  5. **Q-5-01 Layout Fix**: Text wrapping for unbroken long threat names (150+ chars).
- Artifacts: NSIS installer `Zonendpoint_0.1.0_x64-setup.exe` (`3F69A4D54BFB6B71DE671FB76320DBB6BF4A3968D53DE8621304B1319483F807`) and standalone `zonendpoint.exe` (`94C42548B59D34A2C5900F5945FB103A248EEA77671D01F8972B098801CA7257`) at `E:\Zonendpoint-tools\artifacts\79f0f59\`.
- Handoff doc: [rc-vic-to-qeen.md](handoffs/rc-vic-to-qeen.md).
- Draft PR: https://github.com/quockhanh2376/Zonendpoint/pull/1 is fully contained in main (fast-forwarded 2db42a8 → ed1088c, Zon approved, Loki pushed) and can be closed on GitHub by Zon. No separate release or git tag has been created.
- Original source checkout E:\Zonendpoint/main and user changes remain preserved. Previously omitted architecture/licensing specs were restored under the same canonical relative filenames in this branch; architecture has a current addendum and Ed25519 content is unchanged. No alternate-name specs created.

## Current scope/UI
See [v1.0.0-scope.md](v1.0.0-scope.md). Manually initiated Quick scan (13 Windows core files + drivers, excluding `%SystemRoot%\System32\drivers\DriverData`) or picked file/folder Scan; Windows x64 app builds only. English, persisted Light/Dark, Guard OFF. SecConnect/licensing/quarantine deferred. Compact 800x240 UI, 168x162 logo and 13px slogan; Target dropdown has File / Folder only; default and post-scan auto-return to "Quick Scan"; summary-first result view (All Good card on clean, threats-only on infected, non-clean rows + warning on incomplete); server status dot (green / red / grey) with 30s idle poll; preflight checking `zVERSION` with generic "Cannot connect to ZonEndpoint server" copy.

## Verification/evidence
- Core 8e8204a: Rust 20, extended native mock 11 (incl 60s timeout/51-file threat visibility), real lab 6 PASS; fmt/clippy PASS, npm audit 0. Release test-hook guard verified.
- Real native File Unicode selection and Folder/Cancel/rescan PASS. [Core QA report](qa/report-8e8204a.md), [Folder follow-up](qa/folder-picker-8e8204a.md).
- Release package 0de3208: NSIS 0.1.0 x64 setup (`EE35537DBF8ABCBA2038F68B63A4396568F5D00AE757B2D1EBBC6F9CACD7AECD`) Qeen independent acceptance A–H PASS. Details in [quick-package-0de3208.md](qa/quick-package-0de3208.md) and [quick-package-vic-to-qeen.md](handoffs/quick-package-vic-to-qeen.md).
- LAB-RESILIENCE Tier 1: Rust 37 tests (25 lib, 1 CLI, 11 resilience integration tests), fmt, clippy -D warnings, and Vite frontend build all PASS. Addressed Qeen review items P0-01, P0-02, P1-01, P1-02, P1-03, P2-02. Zero new deps. Tier 2 Node.js zero-dep fault proxy harness provided at scripts/resilience-proxy.mjs. Details in [resilience-vic-to-qeen.md](handoffs/resilience-vic-to-qeen.md).
- UI-SIMPLIFY-PREFLIGHT 8c687b9: Rust 41 tests (29 lib, 1 main, 11 resilience), fmt, clippy -D warnings, Vite build, native mock 14/14 PASS. Details in [ui-simplify-vic-to-qeen.md](handoffs/ui-simplify-vic-to-qeen.md).
- P0-01 SECURITY FIX 0a55516: Rust 46 tests, bounded EOF drain, verified by Qeen in [ui-simplify-0a55516.md](qa/ui-simplify-0a55516.md) (All items PASS, identified Finding Q-4-01).
- QUICK-SCOPE-EXCLUSION 6294506: Rust 51 tests (34 lib, 1 main, 16 resilience), fmt, clippy -D warnings, Vite build, native mock 14/14 PASS. Excluded DriverData from Quick Scan target set while preserving all security invariants for other errors and user folder scans. Verified by Loki in Round 5.
- RELEASE CANDIDATE 79f0f59: Rust 51 tests, fmt, clippy -D warnings, Vite build, native mock 15/15 PASS (including Q-5-01 text wrap check). NSIS installer packaged at `E:\Zonendpoint-tools\artifacts\79f0f59\`. Details in [rc-vic-to-qeen.md](handoffs/rc-vic-to-qeen.md). Zon manual test: OK.
- Current release 01e7449: NSIS 0.1.0 x64 build, branded setup/uninstaller, install/shortcut/theme persistence, actual installed native Folder+real ClamAV 2-file scan, offline-not-CLEAN cache check, same-version repair, uninstall/data preservation PASS. [Release report](qa/release-installer-01e7449.md), [handoff](handoffs/release-vic-to-qeen.md).

## Limits / next acceptance
PASS is scoped to observed core/native and unsigned NSIS current-user tests, not production readiness. Exact EICAR through Windows remains BLOCKED by Microsoft Defender (Operational 1116/1117, quarantine); app reported error 225, never CLEAN. ClamAV EICAR on Ubuntu and native positive detection via temporary harmless lab signature passed; signature removed/cleanup verified.

### Items requiring Zon's manual test:
1. **File / Folder dialog pick**: Interacting with native Windows File/Folder picker dialogs (automated test is blocked due to Windows UIPI security boundaries).
2. **Auto-return after File / Folder**: Verify target path automatically clears and primary button returns to "Quick Scan" upon scan completion. (Zon tested RC: OK).
3. **Single "All Good" on a file**: Manually scan a single benign file and verify the single-line "All Good" card with green checkmark renders cleanly without error or extra list rows. (Zon tested RC: OK).

Next acceptance: Task V-8 (Zon UI compact layout) implementation, native mock verification and screenshots by Qeen.

Lab: admzon@100.96.250.245; clamd loopback 3310; normal Windows test tunnel 127.0.0.1:13310. All testing preserved Defender/Sophos policies; no exclusions, public uploads or Internet-exposed clamd.

## Document map / later decisions
[Scope](v1.0.0-scope.md) → [roadmap](roadmap.md) → [technical tasks](implementation-plan.md). [Notebook reconciliation](notebook-review-2026-11-07.md) records nine source groups, policy conflicts and verified primary references; [architecture](zonendpoint_system_architecture_specs.md#12-current-implementation-and-notebook-reconciliation--2026-11-07) has the current-code addendum; [gateway/licensing](zonendpoint_ed25519_secure_gateway_licensing_spec.md) is still Planned/Not implemented. Old [architecture review](architecture-review.md) is explicitly historical.

Open later decisions: pilot distribution/signing, measured budgets, selected feed entitlement/false-positive policy, secure transport/key lifecycle, SQLCipher recovery and AI provider/data policy. Notebook suggestions alone authorize no account setup, external transmission, firewall/AV changes or feature implementation.
