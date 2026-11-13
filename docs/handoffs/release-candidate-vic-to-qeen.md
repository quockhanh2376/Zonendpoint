# Handoff: Task V-51 / P0 — Release Candidate NSIS Build (Vic to Qeen)

**Date**: 2026-11-13  
**From**: Victor (Vic)  
**To**: Qeen  
**Task**: V-51 / P0 (Release Candidate Installer for Qeen Testing)  
**Execution Context**: Execution of steps 1, 2, 6 of `Current execution handoff — V/Q-RELEASE-CURRENT` in `docs/implementation-plan.md`  
**Approval**: Zon approved 2026-11-13 (P0 release candidate prioritized for Qeen test; Guard shifted to v1.1)  
**Branch**: `vic/release-candidate`  
**Base Commit (origin/main)**: `f3e959d9ff2d573a321b7499fcbff2e548cb822a` (`f3e959d`)  
**Status**: READY FOR QEEN TESTING (Pushed branch `vic/release-candidate`; DO NOT push or merge to `main`)  

---

## 1. Source & Build Environment Identity

- **Source SHA**: `f3e959d9ff2d573a321b7499fcbff2e548cb822a` (incorporates close-fix V-30M + parallel scan concurrency J-03/J-04/J-05 capped at 4)
- **Worktree**: `E:\Zonendpoint-worktrees\v51-release` (clean state verified prior to build and commit)
- **Toolchain**:
  - `rustc`: 1.99.1 (58cf8f9c3 2026-10-13)
  - `cargo`: 1.99.1 (5f94df478 2026-08-27)
  - `node`: v26.13.0
  - `npm.cmd`: 11.20.0
  - `makensis`: v3.13 (`C:\Users\admzon\AppData\Local\tauri\NSIS\makensis.exe`)
- **Lockfile Integrity**:
  - `package-lock.json` SHA256: `7B824EAC22B8C36C50E341F0FDD4B940852EF85ABA29FFE719DE6A8AE7E43B6A`
  - `src-tauri/Cargo.lock` SHA256: `58B585D3884CE4DA3D5B6C08DD14EB0512941CAF1E6D790A2BAAE1A2BE14F69E`

---

## 2. Artifact Details

- **Release Build Invocation**: `npm.cmd ci` followed by `npm.cmd run build:windows` (`tauri build --target x86_64-pc-windows-msvc --bundles nsis`)
- **Flags / Settings**: No `native-ui-tests` feature, no test root/fixture override in release build.
- **Application Executable**:
  - Path: `src-tauri\target\x86_64-pc-windows-msvc\release\zonendpoint.exe`
  - PE Machine: `0x8664` (x64 / AMD64)
  - SHA256: `A3E8BEE20F366D18E8B358E5B0E8D9B42511EE48F1E7D9B3F4AABA9E802A9401`
- **NSIS Installer Artifact**:
  - Built Path: `src-tauri\target\x86_64-pc-windows-msvc\release\bundle\nsis\Zonendpoint_0.2.1_x64-setup.exe`
  - Staged Path: `E:\Zonendpoint-tools\artifacts\v51-release\Zonendpoint_0.2.1_x64-setup.exe`
  - Checksum Path: `E:\Zonendpoint-tools\artifacts\v51-release\Zonendpoint_0.2.1_x64-setup.exe.sha256`
  - Size: 3,709,433 bytes (~3.54 MiB)
  - SHA256: `342CA889A878E9AAFD20A8CBB42BDCFC491FE88D60A62AAC4EB95FCAAA1DA7C6`
  - Version: `0.2.1` (Retained as current version; bumping to 1.0.0 is an independent decision reserved for Zon)
  - NSIS Generator Version: `v3.13`
  - Code Signing: Unsigned (not code-signed; no release tag / publish performed)

---

## 3. Pre-Build Quality Gates Verification

All tests run against worktree `v51-release`:

| Gate | Command | Result | Notes |
|---|---|---|---|
| Rust Formatting | `cargo fmt -- --check` | **PASS** | 0 diffs |
| Rust Unit & Resilience | `cargo test` | **PASS** | **70/70 PASS** (51 lib, 1 bin, 18 resilience) |
| Rust Clippy | `cargo clippy --all-targets -- -D warnings` | **PASS** | 0 warnings |
| Frontend Build | `npm.cmd run build` | **PASS** | Vite v8.3.5, 23 modules, 0 TS/CSS errors |
| Native Test Suite | `node tests/native/run.mjs` | **PASS** | **20/20 checks PASS** (including V-30 window close idle & active scan, 0 orphan processes) |

*Note on Q-37 observation*: The native regression previously experienced one transient child process leak in Q-37 (run 2 passed). In this V-51 run, all 20 checks passed on the first run; both idle and during-scan window close passed cleanly without remaining child processes or orphan tasks.

---

## 4. Next Steps for Qeen (Steps 3–5 of V/Q-RELEASE-CURRENT)

1. Independent checkout at this handoff snapshot SHA.
2. Run installation tests in dedicated sandbox (`E:\Zonendpoint-tools\installer-sandbox\app`) — **DO NOT install, overwrite, or uninstall Zon's active production app**.
3. Verify actual native File and Folder dialogs (including Cancel/Unicode paths), initial Quick scan scope, Manual Scan return, Stop→Stopping→rescan, theme/icon persistence, and bounded result window.
4. Verify sandbox install/repair/uninstall and data preservation for this specific artifact (`342CA889...`).
5. Record logs and evidence in `docs/qa/` and `E:\Zonendpoint-tools\artifacts\`.

---

## 5. Scope & Limitations

- Unsigned NSIS installer; SmartScreen / Defender warning expected on untrusted machine.
- Microsoft Defender blocks raw EICAR string in transit; use harmless lab-signature procedure for positive detection tests.
- Guard is explicitly excluded from this release candidate (deferred to v1.1).
- No tag created, no merge to `main`.
