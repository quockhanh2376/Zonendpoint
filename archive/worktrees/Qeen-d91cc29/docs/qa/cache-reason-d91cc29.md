# Qeen QA Report: Task Q-9 — Verification of Fix P1-01 (Cache Threat Name Preservation)

**Date**: 2026-11-08  
**QA Agent**: Qeen  
**Implementation Commit**: `d91cc2942cf2055c60234386e5e6a4453b6abb74` (`d91cc29`)  
**Worktree**: `E:\Zonendpoint-Qeen-d91cc29` (detached at `d91cc29`)  
**Artifacts Directory**: `E:\Zonendpoint-tools\artifacts\d91cc29\`  
**Evidence Directory**: `E:\Zonendpoint-tools\artifacts\d91cc29\qeen\`  
**Verdict**: **PASS** (All 5 acceptance criteria verified; 0 defects found)

---

## 1. Artifact Verification & Build Integrity

| Artifact | File Size | SHA256 Checksum | Result |
|---|---|---|---|
| `zonendpoint.exe` | 8,067,584 bytes | `AE298A429F0B3B8D3EFF6AEB2A694BF3C32092BBF115B65DAD59541AAC0618F7` | **PASS** (Matches handoff exactly) |
| `Zonendpoint_0.1.0_x64-setup.exe` | 3,690,633 bytes | `34E5DE6027FB89029CE951D2D33F68C2007D2354E2965854CBBD6E944F1E0E7E` | **PASS** (Matches handoff exactly) |

- **Unit & Integration Tests**: `cargo test --manifest-path src-tauri/Cargo.toml`  
  - 39/39 lib unit tests PASS
  - 1/1 main unit tests PASS
  - 18/18 resilience integration tests PASS  
  - **Total**: 58 passed; 0 failed; 0 ignored.
- **Code Formatting**: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` → **PASS** (0 diffs).
- **Linter**: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` → **PASS** (0 warnings).
- **Frontend Build**: `npm run build` (`tsc && vite build`) → **PASS** (built in 552ms, 0 errors).

---

## 2. Fix P1-01 Verification: Cache Threat Name Preservation

- **First Scan (Live Detection)**:
  - Intercepted INSTREAM with test threat signature `Win.Test.CustomThreat.P101`.
  - UI displayed outcome `Scan complete` with `1 threat detected`.
  - Result row rendered:
    - Status: `Threat detected`
    - Reason: `Win.Test.CustomThreat.P101`
  - In SQLite `cache.db`: Recorded row with `status = 'INFECTED'`, `threat_name = 'Win.Test.CustomThreat.P101'`.
- **Second Scan (Cache Hit)**:
  - Proxy placed in `passthrough` mode (no fake threats injected; would return CLEAN if queried).
  - Second Quick Scan completed in 3.0s purely via cache hit.
  - UI outcome: `Scan complete` with `1 threat detected`.
  - Result row rendered:
    - Status: `Threat detected`
    - Reason: `Win.Test.CustomThreat.P101`
  - **Critical Assertion**: The reason is strictly preserved as `Win.Test.CustomThreat.P101` and is **NOT** overwritten by `"verified content cache hit"`.
  - **Clean Files Comparison**: Clean files on cache hit correctly retain clean status without displaying threat reasons.

---

## 3. Database Migration & Schema Invariants

- **Schema v1 → v2 Migration**:
  - Initialized sandbox with real schema v1 database (`PRAGMA user_version = 1`, table without `threat_name` column, containing 593 CLEAN entries and 1 unnamed INFECTED entry).
  - Upon scan start, `cache::open()` transactionally executed `ALTER TABLE scan_cache ADD COLUMN threat_name TEXT;` and bumped `PRAGMA user_version = 2`.
  - Application did not crash; table structure preserved.
  - **Legacy INFECTED Handling**: `cache::get()` treated the legacy INFECTED row (`threat_name IS NULL`) as a **cache miss** (`Ok(None)`). This triggered a live re-scan, which successfully updated the entry with the actual threat name `Win.Test.CustomThreat.P101`.
- **Future/Unknown Version Handling**:
  - Databases with `PRAGMA user_version > 2` (e.g. version 99) are strictly rejected with error `cache schema version 99 is newer than supported version 2`.
  - The application handles this gracefully: **never crashes** and **never reports CLEAN**.
- **Constraint Invariants**:
  - Table schema enforces `CHECK(status IN ('CLEAN', 'INFECTED'))` and `CHECK(length(sha256) = 32)`.
  - Attempts to insert `ERROR`, `UNKNOWN`, `CANCELLED`, or `SKIPPED` are rejected at the database level.
  - Transient errors and timeouts never enter cache and can never become CLEAN.

---

## 4. Regression & UI State Verification

- **Status Dot**:
  - Displays green dot (`data-status="connected"`) when daemon/proxy is reachable on port 13399/13310.
  - Displays red/gray dot (`data-status="disconnected"`) when daemon drops connection.
- **Offline Notice**:
  - When daemon is disconnected, banner `.notice.error` displays `"Cannot connect to ZonEndpoint server"`.
  - Quick Scan attempts fail immediately at preflight; offline state **never reports CLEAN**.
- **Quick Scan All Good**:
  - With infected row removed and 587 clean entries cached, Quick Scan completed in <2s.
  - UI displayed `.all-good-card`, `.all-good-check` SVG icon, and heading `All Good`.
  - Primary button displayed `Quick Scan` ready for subsequent scans.

---

## 5. Environment & Teardown Verification

- **Sandbox Cleanup**: Sandbox directories under `E:\Zonendpoint-tools\artifacts\d91cc29\qeen\sandbox\` were uninstalled via `uninstall.exe /S` and completely removed.
- **Lab SSH Tunnel**: SSH tunnel on `127.0.0.1:13310` was preserved throughout testing and verified active (`PONG`).
- **No Malware on Disk**: All testing strictly utilized network proxy protocol rewrites and benign fixtures; no EICAR or live malware existed on disk.

---

## 6. Evidence Files

All evidence generated during independent QA is archived under `E:\Zonendpoint-tools\artifacts\d91cc29\qeen\`:
- `step1-initial.png`: Window initial idle state, connected status dot.
- `step2-first-scan-threat.png`: Live scan outcome rendering `Win.Test.CustomThreat.P101`.
- `step3-cache-hit-preserves-threat.png`: Cache hit outcome confirming threat name preservation.
- `step4-offline-notice.png`: Offline status dot and error banner.
- `step5-allgood.png`: Clean Quick Scan All Good screen.
- `qeen-report-d91cc29.json`: Structured machine-readable test execution report.
- `qeen-verify-p101.mjs`: Automated visible WebDriverIO test script.
- `qeen-proxy.mjs`: Loopback proxy harness.
- `test-cache-migration-unit.mjs`: Database migration and SQLite invariant test script.
