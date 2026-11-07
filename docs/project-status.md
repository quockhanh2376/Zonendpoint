# Project status — 2026-11-07

Phase: current scan candidate stabilization for an **internal company pilot**. Documentation reconciled with ZonEndpoint Notebook; new protection/enrichment features remain backlog. Recommended next task: Qeen independent verification of Quick Scan DriverData exclusion & Q-4-01 resolution in [quick-scope-vic-to-qeen.md](handoffs/quick-scope-vic-to-qeen.md).

## Active checkout/snapshot
- Implementation: E:\Zonendpoint-vic-ui, branch vic/ui-simplify.
- Latest product code: 629450630b7da9db3fe0a1d69d4ae09559cf8015 (`6294506`): Resolved finding Q-4-01 by excluding `%SystemRoot%\System32\drivers\DriverData` from Quick Scan target set (canonical path resolution, case-insensitive comparison, not counted, not shown, no row). Preserved security invariant for all other access-denied paths and user Folder scans. Added 4 TDD tests; 51/51 Rust tests PASS, fmt/clippy PASS, npm run build PASS, native mock 14/14 PASS.
- Artifacts: NSIS installer `Zonendpoint_0.1.0_x64-setup.exe` (`C4571323064D090FD4A7CCA8E798021E60380452DC9442A99C60650D7B9AC30C`) and standalone `zonendpoint.exe` (`038689F3C3DD4E69BBA3C40A9BC014FCFA678D8592D15AB2D39623CD95E0810C`) at `E:\Zonendpoint-tools\artifacts\6294506\`.
- Handoff doc: [quick-scope-vic-to-qeen.md](handoffs/quick-scope-vic-to-qeen.md).
- Draft PR: https://github.com/quockhanh2376/Zonendpoint/pull/1 . No main merge or public release performed.
- Original source checkout E:\Zonendpoint/main and user changes remain preserved. Previously omitted architecture/licensing specs were restored under the same canonical relative filenames in this branch; architecture has a current addendum and Ed25519 content is unchanged. No alternate-name specs created.

## Current scope/UI
See [v1.0.0-scope.md](v1.0.0-scope.md). Manually initiated Quick scan (13 Windows core files + drivers, excluding `%SystemRoot%\System32\drivers\DriverData`) or picked file/folder Scan; Windows x64 app builds only. English, persisted Light/Dark, Guard OFF. SecConnect/licensing/quarantine deferred. Compact 800x240 UI, 168x162 logo and 13px slogan; Target dropdown has File / Folder only (Quick Scan removed); default and post-scan auto-return to "Quick Scan"; summary-first result view (All Good card on clean, threats-only on infected, non-clean rows + warning on incomplete); server status dot (green / red / grey) with 30s idle poll; preflight checking `zVERSION` with generic "Cannot connect to ZonEndpoint server" copy.

## Verification/evidence
- Core 8e8204a: Rust 20, extended native mock 11 (incl 60s timeout/51-file threat visibility), real lab 6 PASS; fmt/clippy PASS, npm audit 0. Release test-hook guard verified.
- Real native File Unicode selection and Folder/Cancel/rescan PASS. [Core QA report](qa/report-8e8204a.md), [Folder follow-up](qa/folder-picker-8e8204a.md).
- Release package 0de3208: NSIS 0.1.0 x64 setup (`EE35537DBF8ABCBA2038F68B63A4396568F5D00AE757B2D1EBBC6F9CACD7AECD`) Qeen independent acceptance A–H PASS. Details in [quick-package-0de3208.md](qa/quick-package-0de3208.md) and [quick-package-vic-to-qeen.md](handoffs/quick-package-vic-to-qeen.md).
- LAB-RESILIENCE Tier 1: Rust 37 tests (25 lib, 1 CLI, 11 resilience integration tests), fmt, clippy -D warnings, and Vite frontend build all PASS. Addressed Qeen review items P0-01, P0-02, P1-01, P1-02, P1-03, P2-02. Zero new deps. Tier 2 Node.js zero-dep fault proxy harness provided at scripts/resilience-proxy.mjs. Details in [resilience-vic-to-qeen.md](handoffs/resilience-vic-to-qeen.md).
- UI-SIMPLIFY-PREFLIGHT 8c687b9: Rust 41 tests (29 lib, 1 main, 11 resilience), fmt, clippy -D warnings, Vite build, native mock 14/14 PASS. Details in [ui-simplify-vic-to-qeen.md](handoffs/ui-simplify-vic-to-qeen.md).
- P0-01 SECURITY FIX 0a55516: Rust 46 tests, bounded EOF drain, verified by Qeen in [ui-simplify-0a55516.md](qa/ui-simplify-0a55516.md) (All items PASS, identified Finding Q-4-01).
- QUICK-SCOPE-EXCLUSION 6294506: Rust 51 tests (34 lib, 1 main, 16 resilience), fmt, clippy -D warnings, Vite build, native mock 14/14 PASS. Excluded DriverData from Quick Scan target set while preserving all security invariants for other errors and user folder scans. Details in [quick-scope-vic-to-qeen.md](handoffs/quick-scope-vic-to-qeen.md).
- Current release 01e7449: NSIS 0.1.0 x64 build, branded setup/uninstaller, install/shortcut/theme persistence, actual installed native Folder+real ClamAV 2-file scan, offline-not-CLEAN cache check, same-version repair, uninstall/data preservation PASS. [Release report](qa/release-installer-01e7449.md), [handoff](handoffs/release-vic-to-qeen.md).

## Limits / next acceptance
PASS is scoped to observed core/native and unsigned NSIS current-user tests, not production readiness. Exact EICAR through Windows remains BLOCKED by Microsoft Defender (Operational 1116/1117, quarantine); app reported error 225, never CLEAN. ClamAV EICAR on Ubuntu and native positive detection via temporary harmless lab signature passed; signature removed/cleanup verified.

Release File/Folder native dialog automation BLOCKED by Windows UIPI (manual check by anh pending).

Next acceptance: Qeen independent verification of Quick Scan DriverData exclusion and All Good achievement on branch `vic/ui-simplify` snapshot `6294506` (installer package `C4571323064D090FD4A7CCA8E798021E60380452DC9442A99C60650D7B9AC30C`).

Lab: admzon@100.96.250.245; clamd loopback 3310; normal Windows test tunnel 127.0.0.1:13310. All testing preserved Defender/Sophos policies; no exclusions, public uploads or Internet-exposed clamd.

## Document map / later decisions
[Scope](v1.0.0-scope.md) → [roadmap](roadmap.md) → [technical tasks](implementation-plan.md). [Notebook reconciliation](notebook-review-2026-11-07.md) records nine source groups, policy conflicts and verified primary references; [architecture](zonendpoint_system_architecture_specs.md#12-current-implementation-and-notebook-reconciliation--2026-11-07) has the current-code addendum; [gateway/licensing](zonendpoint_ed25519_secure_gateway_licensing_spec.md) is still Planned/Not implemented. Old [architecture review](architecture-review.md) is explicitly historical.

Open later decisions: pilot distribution/signing, measured budgets, selected feed entitlement/false-positive policy, secure transport/key lifecycle, SQLCipher recovery and AI provider/data policy. Notebook suggestions alone authorize no account setup, external transmission, firewall/AV changes or feature implementation.
