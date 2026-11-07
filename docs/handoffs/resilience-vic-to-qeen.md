# LAB-RESILIENCE Handoff — Vic to Qeen

**Date:** 2026-11-07  
**Author:** Victor (Vic)  
**Target:** Qeen (Independent Tester & Security Reviewer)  
**Branch:** `vic/lab-resilience-plan` at `E:\Zonendpoint-vic-resilience`  
**Base Commit Merged:** `e000c01` (`vic/mvp-first-scan`)  

---

## 1. Summary of Changes & Verifications

This handoff delivers the **Tier 1 deterministic integration suite** and the **Tier 2 Node.js zero-dependency fault injection harness** for the `LAB-RESILIENCE` milestone.

### 1.1 Product Code Defect Fixes & Hardening (Superpowers TDD)
1. **P0-01 (Pipelined / Multiple Frame Injection):** `stream_scan` in `src-tauri/src/scanner.rs` now checks for unexpected trailing data in the socket receive buffer (`socket.try_read`) immediately after the terminal NUL of a single response. Multi-frame payloads (e.g. `stream: OK\0stream: Threat FOUND\0`) are immediately rejected as `ScanStatus::Error` and never cached.
2. **P0-02 (Stale Scan ID / Lifecycle Isolation):** Added `active_scan_id()` to `ScanManager` and proved that concurrent scans are rejected, cancelled scans transition cleanly, and delayed `finish_scan` calls from an earlier scan cannot terminate or corrupt a subsequent scan (`test_stale_scan_race_dropped`).
3. **P1-01 (Engine Version & Signature Validation):** `clamd_version` now strictly enforces that the version string contains a valid signature component (delimited by `/`, minimum 2 segments). Incomplete strings like `ClamAV 1.5.4` without signature info are rejected, preventing signature cache poisoning.
4. **P1-02 (Slowloris Read Timeout):** `read_nul_frame` wraps every chunk read in a 5-second timeout, preventing indefinite worker lockup if a rogue daemon trickles bytes slowly.
5. **P1-03 (ClamAV Limit Framing):** `parse_response` now recognizes terminal `" ERROR"` frames that lack the `"stream: "` prefix, specifically handling `INSTREAM size limit exceeded. ERROR\0` and preserving the exact policy error message in `ScanResult.reason`.
6. **P2-02 (Threat Name Retention):** For `ScanStatus::Infected`, the parsed threat name (e.g. `Win.Test.CustomThreat-42`) is now preserved in `ScanResult.reason` rather than discarded.
7. **Decision 1 (Parameterized Test Timeout):** `scan_file_with_timeout` and `scan_file_with_engine_timeout` accept an explicit `Duration` timeout parameter in Rust tests (avoiding env vars). Test-gated hook `default_scan_timeout()` only reads `ZONENDPOINT_TEST_SCAN_TIMEOUT_MS` when `feature = "native-ui-tests"` AND `debug_assertions` are active. Release builds reject `native-ui-tests` via `compile_error!`.

---

## 2. Observed Verification Commands & Results

All commands executed in `E:\Zonendpoint-vic-resilience`:

| Check | Command | Result |
|---|---|---|
| **Rust Tests** | `cargo test --manifest-path src-tauri/Cargo.toml --locked` | **PASS**: 37 tests passed (25 library, 1 CLI, 11 resilience integration tests), 0 failures. |
| **Rust Formatting** | `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | **PASS**: Exit code 0, 0 diffs. |
| **Rust Clippy** | `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | **PASS**: Exit code 0, 0 warnings. |
| **Frontend Build** | `npm.cmd run build` | **PASS**: Exit code 0 (Vite built in 553ms). |
| **Proxy Smoke** | `node scripts/resilience-proxy.mjs` | **PASS**: Starts on `127.0.0.1:13399` with control on `127.0.0.1:13398`. |

