# Handoff: Q-4 UI-Simplify & P0-01 Verification Findings — Qeen to Vic

- **Date:** 2026-11-07
- **From:** Qeen (Independent Tester & Security Reviewer)
- **To:** Victor (Vic) — Lead Developer
- **Target Snapshot / SHA:** `0a555169fb6e72db82456e568f385982a8021e39` (`0a55516`)
- **Report Reference:** `docs/qa/ui-simplify-0a55516.md`
- **Evidence Directory:** `E:\Zonendpoint-tools\artifacts\0a55516\qeen\`

---

## 1. Summary of Verification

1. **Artifact Verification:**
   - Installer (`Zonendpoint_0.1.0_x64-setup.exe`): SHA256 `A0C9E63F3358C92E3EC56022C7DB16287D055CB190BAF4AD4DC45C3D40727252` — **MATCH**.
   - Executable (`zonendpoint.exe`): SHA256 `AF414BC52E8D06015243CAFDDA5F0712ABB29F5AF7565F54B4A495E948367FD4` — **MATCH**.
2. **P0-01 Resilience Fix Verification:**
   - 5/5 unit tests passed.
   - Live proxy injection with delayed second frames (50ms, 500ms) and never-closing socket verified: All rejected as `ScanStatus::Error`, bounded by `EOF_DRAIN_TIMEOUT` (1.0s), 0 cache entries written, UI correctly reports `Scan incomplete` and never `All Good` or `Clean`. **P0-01 is RESOLVED.**
3. **UI-Simplify Verification:**
   - Select target menu: File & Folder only (**PASS**).
   - Default primary button: `Quick Scan` (**PASS**).
   - Copy sanitization: Zero occurrences of "ClamAV", IPs, or ports in UI or built assets (**PASS**).
   - Preflight offline and invalid banner: Single clean notice "Cannot connect to ZonEndpoint server", no cache reuse (**PASS**).
   - Server status indicator: Green -> Red with tooltip -> Click re-check without UI freeze -> Green (**PASS**).

---

## 2. Findings Requiring Fix

### Finding Q-4-01 (High / Usability Blocker): Quick Scan fails with `Access is denied (os error 5)` on `DriverData`
- **Severity:** High / Usability Blocker
- **Affected File(s):** `src-tauri/src/quick.rs`, `src-tauri/src/directory.rs`
- **Observed Behavior:**
  - In release builds, `scan_quick` targets `C:\Windows\System32\drivers`.
  - Windows protects `C:\Windows\System32\drivers\DriverData` with ACLs restricted to `NT AUTHORITY\SYSTEM`.
  - Standard user and administrator tokens without SYSTEM privileges encounter `IO error for operation on C:\WINDOWS\system32\drivers\DriverData: Access is denied. (os error 5)`.
  - Consequence: Running the default Quick Scan out of the box always processes 588 clean files + 1 error on `DriverData`, resulting in heading `"Scan incomplete"` and warning *"Do not treat an incomplete scan as clean"*.
  - A standard user running a release build can **never** achieve an `"All Good"` scan via the default primary button.
- **Recommended Remediation:**
  - Explicitly exclude `DriverData` (or directories with unreadable metadata ACLs) when walking `drivers` during Quick Scan, or configure the directory iterator in `quick.rs` to ignore `Access is denied` specifically on system driver log/metadata directories.
