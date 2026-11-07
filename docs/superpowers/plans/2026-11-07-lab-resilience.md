# LAB-RESILIENCE Implementation & Verification Plan

**Goal:** Establish a test-owned failure harness and automated verification suite to prove that **every incomplete scan state is never CLEAN** under network faults, engine disruptions, cancellations, and identity changes.

**Scope:** Planning document only (no product code modification).
**Worktree:** `E:\Zonendpoint-vic-resilience` (branch `vic/lab-resilience-plan` at base `0de3208`).
**Target Component:** Zonendpoint scanner transport (`scanner.rs`), traversal coordinator (`directory.rs`), scan manager (`lib.rs`), and frontend state receiver (`App.tsx`).

---

## 1. Principles & Design Constraints (Ponytail Architecture)

1. **Zero New External Dependencies:**
   - **Rust backend:** Reuse existing `tokio` (`net::TcpListener`, `net::TcpStream`, `time::timeout`, `sync::watch`) already included in `Cargo.toml`.
   - **Node.js native test harness:** Use built-in Node.js `node:net` and `node:child_process` (zero npm dependencies).
2. **Environment & Safety Isolation:**
   - No modifications to corporate VPN, firewall, or Windows Defender / Sophos settings.
   - No modifications to Ubuntu clamd configuration (`/etc/clamav/clamd.conf`).
   - ClamAV remains bound to `127.0.0.1` loopback on Ubuntu; Windows tunnel remains `127.0.0.1:13310`.
   - No public exposure of clamd or the failure harness proxy.
3. **Core Invariant:**
   - Offline, network timeout, connection reset, malformed protocol framing, incomplete transmission, and cancellation must **never yield `CLEAN`** and must **never display the green "All Good" checkmark**.

---

## 2. Test-Owned Failure Harness Architecture

### 2.1 Topology

```
+-------------------------------------------------------------------+
| Host: Windows x64 (Zonendpoint App)                               |
|                                                                   |
| [Zonendpoint App]                                                 |
|    |                                                              |
|    | ZONENDPOINT_CLAMD = 127.0.0.1:<proxy_port> (e.g. 13399)      |
|    v                                                              |
| [Local TCP Fault Proxy] (Test-Owned: Rust test or Node.js harness)|
+----|--------------------------------------------------------------+
     | (Healthy Passthrough OR Fault Injection)
     v
+-------------------------------------------------------------------+
| [SSH Tunnel: 127.0.0.1:13310] (or in-process mock daemon)         |
|    |                                                              |
|    v                                                              |
| [Remote Ubuntu Lab: clamd @ 127.0.0.1:3310 (admzon@100.96.250.245)|
+-------------------------------------------------------------------+
```

### 2.2 Proxy Fault Injection Capabilities

The failure proxy sits between the app and the upstream ClamAV daemon/tunnel. It operates in configurable fault modes:

| Fault Mode | Trigger Stage | Action |
|---|---|---|
| `drop-connect` | TCP Handshake | Refuse connection or accept and immediately close without sending data. |
| `reset-version` | `zVERSION\0` Command | Accept connection, read command, immediately send TCP RST or abruptly close. |
| `stall-version` | `zVERSION\0` Command | Accept connection, read command, sleep indefinitely (> `CONNECT_TIMEOUT` = 5s). |
| `corrupt-version`| `zVERSION\0` Response | Reply with invalid string (e.g. `NOT-CLAMAV\0`, non-UTF8, or truncated prefix). |
| `reset-instream` | `zINSTREAM\0` Chunks | Read command and $N$ chunks of payload, then immediately drop/reset socket. |
| `stall-instream` | `zINSTREAM\0` Chunks | Pause transmission mid-stream until `TOTAL_SCAN_TIMEOUT` (60s in app, or shorter in test). |
| `corrupt-response`| Final Scan Verdict | Forward stream to upstream clamd, intercept reply, mutate to `garbage OK\0`, `stream: OK` (no NUL), or oversized payload. |
| `bump-identity` | `zVERSION\0` Command | Intercept real version (e.g. `ClamAV 1.5.4/28175/...`) and alter daily signature number (e.g. `ClamAV 1.5.4/28176/...`). |
| `passthrough` | All stages | Transparent bidirectional forwarding. |

---

## 3. Test Matrix: Incomplete & Fault States