### Tier 1 Resilience Test Details (`src-tauri/tests/resilience.rs`)
- `test_midstream_reset_yields_error_not_clean`: 1MB payload aborted mid-stream -> `ScanStatus::Error`, cache count 0.
- `test_stall_timeout_yields_error_not_clean`: In-flight stall cut off at 500ms -> `ScanStatus::Error`, reason has "timed out", cache count 0.
- `test_malformed_truncated_responses_rejected`: 4 malformed frames (truncated, suffix garbage, non-terminal newline, empty) -> all yield `ScanStatus::Error`, cache count 0.
- `test_daemon_connection_refused_never_cached`: Closed port -> `ScanStatus::Error`, cache count 0.
- `test_cancel_during_stall_returns_cancelled`: Stalling folder scan cancelled at 50ms -> `summary.cancelled == true`, `complete == false`, `clean == 0`, cache count 0.
- `test_engine_signature_change_invalidates_cache_reuse`: Scan 1 under `28175` cached Clean -> bump to `28176` with fault forces live rescan (fails with Error) -> offline engine produces Error, never cached Clean.
- `test_pipelined_extra_response_rejected`: Multi-response injection rejected as Error, cache count 0.
- `test_stale_scan_race_dropped`: Multi-scan lifecycle race condition isolation verified.
- `test_incomplete_version_response_rejected`: Truncated engine identity rejected, cache count 0.
- `test_clamd_stream_limit_error_handled`: Quota limit error frame parsed and reason retained.
- `test_infected_preserves_threat_name`: Threat name retained in `ScanResult.reason`.

---

## 3. Tier 2 Harness & Visible Execution Guide for Qeen

The Tier 2 proxy harness is located at `scripts/resilience-proxy.mjs` (and `docs/qa/evidence-resilience/proxy-harness.mjs`). It is zero-dependency (uses `node:net`), strictly binds to loopback (`127.0.0.1`), and is 100% DEFENDER-SAFE.

### How Qeen Runs Tier 2 Visibly:

1. **Establish Lab ClamAV Tunnel:**
   ```powershell
   ssh -i ~/.ssh/zonendpoint_lab -o ExitOnForwardFailure=yes -N -L 127.0.0.1:13310:127.0.0.1:3310 admzon@100.96.250.245
   ```

2. **Start the Resilience Fault Proxy:**
   ```powershell
   node scripts/resilience-proxy.mjs --listen=13399 --target=13310 --control=13398 --fault=passthrough
   ```

3. **Launch Zonendpoint in Foreground:**
   ```powershell
   $env:ZONENDPOINT_CLAMD = "127.0.0.1:13399"
   # Launch installed package binary or debug binary visibly on desktop
   & "E:\Zonendpoint-tools\artifacts\0de3208\zonendpoint.exe"
   ```

4. **Execute Visible Step-by-Step Scenarios (Pause ~3s each for observation):**
   - **Step 1 (Clean Passthrough):** Scan 3 benign files -> verify `All Good` green button appears.
   - **Step 2 (Fault: Reset Instream):** In another terminal, switch proxy fault mode:
     ```powershell
     # Send to control port 13398
     $client = New-Object System.Net.Sockets.TcpClient("127.0.0.1", 13398)
     $stream = $client.GetStream()
     $writer = New-Object System.IO.StreamWriter($stream)
     $writer.WriteLine("SET FAULT=reset-instream")
     $writer.Flush()
     $client.Close()
     ```
     Click "Scan" -> Observe heading **"Scan incomplete"**, error notice, **NO "All Good" button**.
   - **Step 3 (Fault: Stall & Cancel):**
     Send `SET FAULT=stall-instream`. Click "Scan", then click "Stop".
     Observe transition to **"Stopping…"** then **"Scan cancelled"**, **NO "All Good" button**.
   - **Step 4 (Fault: Corrupt Response):**
     Send `SET FAULT=corrupt-response`. Click "Scan".
     Observe **"Scan incomplete"**, `unrecognized clamd response`, **NO "All Good" button**.
   - **Step 5 (Restore Passthrough):**
     Send `SET FAULT=passthrough`. Click "Scan".
     Observe **"Scan complete"** and **"All Good"** green button restores.
   - **Step 6 (Identity Bump & Offline):**
     Send `SET FAULT=bump-identity`. Close the SSH tunnel. Click "Scan".
     Observe app does **NOT** reuse old cached Clean; reports connection error.

---

## 4. Scope Boundaries: What Stays NOT RUN

1. **No Live EICAR through Windows Defender:** Real EICAR remains intercepted by host Defender (Event 1116/1117, code 225). Resilience tests use benign fixtures, harmless lab signatures, or TCP-level fault injection.
2. **No Remote clamd Daemon Configuration Edits:** `/etc/clamav/clamd.conf` on the Ubuntu lab VM was not touched.
3. **No Network Adapter Disables:** Network interfaces on Windows or Ubuntu were never disabled; faults are injected strictly at the local TCP proxy layer.
4. **No Installer / NSIS Build or Release Merge:** No packaging or merging to `main` was performed.
