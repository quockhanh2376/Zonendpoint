# Handoff: Task V-11 (Fix P1-01 — Cache Threat Name Preservation) — Vic to Qeen

**Date**: 2026-11-08  
**Author**: Victor (Vic)  
**Branch**: `vic/cache-reason`  
**Base Commit**: `ed1088c` (`origin/main`)  
**Code Snapshot SHA**: `d91cc2942cf2055c60234386e5e6a4453b6abb74` (`d91cc29`)  
**Artifacts Directory**: `E:\Zonendpoint-tools\artifacts\d91cc29\`  
**Status**: Ready for independent QA verification by Qeen — No merge  

---

## 1. Problem Description (Finding P1-01)

Previously, `scan_cache` in `cache.rs` only stored `status` (`'CLEAN'` or `'INFECTED'`) without a `threat_name` column. When a file detected as `INFECTED` (for instance, by YARA rule `YARA.ZonQA_Benign_Marker.UNOFFICIAL` or ClamAV signature) was scanned a second time, the cache hit logic in `scanner.rs` (~line 538) assigned a hardcoded reason:
```rust
Some("verified content cache hit".into())
```
This overwrote the specific malware/threat signature name with `"verified content cache hit"`, obscuring the threat identity on repeated scans.

---

## 2. Summary of Implementation & Fixes

### A. Database Schema Migration (Safe & Transactional)
- **Schema version bump**: PRAGMA `user_version` updated from `1` to `2`.
- **New column**: `threat_name TEXT` added to `scan_cache`.
- **Migration paths**:
  - `version == 0`: Creates `scan_cache` directly with `threat_name TEXT`, sets `user_version = 2`.
  - `version == 1`: Executes `ALTER TABLE scan_cache ADD COLUMN threat_name TEXT;`, sets `user_version = 2`.
  - `version > 2`: Returns error if an unsupported future version is detected.
- **Legacy Cache Migration Safety**: If a legacy database contains `INFECTED` rows with `threat_name IS NULL` (from schema v1), `cache::get()` treats them as a **cache miss** (`Ok(None)`). This forces a live re-scan, automatically updating the entry with the actual threat name on the next scan.
- **CLEAN rows**: Legacy and new `CLEAN` rows retain `threat_name = None` and continue to hit cache normally with `"verified content cache hit"`.

### B. Cache Verdict & Retrieval (`src-tauri/src/cache.rs`)
- `CachedVerdict` updated:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct CachedVerdict {
      pub status: ScanStatus,
      pub threat_name: Option<String>,
  }
  ```
- `cache::put` updated:
  ```rust
  pub fn put(
      path: &Path,
      sha256: &[u8; 32],
      engine_version: &str,
      policy_version: &str,
      status: ScanStatus,
      threat_name: Option<&str>,
      scanned_at: i64,
  ) -> Result<()>
  ```
  Strict invariant: If `status == ScanStatus::Infected`, `threat_name` must be present and non-empty; otherwise `put()` returns an error.

### C. Scanner Integration (`src-tauri/src/scanner.rs`)
- On cache hit (~line 538):
  ```rust
  let reason = match cached.status {
      ScanStatus::Infected => cached.threat_name,
      _ => Some("verified content cache hit".into()),
  };
  let mut result = ScanResult::new(
      &path,
      cached.status,
      Some(hex::encode(hash)),
      reason,
      size,
  );
  ```
- On live scan completion (~line 604):
  `cache::put` receives `result.reason.as_deref()`, storing the exact threat name when infected.

### D. Security Invariants Preserved
- `ERROR`, `UNKNOWN`, `CANCELLED`, `SKIPPED` are never cached and can never become `CLEAN`.
- SQL `CHECK(status IN ('CLEAN', 'INFECTED'))` and `CHECK(length(sha256) = 32)` are strictly maintained.

---

## 3. Artifacts & Checksums

Directory: `E:\Zonendpoint-tools\artifacts\d91cc29\`

| Artifact | File Size | SHA256 Checksum | Description |
|---|---|---|---|
| `zonendpoint.exe` | 8,067,584 bytes | `AE298A429F0B3B8D3EFF6AEB2A694BF3C32092BBF115B65DAD59541AAC0618F7` | Standalone x64 release executable |
| `Zonendpoint_0.1.0_x64-setup.exe` | 3,690,633 bytes | `34E5DE6027FB89029CE951D2D33F68C2007D2354E2965854CBBD6E944F1E0E7E` | NSIS x64 installer |

---

## 4. Verification Evidence & How to Test

### A. Automated Unit Tests (39/39 PASS)
Run:
```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib cache::tests
```
Verified test suite:
- `cache::tests::cache_hit_preserves_threat_name_for_infected` — PASS
- `cache::tests::cache_hit_clean_preserves_clean_verdict_without_threat_name` — PASS
- `cache::tests::legacy_v1_database_migrates_safely_and_treats_unnamed_infected_as_miss` — PASS
- `cache::tests::error_and_unknown_never_cached_and_never_become_clean` — PASS
- `cache::tests::infected_without_threat_name_fails_to_cache` — PASS
- `cache::tests::cache_rejects_sha256_values_not_32_bytes` — PASS
- `cache::tests::cache_hit_requires_content_engine_policy_and_fresh_ttl` — PASS
- `cache::tests::legacy_cache_migration_discards_unverifiable_metadata_rows` — PASS
- `cache::tests::failed_migration_keeps_legacy_table_transactionally` — PASS

### B. Integration & Resilience Tests (18/18 PASS)
Run:
```bash
cargo test --manifest-path src-tauri/Cargo.toml --test resilience
```
Key tests verified:
- `test_infected_cache_hit_preserves_exact_threat_name`:
  Scans file with mock daemon returning `YARA.ZonQA_Benign_Marker.UNOFFICIAL`.
  First scan: `ScanStatus::Infected`, `reason: Some("YARA.ZonQA_Benign_Marker.UNOFFICIAL")`.
  Second scan (cache hit): `ScanStatus::Infected`, `reason: Some("YARA.ZonQA_Benign_Marker.UNOFFICIAL")`.
- `test_clean_cache_hit_preserves_verified_content_cache_hit_message`:
  Scans clean file.
  First scan: `ScanStatus::Clean`.
  Second scan (cache hit): `ScanStatus::Clean`, `reason: Some("verified content cache hit")`.

### C. Linting & Frontend Build
- `cargo fmt -- --check`: PASS (0 diffs)
- `cargo clippy --all-targets -- -D warnings`: PASS (0 warnings)
- `npm run build`: PASS (built in 364ms)
- `npx tauri build --bundles nsis`: PASS (NSIS installer created at `bundle/nsis/Zonendpoint_0.1.0_x64-setup.exe`)

### D. Manual Verification Instructions for Qeen
1. Run `zonendpoint.exe` or install via `Zonendpoint_0.1.0_x64-setup.exe`.
2. Connect to local/lab clamd.
3. Scan a benign test file containing `ZonQA_Benign_Marker` (`ZONQA_BENIGN_MARKER_TEST_STRING_20261108_CLAMAV_LAB`).
4. Observe the first scan: threat name renders as `YARA.ZonQA_Benign_Marker.UNOFFICIAL`.
5. Click **Scan** again on the same file without clearing or restarting clamd.
6. Verify the second scan hits the cache and still displays `YARA.ZonQA_Benign_Marker.UNOFFICIAL` (NOT "verified content cache hit").
7. Scan a clean file twice: verify the second scan displays the clean result with green checkmark and "All Good".