Every fault condition must produce an explicit, non-CLEAN typed outcome, retain per-file failure evidence, and leave the UI in an incomplete/error state.

| # | Fault Scenario | Staged Boundary | Expected Typed Outcome | Summary State | UI Rendered State | Retained Per-File Error | Cache Policy |
|---|---|---|---|---|---|---|---|
| **1** | **Disconnect / Reset Mid-Stream** | In-flight INSTREAM chunk writing | `ScanStatus::Error` | `complete: false`<br>`errors: >= 1`<br>`clean: 0` | "Scan incomplete"<br>Warning notice<br>Scan button (no "All Good") | `status: ERROR`<br>`reason: "write clamd payload: ..."` | **Never Cached** |
| **2** | **Stall → Timeout** | Handshake or waiting for verdict | `ScanStatus::Error` | `complete: false`<br>`errors: >= 1`<br>`clean: 0` | "Scan incomplete"<br>Notice: "Could not connect to ClamAV..."<br>Scan button | `status: ERROR`<br>`reason: "scan timed out"` or `"clamd connection timed out"` | **Never Cached** |
| **3** | **Truncated / Malformed Response** | Terminal response byte stream | `ScanStatus::Error` | `complete: false`<br>`errors: >= 1`<br>`clean: 0` | "Scan incomplete"<br>Warning: "Do not treat an incomplete scan as clean." | `status: ERROR`<br>`reason: "clamd closed before response terminator"` or `"unrecognized clamd response"` | **Never Cached** |
| **4** | **Daemon Restart** | Connect or between files in batch | `ScanStatus::Error` | `complete: false`<br>`errors: >= 1`<br>`clean: 0` | "Scan incomplete"<br>Notice: "Could not connect to ClamAV..." | `status: ERROR`<br>`reason: "connect to clamd: ..."` | **Never Cached** |
| **5** | **Cancel during Fault → Rescan** | During network stall / delay | 1st: `ScanStatus::Cancelled`<br>2nd: `ScanStatus::Clean` (rescan) | 1st: `cancelled: true`, `complete: false`<br>2nd: `complete: true`, `clean: 1` | 1st: "Scan cancelled"<br>2nd: "Scan complete" / "All Good" | 1st: `status: CANCELLED`<br>2nd: `status: CLEAN` | 1st: Never Cached<br>2nd: Cached under live identity |
| **6** | **Engine / Signature Identity Change** | Between Scan 1 and Scan 2 | Scan 1: `CLEAN` (cached)<br>Scan 2: Live rescan (`CLEAN` or `ERROR`) | Dependent on live engine state | Reflects fresh engine verdict; offline produces ERROR | Old cached verdict ignored on identity mismatch | Cache miss on new identity key; must not reuse old `CLEAN` |
| **7** | **Stale Event Exclusion** | Cancel Scan A, trigger Scan B | Scan A events dropped; Scan B progress displayed | `scan_id` isolated | Only Scan B counters and path visible | Race-condition CLEAN from Scan A converted to Cancelled | N/A |

---

## 4. Test Suite Implementation Structure

### 4.1 Tier 1: Rust In-Process Integration Tests (`src-tauri/tests/resilience.rs`)

**Purpose:** Deterministic, fast, CI/developer regression suite without requiring external SSH tunnels or real VMs.

- **Component:** Standalone Rust integration test binary under `src-tauri/tests/resilience.rs`.
- **Harness:** Helper function `spawn_fault_proxy(target_addr, fault_mode) -> (SocketAddr, JoinHandle<()>)` implemented using `tokio::net::TcpListener`.
- **Test Cases:**
  1. `test_midstream_reset_yields_error_not_clean()`:
     - Streams a 1 MB file through proxy configured with `reset-instream`.
     - Asserts `ScanResult.status == ScanStatus::Error`.
     - Verifies sqlite cache table remains empty for this hash.
  2. `test_stall_timeout_yields_error_not_clean()`:
     - Feeds file through proxy with `stall-instream`. Uses short test timeout (500 ms).
     - Asserts `ScanResult.status == ScanStatus::Error` and reason contains `"timed out"`.
  3. `test_malformed_truncated_responses_rejected()`:
     - Feeds 4 distinct corrupt response frames:
       - Truncated `b"stream: OK"` (no trailing `\0`).
       - Suffix extra tokens `b"stream: OK trailing garbage\0"`.
       - Non-terminal token `b"stream: OK \n FOUND\0"`.
       - Empty reply `b""` before connection close.
     - Asserts all 4 return `ScanStatus::Error`.
  4. `test_daemon_connection_refused_never_cached()`:
     - Directs scanner to closed port `127.0.0.1:13398`.
     - Asserts `ScanStatus::Error`, verifies cache count is 0.
  5. `test_cancel_during_stall_returns_cancelled()`:
     - Triggers scan through stalling proxy, fires cancellation after 50 ms.
     - Asserts outcome is `ScanStatus::Cancelled`, `summary.cancelled == true`.
  6. `test_engine_signature_change_invalidates_cache_reuse()`:
     - Scans file under `ClamAV 1.5.4/28175`, caches `CLEAN`.
     - Changes proxy version to `ClamAV 1.5.4/28176`.
     - Simulates engine offline under new version: asserts scanner fails with `ERROR` instead of returning old cached `CLEAN`.

