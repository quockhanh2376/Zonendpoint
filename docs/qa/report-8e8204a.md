# Qeen QA — 8e8204a

**Verdict: BLOCKED for full merge acceptance.** No remaining actionable finding in the two reviewed fixes, but folder-picker runtime and exact native EICAR detection remain unverified. Draft PR: https://github.com/quockhanh2376/Zonendpoint/pull/1 . No merge performed.

## Snapshot and environment
- Original review snapshot e0c3c7652a2e663b8dc29de55bb7bc26c74032fb failed two P2 findings below.
- Fixed code tested: 8e8204aa10f478362263b60b8e46226367e0052f, separate detached QA checkout E:\Zonendpoint-Qeen-e0c3c76. Production fixes were made in Victor checkout, not QA checkout. Supplemental QA scripts are untracked test additions there and preserved under evidence-8e8204a.
- Separate CARGO_TARGET_DIR E:\Zonendpoint-tools\qeen-target; clean npm ci (209 packages), npm audit zero vulnerabilities. Windows MSVC/Rust, Edge WebDriver157.0.4341.42, tauri-driver2.1.0, WebdriverIO10.2.0.
- Ubuntu26.04.1, ClamAV1.5.4/daily28175 through SSH loopback tunnel127.0.0.1:13310. Defender AntivirusEnabled=True and RealTimeProtectionEnabled=True; no AV exclusions or policy changes.

## Fixed findings
| Severity | Finding at original snapshot | Fix/evidence |
|---|---|---|
| P2 | lib.rs mark_user_cancelled replaced INFECTED/ERROR with CANCELLED, discarding evidence | Only racing CLEAN is revoked. Regression failed Cancelled vs Infected before fix; Rust20 tests pass after fix. Positive/error reason and counters retained. |
| P2 | App.tsx showed last50 retained results without notice, hiding threat/error paths | Render all backend-bounded retained results in scrollable list. Native regression failed50 vs51 before fix; folder51 files with first mock INFECTED then50 CLEAN passes and verifies51 stream requests. |

Independent static reviewer rechecked e0c3c76..8e8204a and found no important new issue in these corrections.

## Observed checks
| Area | Status | Evidence |
|---|---|---|
| Fresh QA build/dependencies | PASS | npm ci; native build from exact fixed checkout, separate target. |
| Rust | PASS | cargo test --locked --manifest-path src-tauri/Cargo.toml:20 passed; fmt --check; clippy --all-targets -- -D warnings. |
| Release test-hook rejection | PASS | cargo check --locked --release --features native-ui-tests fails101 specifically at compile_error: native-ui-tests must never be enabled in release builds. Retested on fixed SHA. This is an expected failure, not successful release packaging. |
| Native mock extended suite | PASS11/11 | node tests/native/qeen-timeout.mjs. Theme/persistence, CLEAN/INFECTED, error/malformed,51 results, full60-second scanner timeout, Stop/rescan, offline; errors never All Good/CLEAN. |
| Native real-engine benign/limits | PASS5/5 | Actual ClamAV file/folder benign, theme/persistence,30MiB daemon stream limit yields incomplete/error and never All Good. Targets supplied by explicit debug test hook. |
| Native real-engine positive result | PASS | Temporary lab-only hash signature for a harmless binary fixture produced Threat detected through real Rust IPC/ClamAV, not a mocked response. Combined real-lab suite6/6. Signature removed/reloaded afterward; same bytes returned stream:OK. This is not an EICAR detection claim. |
| EICAR on Ubuntu clamd | PASS | Lab socket returned stream:Eicar-Test-Signature FOUND for standard68-byte EICAR test data. |
| Exact EICAR via Windows native app | BLOCKED | Endpoint AV intercepted fixture first. Scanner returned open scan target:Operation did not complete successfully because the file contains a virus or potentially unwanted software (os error225). All Good absent. Raw assertion report says FAIL because detection was not observed; annotated QA conclusion is AV interference/BLOCKED, not Zonendpoint detection. |
| Real Windows file picker | PASS for selection | Opened QA executable directly with ZONENDPOINT_TEST_FIXTURE unset. Used actual Windows Open dialog, selected Kiểm thử — benign.txt and observed exact Unicode path plus enabled Start scan. No scan result claimed for this direct-picker run (default daemon endpoint differs). |
| Real Windows folder picker | BLOCKED | Desktop control activation/geometry failed before a verified selection. WebDriver-injected folder tests do not replace this check. |
| Installer/shortcut, resource benchmark, real-network reset/cancel matrix | NOT RUN | Outside completed checks; no release artifact produced. |

## Reproduce and evidence
From fixed checkout with PATH including Node/cargo, EDGE_DRIVER matching Edge, and CARGO_TARGET_DIR pointing at a separate QA output:
- cargo test --locked --manifest-path src-tauri/Cargo.toml
- cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
- cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
- cargo check --locked --release --features native-ui-tests --manifest-path src-tauri/Cargo.toml (expect explicit guard failure)
- node docs/qa/evidence-8e8204a/qeen-timeout.mjs
- Set ZONENDPOINT_CLAMD=127.0.0.1:13310; node docs/qa/evidence-8e8204a/qeen-eicar.mjs --skip-build --real (AV may block).
- Custom harmless-signature diagnostic requires the included install/cleanup Python scripts run only on the authorized Ubuntu lab via sudo; qeen-real-detection.mjs then runs with --skip-build --real. Always clean up afterward. The included picker script records the first WebDriver attempt; final successful file selection used desktop control on a directly launched app.

Raw JSON reports/scripts are in evidence-8e8204a. Full local logs: E:\Zonendpoint-tools\qeen-fixed-rust.log, qeen-clippy.log, qeen-fixed-release-guard.log, qeen-native-matrix.log, qeen-real-matrix.log, qeen-real-detection.log. Screenshots in printed temp artifacts; actual EICAR sample not committed. No credentials captured.

## Next gate
Complete actual folder selection in an interactive desktop and record it on this code. Resolve or explicitly accept the exact native EICAR environment limitation without disabling endpoint protection; native positive-result integration has separate harmless-signature evidence. Keep draft pending that decision and any required real-network matrix. No production-readiness or unqualified full-MVP PASS claim.
