# Quick Scan & Packaging QA — 0de3208

Independent test report by Qeen for Zonendpoint package `0de3208`.
Date: 2026-11-07.
Audience: Internal company pilot.
Checkout: `E:\Zonendpoint-Qeen-0de3208` (detached worktree at commit `0de3208a6ce209b52f885c75053e97e09597c81c`).
Vic checkout `E:\Zonendpoint-restored\Zonendpoint-vic` and `E:\Zonendpoint` preserved unmodified.

## Verified Targets & Checksums

| Artifact | Path | Size | SHA256 | Authenticode |
|---|---|---|---|---|
| Source commit | `0de3208a6ce209b52f885c75053e97e09597c81c` | - | - | - |
| Installer (NSIS x64) | `E:\Zonendpoint-tools\artifacts\0de3208\Zonendpoint_0.1.0_x64-setup.exe` | 3,676,007 bytes | `EE35537DBF8ABCBA2038F68B63A4396568F5D00AE757B2D1EBBC6F9CACD7AECD` | NotSigned |
| App Executable | `E:\Zonendpoint-tools\artifacts\0de3208\zonendpoint.exe` | 8,031,232 bytes | `86E31039692EAAD7EDB7E0580297D982B0B7EA3A3638011062B99EC2AEFEAC3C` | NotSigned |

Lockfiles verified identical:
- `package-lock.json`: `C2A4A6525A2881B3083B7320EF8DB7B8EC9A7B249C4D897A04B209F959FE6EAA`
- `src-tauri/Cargo.lock`: `4ACFFD0B3A440B834DC1916B5C0C2CEA6419DD5782BB78679D99D68F5C0FCB7E`

## Test Cases Summary

| Group | Description | Status | Evidence & Observations |
|---|---|---|---|
| **A** | Static verification rerun (`cargo test --locked`, `fmt --check`, `clippy -D warnings`, `npm run build`) | **PASS** | 26 Rust tests (25 library + 1 CLI) passed; fmt exit 0; clippy 0 warnings; Vite build finished in 560ms. Logs in `qeen/cargo-test.log`, `qeen/frontend-build.log`. |
| **B** | Release hook guard (release ignores bogus `ZONENDPOINT_TEST_SYSTEM_ROOT` / fixture overrides) | **PASS** | `cargo check --release --features native-ui-tests` fails compilation with `compile_error!("native-ui-tests must never be enabled in release builds")`. Actual installed release ignores deliberately bogus root/fixture overrides and scans native System32. Log: `qeen/release-hook-compile-error.log`. |
| **C** | CLI exit codes on installed executable | **PASS** | Regression verified on installed release binary: Clean=0; Infected=1; Error=1; Unknown=1; Skipped=1. Cache safety: cached Clean never reused when engine is offline (returns Error, exit 1). Positive fixture with verified lab signature returns Infected, exit 1. Evidence: `qeen/cli-exit-report.json`. |
| **D** | Native UI lifecycle, labels, scope, and single-task controls | **PASS / BLOCKED** | **PASS**: Default button label is "Quick Scan", scope is "Windows core files + drivers". Target menu contains "File", "Folder", "Quick Scan". Quick Scan Stop -> "Stopping…" -> "Scan cancelled" lifecycle cleanly transitions without deadlock; rescan starts immediately. Actual Windows root Quick Scan executed: 589 items, 588 clean, 1 DriverData Access denied (os error 5) -> "Scan incomplete", never "All Good".<br>**BLOCKED**: Unattended programmatic synthetic input into native OS modal dialogs (`#32770`) in release mode without test injection is restricted by Windows UIPI (`SendWait: Access is denied`) and system script policy. |
| **E** | Typed outcomes & error handling | **PASS** | Real lab benign (`127.0.0.1:13310`): CLEAN. Authorized positive (temporary lab signature `Zonendpoint.LabOnly.Test`): INFECTED. Offline engine: ERROR (never CLEAN). Protected path (`DriverData`): ERROR os error 5, resulting in "Scan incomplete" and never "All Good". Window auto-grows to fit details; detail bounds enforced. |
| **F** | Theme persistence & Icon fidelity | **PASS** | First launch starts in Light theme. Accessible toggle switches to Dark. Relaunching application with same profile preserves Dark theme. Icon extraction from setup, app, and uninstaller matches byte-for-byte (SHA256 `4197EB842C22B9EDC5D6B44343C9CE456D95A9292334D511B1D6524F5739A655`). PNGs: `qeen/app-icon.png`, `setup-icon.png`, `uninstaller-icon.png`. |
| **G** | Installer, shortcut, repair, uninstall, and data preservation | **PASS** | Sandbox install (`/S /D=...`) exit 0; Start Menu & Desktop shortcuts target sandbox binary; payload provenance verified (identical 8,031,232 bytes except 3-byte Tauri NSIS bundle marker `UNK`->`NSS` at offset 6139063). Same-version repair restores missing payload byte-identical (exit 0). Silent uninstall (`uninstall.exe /S`) cleans up payload, shortcuts, and HKCU registry; private userdata sentinel `preserve-me.txt` and original user app/data preserved unchanged. Evidence: `qeen/install-result.json`, `repair-result.json`, `uninstall-result.json`. |
| **H** | Security audit of diff `01e7449..0de3208` | **PASS** | Path traversal strictly rejects junctions (`follow_root_links(false)`); no automatic UAC elevation (`asInvoker`); strict error->verdict mapping (only Clean exits 0); IPC surface minimized (quick_scan takes no client paths; scan_path validates ID and 300s expiration); strict CSP (`default-src 'self'`). |

