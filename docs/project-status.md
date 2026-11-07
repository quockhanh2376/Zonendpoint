# Project status — 2026-11-07

Phase:current scan candidate stabilization for an **internal company pilot**. Documentation reconciled with ZonEndpoint Notebook; new protection/enrichment features remain backlog. Recommended next task:V/Q-RELEASE-CURRENT in [implementation-plan.md](implementation-plan.md#current-execution-handoff--vq-release-current).

## Active checkout/snapshot
- Implementation:E:\Zonendpoint-restored\Zonendpoint-vic, branch vic/mvp-first-scan.
- Latest product code:c7f5fd98a9f1f81a47fae7571b9be3d0e135f884 (Quick scan/x64). Older NSIS packaging snapshot01e7449 does not contain this feature.
- Draft PR:https://github.com/quockhanh2376/Zonendpoint/pull/1 . No main merge or public release performed.
- Original source checkout E:\Zonendpoint/main and user changes remain preserved. Previously omitted architecture/licensing specs were restored under the same canonical relative filenames in this branch; architecture has a current addendum and Ed25519 content is unchanged. No alternate-name specs created.

## Current scope/UI
See [v1.0.0-scope.md](v1.0.0-scope.md). Manually initiated Quick scan (13 Windows core files+drivers) or picked file/folder Scan; Windows x64 app builds only. English, persisted Light/Dark, Guard OFF. SecConnect/licensing/quarantine deferred. Compact800x240 UI,168x162 logo and13px slogan; Select target File/Folder/Quick scan; active path; all-CLEAN green/check All Good; bounded result window growth and backend-bounded result list.

## Verification/evidence
- Core8e8204a:Rust20, extended native mock11 (incl60s timeout/51-file threat visibility), real lab6 PASS; fmt/clippy PASS, npm audit0. Release test-hook guard verified.
- Real native File Unicode selection and Folder/Cancel/rescan PASS. [Core QA report](qa/report-8e8204a.md), [Folder follow-up](qa/folder-picker-8e8204a.md).
- Current release01e7449:NSIS0.1.0 x64 build, branded setup/uninstaller, install/shortcut/theme persistence, actual installed native Folder+real ClamAV2-file scan, offline-not-CLEAN cache check, same-version repair, uninstall/data preservation PASS. [Release report](qa/release-installer-01e7449.md), [handoff](handoffs/release-vic-to-qeen.md), [plan/prompts](superpowers/plans/2026-11-07-release-installer-qa.md).
- Final unsigned installer:E:\Zonendpoint-tools\artifacts\01e7449\Zonendpoint_0.1.0_x64-setup.exe. SHA25694336243A891F372B0B05D18C6EB3A180AA1C05E10CC7A54735E1428DDDD14E6. No test installation remains; owned fixture/artifact data retained for evidence.

## Limits / next acceptance
PASS is scoped to observed core/native and unsigned NSIS current-user tests, not production readiness. Exact EICAR through Windows remains BLOCKED by Microsoft Defender (Operational1116/1117, quarantine); app reported error225, never CLEAN. ClamAV EICAR on Ubuntu and native positive detection via temporary harmless lab signature passed; signature removed/cleanup verified.

Current Quick-scan installer and current-snapshot real manual picker rerun NOT RUN. MSI, signed1.0 release, different-version upgrade, multi-user install, resource benchmark and full real-network reset/cancel matrix NOT RUN. PR stays draft pending owner's merge decision and acceptance of documented limits. No further product feature is implied by these tests.

Next acceptance:freeze a source SHA, build x64 NSIS and record SHA256, then Qeen independently verifies that exact package's native Quick/manual picker, lifecycle/theme/offline behavior and sandbox install/shortcut/repair/uninstall/data preservation. Mark protected DriverData error as incomplete; no automatic elevation/exclusion. Follow with real-network resilience, engine operations and measured performance before feature expansion. Task definitions are [here](implementation-plan.md#current-execution-handoff--vq-release-current); they have not yet been executed.

Lab:admzon@100.96.250.245; clamd loopback3310; normal Windows test tunnel127.0.0.1:13310. All testing preserved Defender/Sophos policies; no exclusions, public uploads or Internet-exposed clamd.

## Completed feature handoff — Quick scan/x64
Zon approved13 core files+drivers. Vic implementation committed c7f5fd9; independent Qeen Rust25/native mock15 PASS. Real-engine fixtures6 PASS; actual Windows589 outcomes (588 CLEAN+1 access-denied ERROR), correctly Scan incomplete/no All Good; see [task](tasks/quick-scan-x64.md) and [QA report](qa/quick-scan-c7f5fd9.md). Default Quick scan; selected target Scan; Windows x64 packages only. Previous01e7449 installer is unchanged and must not be used as new-feature evidence.

## Document map / later decisions

[Scope](v1.0.0-scope.md) → [roadmap](roadmap.md) → [technical tasks](implementation-plan.md). [Notebook reconciliation](notebook-review-2026-11-07.md) records nine source groups, policy conflicts and verified primary references; [architecture](zonendpoint_system_architecture_specs.md#12-current-implementation-and-notebook-reconciliation--2026-11-07) has the current-code addendum; [gateway/licensing](zonendpoint_ed25519_secure_gateway_licensing_spec.md) is still Planned/Not implemented. Old [architecture review](architecture-review.md) is explicitly historical.

Open later decisions:pilot distribution/signing, measured budgets, selected feed entitlement/false-positive policy, secure transport/key lifecycle, SQLCipher recovery and AI provider/data policy. Notebook suggestions alone authorize no account setup, external transmission, firewall/AV changes or feature implementation.
