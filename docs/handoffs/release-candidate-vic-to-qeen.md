# Handoff: Task V-51 / V-52 / V-53 / V-54 / V-55 — Release Candidate 0.3.0 NSIS Build (Vic to Qeen)

**Date**: 2026-11-14  
**From**: Victor (Vic)  
**To**: Qeen  
**Task**: V-51 / V-52 / V-53 / V-54 / V-55 (Release Candidate 0.3.0 with Cover Badge, V-53 UTF-8 BOM Fix, UTC+7 Timezone Standardization, V-54 Server Row Hide When Connected OK, and V-55 Auto-Compact Window to Content Height)  
**Execution Context**: Execution of steps 1, 2, 6 of `Current execution handoff — V/Q-RELEASE-CURRENT` in `docs/implementation-plan.md`  
**Approval**: Zon approved 2026-11-13 (P0 release candidate prioritized; version 0.3.0), 2026-11-14 ("Vào 0.3.0" for Task V-52 Cover badge & server integration), 2026-11-14 (Task V-53 UTF-8 BOM and validation fix), 2026-11-14 (V-53 UTC+7 Vietnam timezone standardization), 2026-11-14 (Task V-54 Ẩn ô Server khi kết nối OK), and 2026-11-14 (Task V-55 Co gọn cửa sổ sau scan All Good)  
**Branch**: `vic/release-candidate`  
**Base Commit**: `f3e959d` (origin/main) merged with `6e652c3` (origin/main SecuriteInfo docs)  
**Status**: READY FOR QEEN TESTING (Pushed branch `vic/release-candidate`; DO NOT push or merge to `main`)  

---

## 1. Source & Build Environment Identity

- **Branch**: `vic/release-candidate` (includes V-51 release candidate, 0.3.0 version bump `80d2a7a`, merge of `origin/main` `6e652c3`, Task V-52 Cover badge, Task V-53 BOM/validation fix, V-53 UTC+7 standardization, Task V-54 Server row visibility, and Task V-55 scan results window auto-compact)
- **Worktree**: `E:\Zonendpoint-worktrees\v55-compact`
- **Toolchain**:
  - `rustc`: 1.99.1 (58cf8f9c3 2026-10-13)
  - `cargo`: 1.99.1 (5f94df478 2026-08-27)
  - `node`: v26.13.0
  - `npm.cmd`: 11.20.0
  - `makensis`: v3.13 (`C:\Users\admzon\AppData\Local\tauri\NSIS\makensis.exe`)
- **Lockfile Integrity**:
  - `package-lock.json` SHA256: `EC08DA44135F6A693C0A7A57A4A9400D65B6BE624FC83BC48A0C130CE6E27029`
  - `src-tauri/Cargo.lock` SHA256: `122243C0C54C2EECD61164FA56924FBBD25759AFE47C06346163F745F39D97E5`

---

## 2. Artifact Details

- **Release Build Invocation**: `npm.cmd ci` followed by `npm.cmd run build:windows` (`tauri build --target x86_64-pc-windows-msvc --bundles nsis`)
- **Flags / Settings**: No `native-ui-tests` feature, no test root/fixture override in release build.
- **Application Executable (Target Build vs Installed Payload)**:
  - Built Path: `src-tauri\target\x86_64-pc-windows-msvc\release\zonendpoint.exe`
  - Built Target SHA256: marker `__TAURI_BUNDLE_TYPE_VAR_UNK`
  - Installed Payload SHA256: marker `__TAURI_BUNDLE_TYPE_VAR_NSS`
  - *Explanation of Hash Difference*: Tauri patches the bundle type information in the binary payload packaged into the NSIS installer from `__TAURI_BUNDLE_TYPE_VAR_UNK` (3 bytes `UNK`) to `__TAURI_BUNDLE_TYPE_VAR_NSS` (3 bytes `NSS`), exactly 3 bytes difference, while restoring the target binary to `UNK`. Both files are structurally and functionally identical except for this 3-byte Tauri runtime packaging marker.
- **NSIS Installer Artifact**:
  - Built Path: `src-tauri\target\x86_64-pc-windows-msvc\release\bundle\nsis\Zonendpoint_0.3.0_x64-setup.exe`
  - Staged Path: `E:\Zonendpoint-tools\artifacts\v51-release\Zonendpoint_0.3.0_x64-setup.exe`
  - Checksum Path: `E:\Zonendpoint-tools\artifacts\v51-release\Zonendpoint_0.3.0_x64-setup.exe.sha256`
  - Size: 3,715,823 bytes (~3.54 MiB)
  - SHA256: `fa0495faf4ae9faea6fc84fe962e6b5eac3be14ea789768490206d3b3c8c3e03`
  - Version: `0.3.0`
  - NSIS Generator Version: `v3.13`
  - Code Signing: Unsigned (not code-signed; no release tag / publish performed)

