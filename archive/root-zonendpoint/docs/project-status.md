# Project status

**Read this file first at the start of every Vic/Qeen session.** Updated 2026-11-01.

## Phase

v1.0.0 manual-scan MVP; native UI branding and scan-control retest is complete for its tested scope. Current product rules are in [`v1.0.0-scope.md`](v1.0.0-scope.md). Real ClamAV validation is the next acceptance gate before expanding Guard or Auto-Quarantine.

## Worktrees and snapshots

| Purpose | Location | Branch / snapshot |
|---|---|---|
| Source checkout and project docs | `E:\Zonendpoint` | `main`, HEAD `2db42a8124f19384d154588fac41d7bab791bb97`; pre-existing user changes include `package-lock.json`, icons, `src-tauri/Cargo.lock`, and untracked source docs/assets. Preserve them. |
| Vic implementation | `E:\Zonendpoint-vic` | `vic/mvp-first-scan`; code snapshot `e3ed872ddfd4a95d9bb28a3cc6da53a656e0b0ae`; handoff-doc commit `24876647ebf1f621b3c0a567e60efd40500d6154`. Recheck branch/HEAD before work. |
| Qeen QA | `E:\Zonendpoint-QA-e3ed872` | Detached at code SHA `e3ed872ddfd4a95d9bb28a3cc6da53a656e0b0ae`; Qeen handoff snapshot `24876647ebf1f621b3c0a567e60efd40500d6154`. |

## QA state

Qeen reports **PASS for the tested native UI/icon/theme persistence, pickers, and unified Start → Stop → Stopping… → Start lifecycle only** at code SHA `e3ed872ddfd4a95d9bb28a3cc6da53a656e0b0ae`. Scan lifecycle used a local loopback mock; it is not evidence of real ClamAV detection. Real ClamAV integration and Windows installer/shortcut are **NOT RUN**. See [Qeen report](../../Zonendpoint-QA-e3ed872/docs/qa/report-e3ed872.md), [Qeen handoff](../../Zonendpoint-QA-e3ed872/docs/handoffs/qeen-to-vic.md), and [Vic handoff](../../Zonendpoint-vic/docs/handoffs/vic-to-qeen.md). The earlier [`qa/report-2db42a8.md`](qa/report-2db42a8.md) is a historical FAIL for the prior baseline, not the current code snapshot.

## Blockers and next work

**Blocker:** no isolated real-ClamAV lab result is recorded. Real-engine behavior remains unverified; do not represent mock `stream: OK` as ClamAV detection.

**Next:** prepare/use an isolated ClamAV lab and run native integration on the exact current Vic snapshot. Use benign fixtures and EICAR only inside the lab. If an existing endpoint AV blocks the fixture first, report interference/BLOCKED rather than Zonendpoint detection. Record ClamAV and signature version, command/environment, and observed UI/core result. Do not alter production configuration or security policy and do not submit company files to public services.

**Acceptance criteria:**

- Native Tauri connects only to the intended private/loopback lab daemon; daemon response and engine/signature identity are recorded.
- Clean fixture is CLEAN only after a complete successful scan; infected lab fixture is INFECTED only when the lab daemon reports it; offline, timeout, malformed/incomplete response, cancellation, oversize/policy limit never become CLEAN.
- Start/Stop lifecycle remains single-task and reports confirmed cancellation; per-file errors and final counts remain visible in folder scans.
- Record exact code SHA, commands, expected/actual results, and any AV interference. If the lab is unavailable, mark the integration NOT RUN and retain this blocker.

After this gate, validate a Windows package/installer/shortcut when a release artifact exists; current status remains NOT RUN. Do not expand Guard/Auto-Quarantine ahead of the real-engine gate.

## Later phase

SecConnect/Ed25519 gateway and licensing: **Planned / Not implemented**, outside v1.0.0. Dependencies, architecture conflicts, and decisions are tracked in [`implementation-plan.md`](implementation-plan.md) and the [licensing specification](zonendpoint_ed25519_secure_gateway_licensing_spec.md). No gateway or licensing implementation is authorized by the current scope.

## Source documents

- [v1.0.0 scope](v1.0.0-scope.md)
- [Roadmap](roadmap.md)
- [System architecture specification](zonendpoint_system_architecture_specs.md)
- [Ed25519 secure gateway and licensing specification](zonendpoint_ed25519_secure_gateway_licensing_spec.md)
- [Architecture review](architecture-review.md)
- [Vic/Qeen workflow prompts](Zonendpoint_Vic_Qeen_Codex_Prompts.md)

Recovery helper: scripts/Export-ZonendpointRecovery.ps1 exports old source/Victor/Qeen workspaces to independent bundles plus local changes. Restore into new directories and verify snapshots before resuming MVP work.


