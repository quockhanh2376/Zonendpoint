# QA Report: Quick Scan Scope Update & Finding Q-4-01 Resolution (Build 6294506)

- **Date:** 2026-11-07
- **Reviewer:** Qeen — Independent Tester & Security Reviewer
- **Target Snapshot / Commit:** `629450630b7da9db3fe0a1d69d4ae09559cf8015` (`6294506`)
- **Worktree:** `E:\Zonendpoint-Qeen-6294506` (detached from `E:\Zonendpoint-restored\Zonendpoint-vic`)
- **Vic Worktree / Baseline:** Preserved untouched (`E:\Zonendpoint-vic-ui` and `E:\Zonendpoint-restored\Zonendpoint-vic`)
- **Evidence Directory:** `E:\Zonendpoint-tools\artifacts\6294506\qeen\`

---

## 1. Verified Artifacts & Provenance

| Artifact | Staged Path | Size (bytes) | SHA256 Hash | Provenance / Status |
|---|---|---|---|---|
| **Installer (NSIS x64)** | `E:\Zonendpoint-tools\artifacts\6294506\Zonendpoint_0.1.0_x64-setup.exe` | 3,690,242 | `C4571323064D090FD4A7CCA8E798021E60380452DC9442A99C60650D7B9AC30C` | Exact match with handoff |
| **Standalone Executable** | `E:\Zonendpoint-tools\artifacts\6294506\zonendpoint.exe` | 8,065,024 | `038689F3C3DD4E69BBA3C40A9BC014FCFA678D8592D15AB2D39623CD95E0810C` | Exact match with handoff |
| **Installed Sandbox Executable** | `.../sandbox/app/zonendpoint.exe` | 8,065,024 | `FA5199A988FDB6D1F2432F1B5CA6F05A6AB5E07CCE3E6DF1F73F907892779852` | Verified byte-identical to built binary except permitted 3-byte Tauri NSIS bundle marker (`UNK` -> `NSS` at offset 6,172,855) |

---

## 2. Checks & Results Summary

| Check | Description | Status | Evidence & Metrics |
|---|---|---|---|
| **A** | Automated suites (`cargo test --locked`, `fmt --check`, `clippy -D warnings`, `npm run build`) & Code Review | **PASS** | 51 Rust tests (34 lib + 1 main + 16 resilience) PASS; 0 fmt diffs; 0 clippy warnings; Vite build finished in 621ms. Review of `quick.rs`, `directory.rs`, and `lib.rs` confirms exclusion is strictly canonical `%SystemRoot%\System32\drivers\DriverData`; junctions/symlinks are skipped; 8.3 short names canonicalized; File/Folder scans pass empty exclusions. |
| **B** | Visible live Quick Scan through real tunnel (`127.0.0.1:13310`) | **PASS** | Default button "Quick Scan" executed live on desktop. Finishes with a single "All Good" line (`.all-good-line`), no `DriverData` row emitted, errors 0. Full initial scan duration: ~14.4s (588 clean files); cached rescan duration: 2,995ms. Screenshot: `check-b-all-good.png`. |
| **C1** | Invariant: Forced error via proxy midstream | **PASS** | Injected 1 malformed clamd response via `qeen-proxy.mjs` on `zINSTREAM`. UI transitioned to `Scan incomplete`, warning shown: *"Do not treat an incomplete scan as clean."*, 1 error row visible, never `All Good`. Screenshot: `check-c-scan-incomplete.png`. |
| **C2** | Invariant: Lab tunnel stopped preflight | **PASS** | Stopped SSH tunnel on 13310. Quick Scan preflight immediately emitted error notice *"Cannot connect to ZonEndpoint server"*, 0 files scanned, cached clean files never displayed `All Good`. Tunnel successfully restarted and verified. Screenshot: `check-c-tunnel-stopped.png`. |
| **D** | Folder scan of `C:\Windows\System32\drivers` (DriverData access denial) | **NEEDS ZON MANUAL TEST** | Dialog automation opened Win11 `#32770` dialog (`DIALOG_FOUND`), but programmatic input without test injection in release mode is restricted by Windows UIPI. Rust engine behavior verified in unit test `tdd_folder_scan_of_drivers_reports_denied_subdir` (PASS). Needs 1-click manual test by anh Zon. |
| **E** | Cleanup & Tunnel Verification | **PASS** | Sandbox cleanly uninstalled via `uninstall.exe /S` and removed; test processes stopped; lab tunnel on 13310 verified LISTENING (`TcpTestSucceeded: True`). |

---

## 3. Detailed Security & Architectural Code Review (Check A)

1. **Exclusion Scope (`src-tauri/src/quick.rs`):**
   - Defined `pub const QUICK_EXCLUSIONS: [&str; 1] = ["DriverData"];`.
   - `exclusions_for_root(root: &Path) -> Vec<PathBuf>` returns `vec![root.join("drivers").join("DriverData")]`.
   - `canonical_path` resolves canonical paths via `std::fs::canonicalize`, stripping Windows verbatim `\\?\` prefixes and normalizing path components. If a denied directory cannot be fully canonicalized, it walks up to the nearest canonical ancestor and normalizes remaining components.
   - 8.3 short names (e.g. `PROGRA~1` or short driver folder names) are resolved to canonical long names.
   - `path_matches_exclusion` requires that the candidate path has at least as many components as the exclusion path and that all leading components match case-insensitively. Substring matching or partial name matching elsewhere (e.g. `DriverDataBackup`, `C:\Windows\DriverData`) is rejected.

2. **Traversal Safety (`src-tauri/src/directory.rs`):**
   - WalkDir config preserves `follow_links(false)` and `follow_root_links(false)`. Symlinks and junctions pointing to or from `DriverData` are never traversed and are handled as skipped symlink entries.
   - When an entry matches `exclusions`, the directory iterator calls `it.skip_current_dir()`, entirely bypassing descent into the denied folder.
   - Any WalkDir error on an excluded path is silently ignored, preventing `ScanStatus::Error` from being emitted for `DriverData`.
   - Excluded items are not counted, not added to summary, and emit no UI row.

3. **Invariants Preserved (`src-tauri/src/lib.rs`):**
   - `scan_path` (File / Folder scan commands) passes `Vec::new()` as exclusions. When a user explicitly scans `C:\Windows\System32\drivers` via Folder scan, `DriverData` is NOT excluded; Windows ACL denial (`os error 5`) is caught and reported as an error, marking the scan incomplete.
   - Any unreadable or missing file/folder outside `DriverData` during Quick Scan still emits `ScanStatus::Error` / `ScanStatus::Skipped`, leading to `Scan incomplete` and the mandatory warning.

---

## 4. Verification Evidence & Artifact Manifest

Located under `E:\Zonendpoint-tools\artifacts\6294506\qeen\`:
- `install.mjs`: Sandbox installer runner and SHA256 provenance verifier.
- `check-payload.mjs`: Byte-level comparator verifying the 3-byte Tauri NSIS bundle marker difference.
- `qeen-proxy.mjs`: Loopback fault-injection proxy on 127.0.0.1:13399 (Control: 13398).
- `qeen-suite-6294506.mjs`: Complete WebDriver automation suite executing visible UI tests.
- `cleanup.mjs`: Sandbox uninstaller and teardown script.
- `report.json`: Machine-readable results for all checks.
- `check-b-all-good.png`: Screenshot of Quick Scan achieving All Good with DriverData excluded.
- `check-c-scan-incomplete.png`: Screenshot of forced-error invariant displaying Scan Incomplete.
- `check-c-tunnel-stopped.png`: Screenshot of stopped tunnel displaying connection error notice.