---

## 3. Feature Details: V-52 & V-53 Cover Badge, Tunnel Sync, and UTC+7 Standardization

### A. Server (Lab 100.96.250.245)
- **Timezone**: Changed to `Asia/Ho_Chi_Minh` (`+0700`) via `sudo timedatectl set-timezone Asia/Ho_Chi_Minh`.
- **Systemd Timers Shift Impact**:
  - *Shifted by 7 hours (calendar-based without UTC qualifier)*: `dpkg-db-backup.timer` (00:00:00 +07), `sysstat-rotate.timer` (00:00:00 +07), `sysstat-summary.timer` (00:07:00 +07), `logrotate.timer` (00:35:50 +07), `e2scrub_all.timer`/`xfs_scrub_all.timer` (03:10 +07), `fstrim.timer` (00:43:03 +07).
  - *Invariant schedule / UTC-pinned*: `zonendpoint-yara-update.timer` (defined as `OnCalendar=*-*-* 20:00:00 UTC`; explicitly pinned to UTC, running between ~03:00–05:00 +07 due to randomized delay — **NOT shifted** by the server timezone change to `Asia/Ho_Chi_Minh`).
  - *Invariant cycle (relative interval timers)*: `zonendpoint-sigcount.timer` (every 10m), `zonendpoint-abusech-update.timer` (every 61m), `sysstat-collect.timer` (every 10m), `clamav-check-update-health.timer` (every 30m).
- **Script**: `scripts/server/zonendpoint-sigcount.py`
  - Runs via systemd timer (`zonendpoint-sigcount.timer`) every 10 minutes as root.
  - Generates `/var/lib/zonendpoint/status/signatures.json` with permissions `0644`.
  - Format: `{"signatures": 8616497, "loaded_at": "2026-11-14T11:41:49+07:00", "engine": "ClamAV 1.5.4/28182"}`.

### B. Client Tunnel Supervisor
- **Script**: `scripts/windows/zon-tunnel.ps1`
  - Runs background PowerShell Runspace upon launch.
  - Synchronizes `/var/lib/zonendpoint/status/signatures.json` to `%LOCALAPPDATA%\ZonEndpoint\server-status.json` every ~15 minutes via SSH with the same key (`~/.ssh/zonendpoint_lab`).
  - Strict non-interactive SSH flags per Loki directive: `ssh -n -o BatchMode=yes -o ConnectTimeout=15 -o ServerAliveInterval=15 -o ServerAliveCountMax=2 -i <key> admzon@100.96.250.245 "cat /var/lib/zonendpoint/status/signatures.json"`.
  - Process execution watchdog: 25-second hard execution timeout enforced via `[System.Diagnostics.Process]::WaitForExit(25000)`.
  - **UTF-8 without BOM**: Writes status file using `New-Object System.Text.UTF8Encoding($false)`.
  - **UTC+7 Timezone**: Generates `fetched_at` as `YYYY-MM-DDTHH:MM:SS+07:00`. Normalizes server `loaded_at` / `loaded_at_utc` into `+07:00`.
  - **Strict Validation**: Rejects non-integer numbers (no casting `double` to `long`; rejects `8616497.5`), rejects string numbers, negative numbers, numbers > 1e9. Validates timestamp strictly against exact regex `^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\+07:00|Z)$` and calendar date.

### C. App Backend & UI
- **Rust Backend**: `src-tauri/src/server_status.rs`
  - Command `get_server_status()` reads `%LOCALAPPDATA%\ZonEndpoint\server-status.json`.
  - Strips UTF-8 BOM if present: `content.trim_start_matches('\u{feff}')` before parsing JSON.
  - Parser accepts `YYYY-MM-DDTHH:MM:SS+07:00` and backwards-compatibly converts legacy `YYYY-MM-DDTHH:MM:SSZ` to `+07:00`. Rejects all other offsets (+00:00, +08:00, etc.), fractional seconds, missing offsets.
  - Stale calculation: converts `+07:00` local time to absolute UTC epoch seconds (`total_secs - 7 * 3600`) and compares directly with `SystemTime::now()` UTC epoch seconds. Completely eliminates the 7-hour timezone skew bug.