## Recovery verified — 2026-11-07
All four old checkouts restored independently under E:\Zonendpoint-restored; checksums, HEAD and dirty state match manifest. Victor branch vic/mvp-first-scan at 2487664; QA code e3ed872. See [recovery verification](recovery-verification.md). Historical missing-worktree blocker resolved via these new paths; no fresh build/native QA claimed.


## Native automation implemented — 2026-11-07
Active restored implementation: E:\Zonendpoint-restored\Zonendpoint-vic, branch vic/mvp-first-scan. Code dc57a523b26d9090bfe20ef4441cc57973787ce0; current docs HEAD 4615bab. Fixed folder cancellation race; Rust 19/19, fmt, clippy, npm audit PASS. Native mock suite 9 checks PASS; actual ClamAV benign native file/folder mode 4 checks PASS. Independent Qeen acceptance PENDING; real picker, EICAR, full real-engine failure matrix and installer NOT RUN. See restored implementation docs/handoffs/vic-to-qeen.md and docs/native-ui-tests.md. No push/merge/deploy.


UI UX Pro Max installed globally at C:\Users\admzon\.codex\skills\ui-ux-pro-max; local UX search smoke PASS. Use for subsequent UI reviews while retaining current scope. No UI implementation changed during installation.


## Approved UI snapshot — 2026-11-07
Implementation code d694f4f547c46755dbf4fcf41a7195b0b83fd99c; docs c275548, E:\Zonendpoint-restored\Zonendpoint-vic. Clean working tree after commits. Native mock9/9 + real benign4/4 PASS, static review fixes included. Detailed current state and handoff live in restored implementation docs/project-status.md and docs/qa/report-d694f4f.md. Qeen runtime acceptance and remaining lab/installer gates still pending. No push/merge/deploy.


## Local sync completed — 2026-11-07
Final logo code d5bc913; latest implementation/docs HEAD 62ec2f51a8e6cb5485e7228d83621fd843b816a3. Same SHA imported into this repo as recovery/ui-approved. Implementation working tree clean at E:\Zonendpoint-restored\Zonendpoint-vic. GitHub origin restored there; no push/merge. Source main remains2db42a8 with pre-existing dirty files preserved. Continue development in restored implementation checkout. Latest frontend build PASS; native runtime evidence remains d694f4f before final CSS sizing.


All Good/auto-resize synchronized locally: code e5ff7ee; implementation branch and source recovery/ui-approved share latest documentation HEAD (verify via git rev-parse). Pre-sync native mock9/9 PASS; implementation working tree clean. No GitHub push or main merge.


## GitHub draft PR and Qeen gate — 2026-11-07
Draft PR https://github.com/quockhanh2376/Zonendpoint/pull/1 . Remote/implementation/local recovery branch HEAD b462a3b81dd518a95c920ce86d4cb3062570a1fa; tested code8e8204a fixes two Qeen P2 issues. Qeen separate checkout Rust20/mock11/real-lab6 PASS; hook release guard verified. Real file picker Unicode selection PASS, folder picker unverified; exact Windows EICAR blocked by active AV with error225 and no CLEAN. Detailed report in restored implementation docs/qa/report-8e8204a.md. Keep draft; no merge/main overwrite. Temporary ClamAV lab-only test signature removed and cleanup verified.


Native Folder blocker closed: actual Windows dialog -> ClamAV2-file scan, Cancel/rescan PASS on code8e8204a. Report/PR updated at docsHEADc051cbc; exact EICAR Windows AV interference remains BLOCKED; no merge. Details in restored implementation docs/qa/folder-picker-8e8204a.md.


## Release installer QA — 2026-11-07
Vic/Qeen plan/prompts saved in restored implementation docs/superpowers/plans/2026-11-07-release-installer-qa.md. Product/config01e7449 fixes NSIS setup/uninstaller icons; final artifactSHA94336243A891F372B0B05D18C6EB3A180AA1C05E10CC7A54735E1428DDDD14E6. Scoped unsigned NSIS0.1.0 install/shortcut/native real scan/theme/offline/repair/uninstall/data preservation PASS. Report/docsHEADfd6a62e pushed to PR#1 and synced recovery/ui-approved. No test installation remains; original user cache preserved. EICAR Defender interference/signing/other release limits retained; no main merge/public release.


