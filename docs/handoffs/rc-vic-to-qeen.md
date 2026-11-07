# Handoff: Release Candidate for Zon (RC) — Vic to Qeen

- **Date:** 2026-11-07
- **From:** Victor (Vic) — Lead Developer
- **To:** Qeen — Independent Tester & Security Reviewer
- **Branch:** `vic/ui-simplify` in `E:\Zonendpoint-vic-ui`
- **Target Snapshot / Source SHA:** `79f0f596a8fc3a6a17d093283fedd09693c72847` (`79f0f59`)
- **Base / Reference Commit:** `e1a48d3`

---

## 1. Exact Artifacts & Checksums

The Release Candidate (RC) artifacts have been compiled and staged in `E:\Zonendpoint-tools\artifacts\79f0f59\`:

| Artifact | Path | Size (bytes) | SHA256 | Target Architecture |
|---|---|---|---|:---:|
| **NSIS Installer** | `E:\Zonendpoint-tools\artifacts\79f0f59\Zonendpoint_0.1.0_x64-setup.exe` | 3,690,442 | `3F69A4D54BFB6B71DE671FB76320DBB6BF4A3968D53DE8621304B1319483F807` | x86_64-pc-windows-msvc (NSIS bundle) |
| **Standalone Executable** | `E:\Zonendpoint-tools\artifacts\79f0f59\zonendpoint.exe` | 8,065,024 | `94C42548B59D34A2C5900F5945FB103A248EEA77671D01F8972B098801CA7257` | x86_64-pc-windows-msvc (x64) |

*Note: No installer run was executed by Vic. No merge to main.*

---

## 2. Full List of Changes Since `e1a48d3`

This build consolidates all UI simplification, resilience hardening, security fixes, and layout polish completed on `vic/ui-simplify`:

### 1. UI Simplification & Preflight (`8c687b9`)
- **Target Selector Dropdown:** Simplified to "File" and "Folder" (removed redundant "Quick Scan" option).
- **Dynamic Primary Button:** Defaults to "Quick Scan" when idle; switches to "Scan" when a custom File or Folder target is selected.
- **Auto-Return Post-Scan:** After completing a File or Folder scan, the active target selection clears automatically and the primary action button returns to "Quick Scan", while retaining the scan outcome on screen.
- **Summary-First Results Presentation:**
  - 100% Clean scans display a clean single-line **"All Good"** card with green checkmark icon (no verbose rows).
  - Infected scans display only threat rows (`INFECTED`).
  - Incomplete / Error scans display non-clean rows (`ERROR`, `SKIPPED`, `CANCELLED`) accompanied by the mandatory warning banner: *"Do not treat an incomplete scan as clean."*
- **Server Status Indicator & Clamd Preflight:**
  - Status dot in topbar header (green = connected, red = disconnected, grey = checking) with 30s background poll.
  - Preflight checks `zVERSION` before scan execution; connection errors display user-friendly copy (*"Cannot connect to ZonEndpoint server"*) without leaking daemon details or caching failures as clean.
- **Window Dimensions:** Compact default window size (800x240), dynamically expanding only when results list is displayed.

### 2. Protocol Frame & EOF Drain Hardening — P0-01 (`0a55516`)
- Required bounded EOF drain (`read_to_end` with remaining scan timeout and response cap) immediately following parsed clamd response frame.
- Prevents frame desynchronization and connection reuse contamination from pipelined or delayed extra daemon data.

### 3. Quick Scan Scope Exclusion for DriverData — Q-4-01 (`6294506`)
- Excluded `%SystemRoot%\System32\drivers\DriverData` from the Quick Scan target manifest due to Windows SYSTEM-only ACL restrictions.
- Traversal skips this directory cleanly without counting, without error, and without emitting an error row, enabling clean systems to achieve "All Good".
- Security invariant preserved: user-initiated Folder scans of `drivers` and any other inaccessible subfolder continue to report access denial and fail as `Scan incomplete`.

### 4. Long Threat Name Text Wrapping — Q-5-01 (`79f0f59`)
- Added `overflow-wrap: anywhere;` to `.results small` in `src/App.css` (matching `.result-path`).
- Prevents unbroken long threat signatures (150+ characters) from expanding layout or causing horizontal scrollbars in the `.results` container.
- Added native mock check verifying `scrollWidth <= clientWidth` in `.results` in both Light and Dark themes.

---

## 3. Automated Quality Gate Summary

All automated quality gates passed cleanly on source SHA `79f0f59`:

- `cargo test --locked`: **51/51 PASS** (34 lib unit tests + 1 main CLI test + 16 resilience integration tests).
- `cargo fmt --check`: **PASS** (Zero formatting issues).
- `cargo clippy --locked --all-targets -- -D warnings`: **PASS** (0 warnings).
- `npm.cmd run build`: **PASS** (Vite + TypeScript build clean).
- `npm.cmd run test:native`: **15/15 PASS** (including the new 150+ char threat name text wrapping test across Light and Dark themes).

---

## 4. Release Candidate Status & Manual Testing Scope for Zon

This artifact package (`79f0f59`) is the official **Release Candidate (RC)** prepared for Zon's manual review.

The following manual tests remain to be performed by Zon (due to Windows UIPI preventing automated native file dialog clicks):
1. **File / Folder Dialog Picker:** Click "Select target ▾" -> choose "File" or "Folder", pick a target from native Windows explorer dialog, and verify selected path displays properly.
2. **Auto-Return Lifecycle:** Run a scan on the selected File or Folder, verify completion, verify primary button reverts to "Quick Scan", and verify target selection clears.
3. **Single "All Good" Line:** Run a scan on a single known clean file and observe the single-line "All Good" card with green checkmark.