- **Frontend UI**:
  - Element `.brand-cover` positioned under logo & slogan "Protect your digital life".
  - Always displays cover count directly on the badge: `"Cover: 8,616,497"`.
  - Tooltip: `"ClamAV signatures on server (loaded 14/11/2026 11:41 (UTC+7)), synced 14/11/2026 11:45 (UTC+7)"`. Displayed strictly in `DD/MM/YYYY HH:mm (UTC+7)` format regardless of the client machine's Windows locale/timezone settings.

### D. Server Configuration Row Visibility (Task V-54)
- **Automatic Hide When Connected OK**: When the daemon connection is healthy (`serverStatus === "connected"`), the Server configuration row (`#server-address-input` and `.server-save-btn`) is automatically hidden to keep the interface clean and focused.
- **Initial Connection Check**: During initial startup and preflight (`serverStatus === "checking"`), the server row is not displayed to prevent visual flicker.
- **Automatic Show on Connection Failure**: When connection fails or is lost (`serverStatus === "disconnected"` or error notice displayed), the Server configuration row automatically becomes visible so the user can easily reconfigure the address and save.
- **Manual Toggle via Status Button**:
  - When connected, clicking the server status button (green checkmark icon) toggles manual visibility (`showServerManual`).
  - Button `title` and `aria-label` dynamically reflect action: `"Show server settings"` when row is hidden; `"Hide server settings"` when row is visible. Fully accessible via keyboard.
- **Focus Preservation**:
  - If the user is actively typing in `#server-address-input` (`isServerInputFocused`), the row remains visible even if background re-checks or state changes occur. It never disappears out from under the user's cursor.
  - Upon successful save/reconnect, if focused, the input is programmatically blurred and hidden cleanly.
- **Layout & Responsiveness**:
  - Compact window geometry (800x240) preserved across both Light and Dark themes.

### E. Auto-Compact Scan Results Window (Task V-55)
- **Problem Statement (Q-40 Defect)**: Release candidate scan results window remained at ~800px tall after clean scan ("All Good"), leaving a large empty gap beneath the scan summary card.
- **Root Cause**:
  1. `src/App.tsx` previously used `Math.max(window.innerHeight, ...)` and guarded window resizing behind `if (height > window.innerHeight)`. This made window resizing strictly one-way (expand-only), preventing the window from ever contracting back to compact size.
  2. The desired height calculation used `.app-shell.scrollHeight`. Because `.selection-panel` has CSS `min-height: max(208px, calc(100vh - 32px))`, the panel naturally expands to fill the entire viewport, meaning `scrollHeight >= window.innerHeight` at all times.
- **Implementation**:
  1. Implemented `measureContentHeight()`: accurately computes the bottom coordinate of all rendered descendant elements (`.brand-heading` and `.selection-panel` descendants plus margins and padding), independent of viewport-stretched container heights.
  2. In the `useEffect([summary])` resize handler:
     - When `isClean` (scan complete, `scanned > 0`, `clean === scanned`, 0 threats/errors/cancelled): target height is set directly to compact standard `240px`.
     - When non-clean (threats detected, errors, or cancellation): target height is dynamically computed as `Math.max(240, measureContentHeight())` and bounded by monitor `workArea`.
     - Replaced one-way condition with bidirectional resize check: `if (!disposed && Math.abs(targetHeight - window.innerHeight) > 2)`.
  3. Rescan Behavior: When running a second scan that results in All Good after an earlier scan that produced a tall threat list, the window cleanly and automatically shrinks back to 240px.
  4. Manual User Resize: Dragging or resizing the window manually is preserved and not abruptly reset until a new scan completes.
- **Test Verification**:
  - Native test suite updated with assertions for both behaviors:
    - Non-clean threat results expand window: `innerHeight > 300px`.
    - Clean All Good results compact window: `innerHeight <= 300px` and empty space gap beneath the card `< 80px`.
  - Ran native test suite **2 consecutive times** (`node tests/native/run.mjs`): **21/21 PASS** on both runs with 0 flakes.

---

## 4. Pre-Build Quality Gates Verification

All tests run against worktree `v55-compact` at version 0.3.0:

