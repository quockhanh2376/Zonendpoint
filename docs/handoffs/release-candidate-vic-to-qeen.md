# Handoff: Task V-51 / V-52 / V-53 — Release Candidate 0.3.0 NSIS Build (Vic to Qeen)

**Date**: 2026-11-14  
**From**: Victor (Vic)  
**To**: Qeen  
**Task**: V-51 / V-52 / V-53 (Release Candidate 0.3.0 with Cover Badge & V-53 UTF-8 BOM Fix for Qeen Testing)  
**Execution Context**: Execution of steps 1, 2, 6 of `Current execution handoff — V/Q-RELEASE-CURRENT` in `docs/implementation-plan.md`  
**Approval**: Zon approved 2026-11-13 (P0 release candidate prioritized; version 0.3.0), 2026-11-14 ("Vào 0.3.0" for Task V-52 Cover badge & server integration), and 2026-11-14 (Task V-53 UTF-8 BOM and validation fix)  
**Branch**: `vic/release-candidate`  
**Base Commit**: `f3e959d` (origin/main) merged with `6e652c3` (origin/main SecuriteInfo docs)  
**Status**: READY FOR QEEN TESTING (Pushed branch `vic/release-candidate`; DO NOT push or merge to `main`)  

---

## 1. Source & Build Environment Identity

- **Branch**: `vic/release-candidate` (includes V-51 release candidate, 0.3.0 version bump `80d2a7a`, merge of `origin/main` `6e652c3`, Task V-52 Cover badge, and Task V-53 BOM/validation fix)
- **Worktree**: `E:\Zonendpoint-worktrees\v53-cover-fix`
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
  - Size: 8,139,776 bytes
  - Built Target SHA256: `1CF16D717875A7EF6EAA9999031F1FF9B91DBB3D73684AF301F0A5405FC23DEA` (marker: `__TAURI_BUNDLE_TYPE_VAR_UNK`)
  - Installed Payload SHA256: `9A09F749F81F71ECED35A63F4282D33D20385D5A996EF025CB2F104B64EDAF49` (marker: `__TAURI_BUNDLE_TYPE_VAR_NSS`)
  - *Explanation of Hash Difference*: Tauri patches the bundle type information in the binary payload packaged into the NSIS installer from `__TAURI_BUNDLE_TYPE_VAR_UNK` (3 bytes `UNK`) to `__TAURI_BUNDLE_TYPE_VAR_NSS` (3 bytes `NSS`), exactly 3 bytes difference at offset 6237951, while restoring the target binary to `UNK`. Both files are structurally and functionally identical except for this 3-byte Tauri runtime packaging marker.
- **NSIS Installer Artifact**:
  - Built Path: `src-tauri\target\x86_64-pc-windows-msvc\release\bundle\nsis\Zonendpoint_0.3.0_x64-setup.exe`
  - Staged Path: `E:\Zonendpoint-tools\artifacts\v51-release\Zonendpoint_0.3.0_x64-setup.exe`
  - Checksum Path: `E:\Zonendpoint-tools\artifacts\v51-release\Zonendpoint_0.3.0_x64-setup.exe.sha256`
  - Size: 3,714,811 bytes (~3.54 MiB)
  - SHA256: `4C9B82F507DCA4A768A214909F84B218209F6822F656B395CE2C0F52FDD52F54`
  - Version: `0.3.0`
  - NSIS Generator Version: `v3.13`
  - Code Signing: Unsigned (not code-signed; no release tag / publish performed)

---

## 3. Feature Details: V-52 & V-53 Cover Badge, Tunnel Sync, and Fixes

### A. Server (Lab)
- **Script**: `scripts/server/zonendpoint-sigcount.py`
  - Runs via systemd timer (`zonendpoint-sigcount.timer`) every 10 minutes as root.
  - Reads newest load/reload line from `clamav-daemon.service` journal (`journalctl -u clamav-daemon.service -r -g signatures -n 10 -o json`).
  - Ghi nguyên tử sang `/var/lib/zonendpoint/status/signatures.json` với quyền `0644`.
  - Format: `{"signatures": 8616497, "loaded_at_utc": "2026-11-14T04:41:49Z", "engine": "ClamAV 1.5.4/28182"}`
  - Verified live on lab server: `8,616,497 signatures`.
- **Install on server**:
  ```bash
  sudo cp scripts/server/zonendpoint-sigcount.py /usr/local/bin/zonendpoint-sigcount.py
  sudo chmod 0755 /usr/local/bin/zonendpoint-sigcount.py
  sudo cp scripts/server/zonendpoint-sigcount.service /etc/systemd/system/
  sudo cp scripts/server/zonendpoint-sigcount.timer /etc/systemd/system/
  sudo systemctl daemon-reload
  sudo systemctl start zonendpoint-sigcount.service
  sudo systemctl enable --now zonendpoint-sigcount.timer
  ```