## LAB-RESILIENCE independent verification (Q-3) — 2026-11-07
Qeen verified LAB-RESILIENCE at commit e1a48d3 (branch vic/lab-resilience-plan) in isolated worktree E:\Zonendpoint-Qeen-e1a48d3. Checks A (cargo test 37/37, fmt, clippy, build), B (release guard compile_error!), D (proxy loopback-only 13399/13398), C (Tier 2 visible run 7/7 steps) PASS with real lab ClamAV on 13310. SQLite cache integrity verified (1 Clean row, 0 error/cancelled). Finding recorded on P0-01 try_read delayed second frame vulnerability and proxy instream chunk buffering race. Reports in docs/qa/resilience-e1a48d3.md and docs/handoffs/qeen-to-vic-e1a48d3.md; evidence under E:\Zonendpoint-tools\artifacts\resilience\qeen\. No push, no merge.


## Fix P1-01 independent verification (Task Q-9) — 2026-11-08
Qeen verified fix P1-01 (cache threat name preservation) at commit `d91cc29` (branch `vic/cache-reason`) in isolated detached worktree `E:\Zonendpoint-Qeen-d91cc29`.
- Artifacts & hashes match handoff: `zonendpoint.exe` (SHA256 `AE298A...`), NSIS installer (SHA256 `34E5DE...`). Cargo test 58/58 PASS, fmt/clippy PASS, npm run build PASS.
- Fix P1-01 verified: INFECTED on 2nd scan (cache hit) retains exact threat name (`Win.Test.CustomThreat.P101`), no longer overwritten by "verified content cache hit".
- Database migration verified: Schema v1 safely bumps to v2 with column `threat_name TEXT`; legacy unnamed INFECTED rows treated as cache miss and updated with real threat name; future version (e.g. 99) rejected without crash or CLEAN.
- Invariants & regressions verified: Quick Scan All Good, status dot, offline notice PASS.
- Sandbox uninstalled & cleaned up; lab SSH tunnel on 13310 verified active (PONG). Full report in `docs/qa/cache-reason-d91cc29.md`. No push, no merge.


## Integration smoke verification (Task Q-10) — 2026-11-08
Qeen verified integrated commit `69d8577` (merging `vic/ui-compact` and `vic/cache-reason`) in isolated detached worktree `E:\Zonendpoint-Qeen-69d8577`.
- Artifacts & hashes match handoff: `zonendpoint.exe` (SHA256 `8FC2193767F747E66B71214667DCCEA0E773A2629410E0D7D059EB82B83D913C`), NSIS installer (SHA256 `9B4E8D693E9D47B59C928B2124E0F01A9C17B9A5A74E28515D0A8CC6C052C728`). Cargo test 58/58 PASS, fmt/clippy PASS, npm run build PASS.
- Layout gọn verified: status dot positioned beside logo in left column, theme toggle inside scan card header row beside title, no floating header controls, >=44x44px hit targets, Light & Dark modes verified.
- Cache hit threat name preservation verified on new compact layout: live detection records `Win.Test.CustomThreat.P101` in SQLite v2 schema; second scan (cache hit) strictly preserves threat name, no overwrite by "verified content cache hit".
- Quick Scan All Good verified; tunnel off triggers red dot and offline error notice with no CLEAN verdict.
- Sandbox uninstalled & cleaned up; lab SSH tunnel on 13310 verified active (PONG). Full report in `docs/qa/integrate-69d8577.md`. 0 defects found. Ready for main merge. No push.

## YARA Live Verification (Task Q-12) — 2026-11-08
Qeen verified YARA-Forge Core live deployment end-to-end on Windows with main `4b8f0f4` (artifacts `69d8577`) against ClamAV Lab daemon on `127.0.0.1:13310` (ClamAV 1.5.4/28177 with 3,632,024 active signatures).
- Quick Scan timing: 1st scan (cache miss due to sig bump 28176->28177): 233.1s, 588 items (587 clean, 0 threats, 1 DriverData os error 5); 2nd scan (cache hit): 3.4s (All Good).
- Positive control: Benign marker test target containing `ZONQA_BENIGN_MARKER_TEST_STRING_20261108_CLAMAV_LAB` detected as `YARA.ZonQA_Benign_Marker.UNOFFICIAL` (exit 1); rescan preserves exact threat name via cache hit. Test file cleaned up.
- FP Gate: Quick Scan 0 YARA hits. Folder scan `C:\Program Files` (15,000 files) 0 hits.
- **CRITICAL FP FINDING**: Folder scan `C:\Windows\System32` (18,903 files) triggered 1 False Positive hit:
  - File: `C:\Windows\System32\winevt\Logs\Windows PowerShell.evtx`
  - Threat: `YARA.SIGNATURE_BASE_SUSP_PS1_JAB_Pattern_Jun22_1.UNOFFICIAL`
  - Root cause: Rule checks base64-encoded single-char variable PowerShell assignment without filetype constraints (`filesize < 30MB and 1 of them`); standard Windows PowerShell event log files trigger this pattern.