### 4.2 Tier 2: Qeen Native Lab Verification Suite (`qa/resilience-qeen-suite.mjs`)

**Purpose:** Independent black-box E2E validation against the real packaged desktop binary, real Windows WebView2 UI, and actual ClamAV lab tunnel (`127.0.0.1:13310`).

- **Harness:** Lightweight Node.js script (`docs/qa/evidence-resilience/proxy-harness.mjs`):
  ```javascript
  // Uses built-in node:net to bind 127.0.0.1:13399 -> forward to 127.0.0.1:13310
  // Can be dynamically instructed via IPC or CLI flags: --fault=reset|stall|corrupt|bump
  ```
- **Execution Workflow for Qeen:**
  1. Open SSH tunnel to lab:
     `ssh -i ~/.ssh/zonendpoint_lab -o ExitOnForwardFailure=yes -N -L 127.0.0.1:13310:127.0.0.1:3310 admzon@100.96.250.245`
  2. Start test proxy on `127.0.0.1:13399`.
  3. Launch Zonendpoint binary with `ZONENDPOINT_CLAMD=127.0.0.1:13399`.
  4. Execute staged fault sequences:
     - Trigger folder scan containing benign files. Inject `reset-instream` at file 2.
     - Observe UI: "Scan incomplete", errors counter increments, "All Good" button is **ABSENT**.
     - Inject `stall` at file 3, click "Stop". Observe: state transitions to "Stopping...", outcome records "Cancelled", UI retains file 1 (Clean), file 2 (Error), file 3 (Cancelled).
     - Restore proxy to `passthrough`, click "Scan" again. Rescan completes `All Good`.
     - Bump signature version string in proxy, sever tunnel. Scan again. Verify app displays error notice and does **NOT** report `All Good` from cache.
  5. Save DOM snapshot, process logs, and SQLite cache dump into `docs/qa/evidence-resilience/`.

---

## 5. Scope Boundaries: What Stays NOT RUN

To maintain safety and resource boundaries, the following remain strictly **NOT RUN**:
1. **No Live EICAR through Windows Defender:**
   - Real EICAR on Windows host continues to be intercepted by Defender (Event 1116/1117, code 225). Resilience tests use benign test fixtures and harmless lab signatures.
2. **No Remote clamd Daemon Configuration Edits:**
   - `/etc/clamav/clamd.conf` on Ubuntu remains untouched. Signature updates or daemon restarts on Ubuntu are deferred to `ENGINE-OPS`.
3. **No Network Interface Disables:**
   - Host Ethernet/Wi-Fi adapters are never toggled; network faults are injected entirely at the local TCP socket layer.
4. **No Codec or Binary Packaging in this Phase:**
   - Release signing, MSI packaging, and installer upgrades remain out of scope for this plan.

---

## 6. Acceptance Criteria for Handoff to Qeen

1. `cargo test --test resilience` runs 100% PASS with 0 network dependencies outside loopback.
2. Under every single staged fault (disconnect, stall, corrupt, refused, cancelled), the UI and backend verdict **never report `CLEAN`**.
3. Summary `complete` is `false` whenever any file encounters a fault or cancellation.
4. Per-file errors are preserved in the result list up to the 1,000 file retention bound.
5. Cache lookup requires exact `(sha256, engine_version, policy_version)` match; signature bumping forces live re-validation.
6. Stale events from cancelled scans never corrupt active scan UI state.