- **Uninstall from server**:
  ```bash
  sudo systemctl disable --now zonendpoint-sigcount.timer
  sudo rm -f /etc/systemd/system/zonendpoint-sigcount.timer /etc/systemd/system/zonendpoint-sigcount.service /usr/local/bin/zonendpoint-sigcount.py
  sudo systemctl daemon-reload
  ```

### B. Client Tunnel Supervisor (V-53 Fixes)
- **Script**: `scripts/windows/zon-tunnel.ps1`
  - Runs background PowerShell Runspace upon launch.
  - Synchronizes `/var/lib/zonendpoint/status/signatures.json` to `%LOCALAPPDATA%\ZonEndpoint\server-status.json` every ~15 minutes via SSH with the same key (`~/.ssh/zonendpoint_lab`).
  - Strict non-interactive SSH flags per Loki directive: `ssh -n -o BatchMode=yes -o ConnectTimeout=15 -o ServerAliveInterval=15 -o ServerAliveCountMax=2 -i <key> admzon@100.96.250.245 "cat /var/lib/zonendpoint/status/signatures.json"`.
  - Process execution watchdog: 25-second hard execution timeout enforced via `[System.Diagnostics.Process]::WaitForExit(25000)`; kills unresponsive SSH processes cleanly to prevent hanging the background runspace or tunnel loop.
  - **UTF-8 without BOM**: Writes status file using `New-Object System.Text.UTF8Encoding($false)`. Eliminates the `EF BB BF` byte sequence that caused `serde_json` parse failures.
  - **`fetched_at_utc` Field**: Adds client fetch timestamp (`YYYY-MM-DDTHH:MM:SSZ`) at the moment of successful retrieval.
  - **Strict Validation**: Rejects non-integer numbers (no casting `double` to `long`; rejects `8616497.5`), rejects string numbers, negative numbers, numbers > 1e9. Validates timestamp strictly against exact `^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$` and calendar date via `[DateTime]::TryParseExact`.
  - Atomic write via temporary file; errors logged to `%LOCALAPPDATA%\ZonEndpoint\tunnel\tunnel.log` without exposing secrets or key paths.
  - Zero disruption to the main thread's blocking `ssh -N -L` tunnel.

### C. App Backend & UI (V-53 Fixes)
- **Rust Backend**: `src-tauri/src/server_status.rs`
  - Command `get_server_status()` reads `%LOCALAPPDATA%\ZonEndpoint\server-status.json`.
  - Strips UTF-8 BOM if present: `content.trim_start_matches('\u{feff}')` before parsing JSON (compatible with both legacy and new files).
  - Untrusted data checks: max file size 64 KB, signatures range `0..1e9` strictly integer (`n.is_u64()`), strict ISO-8601 UTC timestamp parser (exact 20 bytes, no fractional seconds, no timezone offsets).
  - Stale calculation: based on `fetched_at_utc` > 2 hours (`STALE_THRESHOLD`). If `fetched_at_utc` is missing (legacy file), falls back to file modification time `metadata.modified()`.
- **Frontend UI**:
  - Element `.brand-cover` positioned under logo & slogan "Protect your digital life".
  - Always displays cover count directly on the badge: `"Cover: 8,616,497"`.
  - Tooltip: `"ClamAV signatures on server (loaded <loaded_at>), synced <fetched_at>"`. If stale (>2h), displays `"ClamAV signatures on server (stale, >2h) (loaded <loaded_at>), synced <fetched_at>"`.
  - Missing/invalid file: `"Cover: —"`.
  - Rounded orange border, high contrast across both Light and Dark themes, resilient to narrow window widths.

---

## 4. Pre-Build Quality Gates Verification

All tests run against worktree `v53-cover-fix` at version 0.3.0:

| Gate | Command | Result | Notes |
|---|---|---|---|
| Rust Formatting | `cargo fmt -- --check` | **PASS** | 0 diffs |
| Rust Unit & Resilience | `cargo test` | **PASS** | **79/79 PASS** (60 lib, 1 bin, 18 resilience; 9 status/validation/BOM/stale tests) |
| Rust Clippy | `cargo clippy --all-targets -- -D warnings` | **PASS** | 0 warnings |
| Frontend Build | `npm.cmd run build` | **PASS** | Vite v8.3.5, 23 modules, 0 TS/CSS errors |
| Native Test Suite | `node tests/native/run.mjs` | **PASS** | **21/21 checks PASS** (UI verified displaying `v0.3.0` & `Cover: 8,616,464` with UTF-8 BOM mock file; V-30 window close idle & active scan, 0 orphan processes) |

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
5. Verify sandbox install/repair/uninstall and data preservation for artifact `4C9B82F5...`.
6. Record logs and evidence in `docs/qa/` and `E:\Zonendpoint-tools\artifacts\`.

---

## 7. Scope & Limitations

- Unsigned NSIS installer; SmartScreen / Defender warning expected on untrusted machine.
- Microsoft Defender blocks raw EICAR string in transit; use harmless lab-signature procedure for positive detection tests.
- Guard is explicitly excluded from this release candidate (deferred to v1.1).
- No tag created, no merge to `main`.
