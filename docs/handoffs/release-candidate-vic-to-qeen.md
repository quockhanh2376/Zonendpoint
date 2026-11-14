# Handoff: Task V-51 / V-52 — Release Candidate 0.3.0 NSIS Build (Vic to Qeen)

**Date**: 2026-11-14  
**From**: Victor (Vic)  
**To**: Qeen  
**Task**: V-51 / V-52 (Release Candidate 0.3.0 with Cover Badge for Qeen Testing)  
**Execution Context**: Execution of steps 1, 2, 6 of `Current execution handoff — V/Q-RELEASE-CURRENT` in `docs/implementation-plan.md`  
**Approval**: Zon approved 2026-11-13 (P0 release candidate prioritized; version 0.3.0) and 2026-11-14 ("Vào 0.3.0" for Task V-52 Cover badge & server integration)  
**Branch**: `vic/release-candidate`  
**Base Commit**: `f3e959d` (origin/main) merged with `6e652c3` (origin/main SecuriteInfo docs)  
**Status**: READY FOR QEEN TESTING (Pushed branch `vic/release-candidate`; DO NOT push or merge to `main`)  

---

## 1. Source & Build Environment Identity

- **Branch**: `vic/release-candidate` (includes V-51 release candidate, 0.3.0 version bump `80d2a7a`, merge of `origin/main` `6e652c3`, and Task V-52 Cover badge implementation)
- **Worktree**: `E:\Zonendpoint-worktrees\v52-cover`
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
- **Application Executable**:
  - Path: `src-tauri\target\x86_64-pc-windows-msvc\release\zonendpoint.exe`
  - PE Machine: `0x8664` (PE32+ x64 / AMD64)
  - SHA256: `D6D71BC82E821E5E1302B97CA745BF91308E6F6D53CA79B58FDEF7EBDFD89950`
- **NSIS Installer Artifact**:
  - Built Path: `src-tauri\target\x86_64-pc-windows-msvc\release\bundle\nsis\Zonendpoint_0.3.0_x64-setup.exe`
  - Staged Path: `E:\Zonendpoint-tools\artifacts\v51-release\Zonendpoint_0.3.0_x64-setup.exe`
  - Checksum Path: `E:\Zonendpoint-tools\artifacts\v51-release\Zonendpoint_0.3.0_x64-setup.exe.sha256`
  - Size: 3,714,887 bytes (~3.54 MiB)
  - SHA256: `117DF7F33CC590C5642B51BC7803633F4E841ABEC5D45DF2973938C8E08A0F5F`
  - Version: `0.3.0`
  - NSIS Generator Version: `v3.13`
  - Code Signing: Unsigned (not code-signed; no release tag / publish performed)

---

## 3. Feature Details: V-52 Cover Badge & Server Sync

### A. Server (Lab)
- **Script**: `scripts/server/zonendpoint-sigcount.py`
  - Runs via systemd timer (`zonendpoint-sigcount.timer`) every 10 minutes as root.
  - Reads newest load/reload line from `clamav-daemon.service` journal (`journalctl -u clamav-daemon.service -r -g signatures -n 10 -o json`).
  - Ghi nguyên tử sang `/var/lib/zonendpoint/status/signatures.json` với quyền `0644`.
  - Format: `{"signatures": 8616478, "loaded_at_utc": "2026-11-14T03:39:43Z", "engine": "ClamAV 1.5.4/28182"}`
  - Verified live on lab server: `8,616,478 signatures`.
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

### B. Client Tunnel Supervisor
- **Script**: `scripts/windows/zon-tunnel.ps1`
  - Starts an independent background PowerShell Runspace upon launch.
  - Synchronizes `/var/lib/zonendpoint/status/signatures.json` to `%LOCALAPPDATA%\ZonEndpoint\server-status.json` every ~15 minutes via SSH with the same key (`~/.ssh/zonendpoint_lab`).
  - Strict validation: valid JSON, integer signatures `0..1e9`, valid ISO 8601 UTC timestamp.
  - Atomic write via temporary file; errors logged to `%LOCALAPPDATA%\ZonEndpoint\tunnel\tunnel.log` without exposing secrets or key paths.
  - Zero disruption to the main thread's blocking `ssh -N -L` tunnel.

### C. App Backend & UI
- **Rust Backend**: `src-tauri/src/server_status.rs`
  - Command `get_server_status()` reads `%LOCALAPPDATA%\ZonEndpoint\server-status.json`.
  - Untrusted data checks: max file size 64 KB, signatures range `0..1e9`, timestamp validation.
  - Stale detection: `stale = true` if older than 2 hours.
- **Frontend UI**:
  - Element `.brand-cover` positioned under logo & slogan "Protect your digital life".
  - Displays `"Cover: 8,616,464"` (comma-separated).
  - Tooltip: `"ClamAV signatures on server, updated <time>"` (indicates stale if >2h).
  - Missing/invalid file: `"Cover: —"`.
  - Rounded orange border, high contrast across both Light and Dark themes, resilient to narrow window widths.

---

## 4. Pre-Build Quality Gates Verification

All tests run against worktree `v52-cover` at version 0.3.0:

| Gate | Command | Result | Notes |
|---|---|---|---|
| Rust Formatting | `cargo fmt -- --check` | **PASS** | 0 diffs |
| Rust Unit & Resilience | `cargo test` | **PASS** | **77/77 PASS** (58 lib, 1 bin, 18 resilience; 7 new status/validation tests) |
| Rust Clippy | `cargo clippy --all-targets -- -D warnings` | **PASS** | 0 warnings |
| Frontend Build | `npm.cmd run build` | **PASS** | Vite v8.3.5, 23 modules, 0 TS/CSS errors |
| Native Test Suite | `node tests/native/run.mjs` | **PASS** | **21/21 checks PASS** (UI verified displaying `v0.3.0` & `Cover: 8,616,464`; V-30 window close idle & active scan, 0 orphan processes) |

---

## 5. Next Steps for Qeen (Steps 3–5 of V/Q-RELEASE-CURRENT)

1. Independent checkout at this handoff snapshot SHA on `vic/release-candidate`.
2. Run installation tests in dedicated sandbox (`E:\Zonendpoint-tools\installer-sandbox\app`) — **DO NOT install, overwrite, or uninstall Zon's active production app**.
3. Verify Cover badge behavior with mock and real `server-status.json` in both themes.
4. Verify actual native File and Folder dialogs (including Cancel/Unicode paths), initial Quick scan scope, Manual Scan return, Stop→Stopping→rescan, theme/icon persistence, and bounded result window.
5. Verify sandbox install/repair/uninstall and data preservation for artifact `117DF7F3...`.
6. Record logs and evidence in `docs/qa/` and `E:\Zonendpoint-tools\artifacts\`.

---

## 6. Scope & Limitations

- Unsigned NSIS installer; SmartScreen / Defender warning expected on untrusted machine.
- Microsoft Defender blocks raw EICAR string in transit; use harmless lab-signature procedure for positive detection tests.
- Guard is explicitly excluded from this release candidate (deferred to v1.1).
- No tag created, no merge to `main`.
