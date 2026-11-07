# Handoff: Quick Scan Scope Update & Finding Q-4-01 Resolution — Vic to Qeen

- **Date:** 2026-11-07
- **From:** Victor (Vic) — Lead Developer
- **To:** Qeen — Independent Tester & Security Reviewer
- **Branch:** `vic/ui-simplify` in `E:\Zonendpoint-vic-ui`
- **Target Snapshot / Source SHA:** `629450630b7da9db3fe0a1d69d4ae09559cf8015` (`6294506`)
- **Parent Commits:**
  - `bbdf68a`: `docs(qa): import qeen q-4 ui-simplify verification report and handoff for 0a55516`
  - `6294506`: `fix(quick): exclude %SystemRoot%\System32\drivers\DriverData from Quick Scan (Q-4-01)`

---

## 1. Exact Artifacts & Checksums

Artifacts have been built and staged in `E:\Zonendpoint-tools\artifacts\6294506\`:

| Artifact | Path | Size (bytes) | SHA256 | Target Architecture |
|---|---|---|---|:---:|
| **NSIS Installer** | `E:\Zonendpoint-tools\artifacts\6294506\Zonendpoint_0.1.0_x64-setup.exe` | 3,690,242 | `C4571323064D090FD4A7CCA8E798021E60380452DC9442A99C60650D7B9AC30C` | x86_64-pc-windows-msvc (NSIS bundle) |
| **Standalone Executable** | `E:\Zonendpoint-tools\artifacts\6294506\zonendpoint.exe` | 8,065,024 | `038689F3C3DD4E69BBA3C40A9BC014FCFA678D8592D15AB2D39623CD95E0810C` | x86_64-pc-windows-msvc (x64) |

*Note: No installer run was executed by Vic. No merge to main.*

---

## 2. What Changed (Resolution of Finding Q-4-01)

### Decision & Scope
- **Zon's Decision:** "Remove DriverData from Quick Scan".
- **Documentation:** Updated `docs/v1.0.0-scope.md` under `### Quick scan exclusions (DriverData)` documenting the exact exclusion, the Windows ACL rationale (`NT AUTHORITY\SYSTEM` restriction), and the security invariants.

### Implementation Details
1. **Explicit Quick Scan Exclusions:**
   - In `src-tauri/src/quick.rs`, defined `QUICK_EXCLUSIONS = ["DriverData"]` and `exclusions_for_root(root: &Path) -> Vec<PathBuf>` producing `%SystemRoot%\System32\drivers\DriverData`.
   - Added canonical path resolution and case-insensitive component matching (`canonical_path`, `is_path_excluded`, `path_matches_exclusion`). Matches require equality or descent from the canonical exclusion path; no substring matching or loose name matching elsewhere.
2. **Directory Walker Exclusion Handling:**
   - In `src-tauri/src/directory.rs`, introduced `scan_roots_with_exclusions(roots, exclusions, cancel, scan, progress)`.
   - During directory traversal, when an entry or directory matches `exclusions`, the directory iterator calls `skip_current_dir()` and skips it.
   - Any WalkDir error on an excluded path is cleanly ignored.
   - Excluded items are not part of the Quick Scan target set: **not counted, not shown, no row emitted**.
3. **Preservation of Security Invariant:**
   - Any OTHER unreadable, missing, or access-denied file/folder in Quick Scan still emits `ScanStatus::Error` / `ScanStatus::Skipped`, leading to `Scan incomplete` and the mandatory warning *"Do not treat an incomplete scan as clean."*, never `All Good`.
   - User-initiated File and Folder scans (`scan_path`) pass empty exclusions (`Vec::new()`). If a user selects `%SystemRoot%\System32\drivers` via Folder scan, `DriverData` is NOT excluded; access denial is reported as an error and marks the scan incomplete.
   - Folders named `DriverData` located anywhere other than `%SystemRoot%\System32\drivers\DriverData` are never excluded.
4. **Wiring in Tauri Commands:**
   - `quick_scan` passes `quick::exclusions_for_root(&root)`.
   - `scan_path` passes `Vec::new()` (unaffected).

---

## 3. Automated Verification Results

- `cargo test --locked`: **PASS** (51 tests: 34 lib + 1 main + 16 resilience).
  - TDD (a): `tdd_driverdata_under_quick_root_excluded` — DriverData skipped, scan finishes complete with 0 errors.
  - TDD (b): `tdd_different_unreadable_dir_under_drivers_yields_incomplete` — non-excluded unreadable directory yields error and incomplete scan.
  - TDD (c): `tdd_folder_named_driverdata_elsewhere_not_excluded` — folder named `DriverData` outside quick root is scanned normally.
  - TDD (d): `tdd_folder_scan_of_drivers_reports_denied_subdir` — Folder scan of `drivers` still catches `DriverData` access denial and reports incomplete scan.
- `cargo fmt --check`: **PASS** (Zero formatting diffs).
- `cargo clippy --locked --all-targets -- -D warnings`: **PASS** (0 warnings).
- `npm.cmd run build`: **PASS** (Frontend Vite build successful).
- Native mock test suite (`npm.cmd run test:native`): **PASS** (14/14 checks passed, including Quick Scan with mock DriverData fixture).

---

## 4. What Qeen Must Re-Check

1. **Quick Scan All Good on Live System:**
   - Run the primary action button (`Quick Scan`) against the live ClamAV daemon tunnel (`127.0.0.1:13310`).
   - Verify Quick Scan traverses `C:\Windows\System32\drivers` without failing on `DriverData`.
   - Verify the scan achieves **"All Good"** on clean system files (clean count ~588, errors 0, no `DriverData` row).
2. **Security Invariant on Folder Scan:**
   - Select target menu -> "Folder" -> choose `C:\Windows\System32\drivers` (or mock folder with denied subfolder) -> click "Scan".
   - Verify that this Folder scan DOES encounter access denial on `DriverData` (`os error 5`) and correctly reports **"Scan incomplete"** with the yellow warning *"Do not treat an incomplete scan as clean"*.
3. **Artifact Integrity:**
   - Verify SHA256 of `E:\Zonendpoint-tools\artifacts\6294506\Zonendpoint_0.1.0_x64-setup.exe` and `zonendpoint.exe`.
