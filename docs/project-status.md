# Project status — 2026-11-07

## Active checkout/snapshot
- Implementation:E:\Zonendpoint-restored\Zonendpoint-vic, branch vic/mvp-first-scan.
- Latest product/config code:01e744974b75332592aa5621bbcfc0e12da8b477. Core scan QA snapshot8e8204a;01e7449 only brands NSIS setup/uninstaller icons.
- Draft PR:https://github.com/quockhanh2376/Zonendpoint/pull/1 . No main merge or public release performed.
- Original source checkout E:\Zonendpoint/main and user changes remain preserved. Source architecture/licensing specifications remain there; no duplicate specs created.

## Current scope/UI
See [v1.0.0-scope.md](v1.0.0-scope.md). Manual file/folder scan, English, persisted Light/Dark, Guard OFF. SecConnect/licensing/quarantine deferred. Compact800x240 UI,168x162 logo and13px slogan; Select target File/Folder; active path; all-CLEAN green/check All Good; bounded result window growth and backend-bounded result list.

## Verification/evidence
- Core8e8204a:Rust20, extended native mock11 (incl60s timeout/51-file threat visibility), real lab6 PASS; fmt/clippy PASS, npm audit0. Release test-hook guard verified.
- Real native File Unicode selection and Folder/Cancel/rescan PASS. [Core QA report](qa/report-8e8204a.md), [Folder follow-up](qa/folder-picker-8e8204a.md).
- Current release01e7449:NSIS0.1.0 x64 build, branded setup/uninstaller, install/shortcut/theme persistence, actual installed native Folder+real ClamAV2-file scan, offline-not-CLEAN cache check, same-version repair, uninstall/data preservation PASS. [Release report](qa/release-installer-01e7449.md), [handoff](handoffs/release-vic-to-qeen.md), [plan/prompts](superpowers/plans/2026-11-07-release-installer-qa.md).
- Final unsigned installer:E:\Zonendpoint-tools\artifacts\01e7449\Zonendpoint_0.1.0_x64-setup.exe. SHA25694336243A891F372B0B05D18C6EB3A180AA1C05E10CC7A54735E1428DDDD14E6. No test installation remains; owned fixture/artifact data retained for evidence.

## Limits / next acceptance
PASS is scoped to observed core/native and unsigned NSIS current-user tests, not production readiness. Exact EICAR through Windows remains BLOCKED by Microsoft Defender (Operational1116/1117, quarantine); app reported error225, never CLEAN. ClamAV EICAR on Ubuntu and native positive detection via temporary harmless lab signature passed; signature removed/cleanup verified.

MSI, signed1.0 release, different-version upgrade, multi-user install, resource benchmark and full real-network reset/cancel matrix NOT RUN. PR stays draft pending owner's merge decision and acceptance of documented limits. No further product feature is implied by these tests.

Lab:admzon@100.96.250.245; clamd loopback3310; normal Windows test tunnel127.0.0.1:13310. All testing preserved Defender/Sophos policies; no exclusions, public uploads or Internet-exposed clamd.