## Detailed Evidence & Audit Notes

### 1. Lab Positive & Cleanup
- Lab connection: loopback tunnel `127.0.0.1:13310` mapping to ClamAV `3310` on `100.96.250.245`.
- Harmless lab signature procedure: `Zonendpoint.LabOnly.Test` installed via `install.py` with sudo. Tested installed release CLI against `harmless-positive.bin`: status returned `Infected`, exit code `1`.
- Cleanup procedure: `cleanup.py` executed; `CLEANUP VERIFIED: stream: OK` confirmed by reload. Lab signature completely removed.
- Microsoft Defender remained enabled at all times with zero exclusions.

### 2. Actual System Quick Scan
- Run directory: `zonendpoint-windows-quick-IqK2zp`
- Results: Processed 589 items, 588 clean, 0 threats, 0 unknown, 1 error, 0 skipped, 0 cancelled.
- Retained core files: all 13 core files present (`ntoskrnl.exe`, `ntdll.dll`, `kernel32.dll`, `kernelbase.dll`, `ci.dll`, `smss.exe`, `csrss.exe`, `wininit.exe`, `services.exe`, `lsass.exe`, `svchost.exe`, `winlogon.exe`, `winload.efi`).
- Protected directory: `C:\WINDOWS\system32\drivers\DriverData` returned `Access is denied (os error 5)`.
- UI Heading: `Scan incomplete`. Button: `Quick Scan` (not `All Good`). Text explicitly warns: `Do not treat an incomplete scan as clean.`

### 3. Defender Incident & Safe Execution Rule Adherence
- At 4:10 PM, Defender intercepted an inline `node -e` command chaining process execution with `ExecutionPolicy Bypass`.
- In compliance with anh Zon's instructions:
  - All subsequent tests were written to dedicated `.mjs` scripts under `E:\Zonendpoint-tools\artifacts\0de3208\qeen\`.
  - No `powershell -ExecutionPolicy Bypass` was invoked from Node.
  - Native UI tests ran in the visible foreground.
  - Defender policies remained strictly enforced without exclusions.

## Artifact Manifest in `E:\Zonendpoint-tools\artifacts\0de3208\qeen\`

- `cargo-test.log`: Full cargo test output (26 passed).
- `frontend-build.log`: Vite production build log.
- `release-hook-compile-error.log`: Proof of compile-time rejection of test feature in release.
- `cli-exit-report.json`: CLI exit code regression results across all outcomes.
- `payload-provenance.json`: Byte-level comparison between built and installed executables.
- `native-lifecycle-report.json`: WebdriverIO native UI lifecycle and theme persistence results.
- `quick-scan-actual-report.json`: Real Windows System32 Quick Scan results (589 items).
- `quick-scan-actual-result.png`: Screenshot of actual Quick Scan terminal state.
- `install-result.json`, `repair-result.json`, `uninstall-result.json`: NSIS installer test results.
- `app-icon.png`, `setup-icon.png`, `uninstaller-icon.png`: Extracted 32px icons.

## Conclusion

Commit `0de3208` successfully fixes the CLI exit code defect from `84c4dfa` (non-clean scan outcomes now strictly exit with code 1). Unsigned NSIS packaging, branding, theme persistence, and Quick Scan scope on x64 are verified. PR remains draft pending product owner merge decision. No public release or production deployment claim.