- Safety & Lab rules respected: 0 automatic rollback performed (Loki/Vic to handle rule blacklisting); Lab kept read-only; SSH tunnel on 13310 kept alive. Full report in `E:\Zonendpoint-tools\artifacts\feedlab\yaraforge\qeen-live-verify.md`. No push.

## Server Status Icon Flicker Fix Verification (Task Q-15) — 2026-11-08
Qeen verified V-17 (elimination of status icon flicker on preflight and periodic background checks) at commit `f41f852` (code `5199892`) in isolated detached worktree `E:\Zonendpoint-Qeen-f41f852`.
- Tests: `npm run build` PASS, `cargo test` 58/58 PASS, `npm test` 18/18 mock tests PASS (including dedicated preflight/background icon persistence test).
- Flicker check: Starting Quick Scan and continuous sampling over 32 seconds (covering 30s background poll cycle): 501 samples (50ms interval) recorded 0 occurrences of spinner/checking. Status icon remains strictly connected (tick xanh ✓).
- Manual click check: Clicking status button directly triggers spinner, button disabled, suppresses duplicate clicks, then restores connected.
- Empty port check: Port `127.0.0.1:13399` triggers disconnected (X đỏ ✕); manual click triggers spinner and returns to X đỏ; lab tunnel on 13310 kept alive.
- Regression: Quick Scan finishes with All Good.
- Defect count: 0 bugs found. Finding P2-UX from Q-14 is completely resolved. Report in `E:\Zonendpoint-tools\artifacts\status-icon\qeen-verify-2.md`. Ready for main merge. No push.

## Connect Setup & Tunnel Verification (Task Q-16) — 2026-11-08
Qeen verified V-19 (persistent SSH tunnel supervisor + in-app server settings) at commit `07ce3cc` (code `f12ae41`) in isolated worktree `E:\Zonendpoint-Qeen-07ce3cc`.
- Tests: `npm test` 19/19 mock tests PASS (3/3 runs), `cargo test` 62/62 PASS (3/3 runs: 43 lib, 1 main, 18 resilience), `npm run build` PASS (3/3 runs). 0 flakiness observed.
- Tunnel Supervisor: Scheduled Task `ZonEndpointTunnel` active. Child `ssh.exe` killed -> auto-reconnected via 5s backoff; `zPING` on 13310 returned `PONG` in 5.61s (<=60s criteria). Log `%LOCALAPPDATA%\ZonEndpoint\tunnel.log` contains zero secrets. Task kept running.
- In-App Server Settings:
  - Default (no env, no config) -> `127.0.0.1:13310` -> connected (tick xanh ✓).
  - Save `127.0.0.1:13399` -> disconnected (X đỏ ✕) + writes `config.json`; save `127.0.0.1:13310` -> returns to connected (tick xanh ✓).
  - Validation: public IP (8.8.8.8:3310), domain name, missing port correctly rejected with clear error messages.
  - Env override: process with `ZONENDPOINT_CLAMD` displays `ENV ACTIVE` badge and beats saved config.
  - Layout: clean 28px form inside card, no horizontal scroll, Light/Dark screenshots captured.
- Uninstaller Analysis & Risk Finding: NSIS `$INSTDIR` is `%LOCALAPPDATA%\Zonendpoint`. `uninstall.exe` only removes `zonendpoint.exe` and `uninstall.exe`, and uses non-recursive `RMDir "$INSTDIR"`. Uninstallation does NOT delete `cache.db`, `config.json`, or `zon-tunnel.ps1`. However, co-locating app binaries, user data, and service scripts in the same folder is an operational hazard; recommendation made to isolate tunnel scripts. Report in `E:\Zonendpoint-tools\artifacts\connect-setup\qeen-verify.md`. No push.

## Release Build from main (Task V-20) — 2026-11-08
Vic built release binaries from `origin/main` commit `07ce3cc` in isolated detached worktree `E:\Zonendpoint-build-07ce3cc`.
- Verification gates: `npm ci` PASS, `npm test` 19/19 native mock PASS, `cargo test` 62/62 PASS (43 lib, 1 main, 18 resilience), `npm run build:windows` PASS.
- Artifacts & SHA256 hashes generated in `E:\Zonendpoint-tools\artifacts\07ce3cc\`:
  - `zonendpoint.exe`: `E3AFDA906A548F7FD3BD2A3E66DC7DA4A7AF4CF22403840AD7070FAD7BB907AB`
  - `Zonendpoint_0.1.0_x64-setup.exe`: `764A230E7BEBA36C30C414DAB5439BFD68A27ED1A9A423283101EEE9F9CCD762`
  - `SHA256SUMS.txt` created.
- Rules respected: No git tag/release created, no git push to main, no installer execution, Scheduled Task `ZonEndpointTunnel` left running untouched.