| Gate | Command | Result | Notes |
|---|---|---|---|
| Rust Formatting | `cargo fmt -- --check` | **PASS** | 0 diffs |
| Rust Unit & Resilience | `cargo test` | **PASS** | **81/81 PASS** (62 lib, 1 bin, 18 resilience; 11 server_status tests including anti-skew 7h test) |
| Rust Clippy | `cargo clippy --all-targets -- -D warnings` | **PASS** | 0 warnings |
| Frontend Build | `npm.cmd run build` | **PASS** | Vite v8.3.5, 23 modules, 0 TS/CSS errors |
| Native Test Suite | `node tests/native/run.mjs` | **PASS** | **21/21 PASS (Run 1: PASS, Run 2: PASS)** (Verified: 1) connected OK -> server row hidden, 2) offline/fail -> server row visible, 3) icon click -> toggle show/hide; 4) All Good compact <= 300px with gap < 80px; 5) non-clean results expand > 300px; 6) V-30 window close clean exit with 0 orphans) |

---

## 5. Uninstall Risk Analysis & Recommendation (Item 7)

### A. Analysis of Tauri NSIS Uninstall Script
Inspection of the Tauri v2 NSIS uninstaller template (`Section Uninstall`):
1. **Explicit Deletes**: The uninstaller only deletes specific application files:
   - `Delete "$INSTDIR\${MAINBINARYNAME}.exe"` (`zonendpoint.exe`)
   - `Delete "$INSTDIR\uninstall.exe"`
2. **Directory Removal**: The uninstaller executes `RMDir "$INSTDIR"` (without `/r`).
   - In NSIS, `RMDir` without `/r` **only removes the directory if it is completely empty**.
   - Because `%LOCALAPPDATA%\ZonEndpoint` contains runtime data (`cache.db`, `config.json`, `server-status.json`, and the `tunnel\` directory), `RMDir "$INSTDIR"` fails silently and leaves all runtime data and subdirectories completely intact.
3. **"Delete app data" Checkbox State**:
   - When the user checks "Delete app data" (`$DeleteAppDataCheckboxState = 1`), Tauri executes:
     `RmDir /r "$LOCALAPPDATA\${BUNDLEID}"`
   - Since `${BUNDLEID}` is `com.zonendpoint.desktop` (from `tauri.conf.json`), this removes the WebView2 runtime cache (`%LOCALAPPDATA%\com.zonendpoint.desktop`), but **never touches `%LOCALAPPDATA%\ZonEndpoint`**.
4. **Definitive Answer**: Running the uninstaller (with or without "Delete app data" checked) **DOES NOT delete** `cache.db`, `config.json`, `server-status.json`, or `tunnel\zon-tunnel.ps1`.

### B. Latent Risks & Proposed Remediation
- **Risk**: Windows NTFS is case-insensitive. Therefore, the application installation directory `%LOCALAPPDATA%\Zonendpoint` and the runtime data directory `%LOCALAPPDATA%\ZonEndpoint` are the exact same folder on disk. Co-locating binaries and runtime state is risky if a future uninstaller or cleaning utility uses recursive removal (`RMDir /r`).
- **Standard Convention**: Windows desktop apps install per-user executables into `%LOCALAPPDATA%\Programs\<AppName>` (e.g., VS Code, Discord, Tauri `perUser` default when configured).
- **Proposal for Zon's Decision**: Change the installation target in `tauri.conf.json` or NSIS script to `%LOCALAPPDATA%\Programs\ZonEndpoint` (separate from data directory `%LOCALAPPDATA%\ZonEndpoint`). Left unchanged for now pending Zon's decision.

---

## 6. Next Steps for Qeen (Steps 3–5 of V/Q-RELEASE-CURRENT)

1. Independent checkout at this handoff snapshot SHA on `vic/release-candidate`.
2. Run installation tests in dedicated sandbox (`E:\Zonendpoint-tools\installer-sandbox\app`) or isolated VM — **DO NOT install, overwrite, or uninstall Zon's active production app**.
3. Verify Cover badge behavior with mock and real `server-status.json` in both themes.
4. Verify actual native File and Folder dialogs (including Cancel/Unicode paths), initial Quick scan scope, Manual Scan return, Stop→Stopping→rescan, theme/icon persistence, and bounded result window.
5. Verify sandbox install/repair/uninstall and data preservation for artifact `A6797CA3...`.
6. Record logs and evidence in `docs/qa/` and `E:\Zonendpoint-tools\artifacts\`.

---

## 7. Scope & Limitations

- Unsigned NSIS installer; SmartScreen / Defender warning expected on untrusted machine.
- Microsoft Defender blocks raw EICAR string in transit; use harmless lab-signature procedure for positive detection tests.
- Guard is explicitly excluded from this release candidate (deferred to v1.1).
- No tag created, no merge to `main`.
