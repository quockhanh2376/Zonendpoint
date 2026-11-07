# Handoff: P0-01 Security Fix & Q-3 Findings Resolution — Vic to Qeen

- **Date:** 2026-11-07
- **From:** Victor (Vic) — Lead Developer
- **To:** Qeen — Independent Tester & Security Reviewer
- **Branch:** `vic/ui-simplify` in `E:\Zonendpoint-vic-ui`
- **Target Snapshot / Source SHA:** `0a555169fb6e72db82456e568f385982a8021e39` (`0a55516`)
- **Parent Commits:**
  - `2b82b13`: `docs(qa): import qeen q-3 resilience verification report and handoff for e1a48d3`
  - `0a55516`: `fix(scanner): require bounded EOF drain after clamd response frame (P0-01)`

---

## 1. Exact Artifacts & Checksums

Artifacts have been built and staged in `E:\Zonendpoint-tools\artifacts\0a55516\`:

| Artifact | Path | Size (bytes) | SHA256 | PE Machine Architecture |
|---|---|---|---|:---:|
| **NSIS Installer** | `E:\Zonendpoint-tools\artifacts\0a55516\Zonendpoint_0.1.0_x64-setup.exe` | 3,688,462 | `A0C9E63F3358C92E3EC56022C7DB16287D055CB190BAF4AD4DC45C3D40727252` | `0x014C` (x86 bootstrapper / x64 payload) |
| **Standalone Executable** | `E:\Zonendpoint-tools\artifacts\0a55516\zonendpoint.exe` | 8,057,344 | `AF414BC52E8D06015243CAFDDA5F0712ABB29F5AF7565F54B4A495E948367FD4` | `0x8664` (x64 / AMD64) |

*Note: No installer run was executed by Vic. No merge to main.*

---

## 2. What Changed (Resolution of Qeen Q-3 Findings)

### Finding 1 (P0-01, Security): Race condition in `try_read` after response terminator
- **Problem:** `scanner.rs` previously relied on non-blocking `socket.try_read` right after reading the NUL terminator. If clamd or a proxy delayed a second frame (`stream: Threat FOUND\0`) by even a few milliseconds, `try_read` returned `WouldBlock` (`n == 0`), bypassing detection and falsely marking the file `Clean` / `All Good`, caching it in SQLite.
- **Fix:**
  - Introduced `read_until_eof(&mut socket, EOF_DRAIN_TIMEOUT)` where `EOF_DRAIN_TIMEOUT = Duration::from_secs(1)`.
  - After receiving the terminal NUL byte in `stream_scan`, the socket continues reading until EOF (`read == 0`).
  - Any extra bytes arriving (`read > 0`) produce `ScanStatus::Error` (never `Clean`, zero cache writes).
  - Timeout without EOF produces `ScanStatus::Error` (never `Clean`, zero cache writes).
  - Applied the identical bounded EOF drain rule to `clamd_version_with_timeout` (zVERSION / preflight check).
- **TDD Tests:**
  - Implemented 5 new Tier 1 integration tests in `tests/resilience.rs`:
    1. `test_delayed_second_frame_50ms_rejected`: 50ms delayed second frame produces `ScanStatus::Error`, cache entries = 0.
    2. `test_delayed_second_frame_500ms_rejected`: 500ms delayed second frame produces `ScanStatus::Error`, cache entries = 0.
    3. `test_response_without_eof_never_closed_rejected`: server that never closes socket times out and produces `ScanStatus::Error`, cache entries = 0.
    4. `test_preflight_delayed_extra_frame_rejected`: preflight check with delayed extra bytes fails.
    5. `test_preflight_without_eof_rejected`: preflight check without EOF close fails.

### Finding 2 (Harness / Infrastructure): Dropped early client chunks in resilience proxy
- **Problem:** When detecting `zINSTREAM\0`, `scripts/resilience-proxy.mjs` initiated `connectUpstream` asynchronously; early file data chunks from the client were dropped while the upstream connection was pending.
- **Fix:** Ported Qeen's fix into `scripts/resilience-proxy.mjs`:
  - `clientSocket.pause()` upon starting upstream connection.
  - Early client chunks queued into `pendingChunks`.
  - Upon upstream `'connect'`, buffered `pendingChunks` are flushed to upstream and `clientSocket.resume()` is called.

### Finding 3 (Automation / Documentation): Windows 11 file picker automation
- Documented in `docs/native-ui-tests.md`:
  - `ComboBoxEx32` (AutomationId `1148`) in Windows 11 `IFileOpenDialog` lacks `ValuePattern`.
  - Automated tests must send `WM_SETTEXT` directly to the child `Edit` window and trigger `Open` (AutomationId `1`).

### Finding 4 (Q-3 Report & Handoff Archival)
- Copied Qeen's verification report `docs/qa/resilience-e1a48d3.md` and handoff `docs/handoffs/qeen-to-vic-e1a48d3.md` into the branch and committed under SHA `2b82b13`.

---

## 3. Observed Verification Results

| Test / Gate | Result | Notes |
|---|:---:|---|
| `cargo test --locked` | **PASS** | 46 tests passed (29 lib + 1 main + 16 resilience), 0 failed |
| `cargo fmt --check` | **PASS** | Formatted cleanly |
| `cargo clippy --locked --all-targets -- -D warnings` | **PASS** | 0 warnings |
| `npm.cmd run build` | **PASS** | Vite + TypeScript client build exit 0 |
| Native Mock Test Suite (14 checks) | **PASS** | 14/14 tests passed with temporary profile isolation |
| `npm.cmd run build:windows` | **PASS** | NSIS x64 installer packaged successfully |

---

## 4. What Qeen Must Re-Check

1. **P0-01 Live & Fault Verification:**
   - Verify that delayed second frames (50ms, 500ms) or lingering connections without EOF never produce `Clean` or `All Good` in UI, and never create records in `cache.db`.
   - Verify `zVERSION` / preflight check against proxies sending delayed frames or hanging open.
2. **Resilience Proxy Harness:**
   - Verify `scripts/resilience-proxy.mjs` in Tier 2 visible test runs to ensure no client chunks are lost during upstream connect.
3. **Installer Integrity:**
   - Verify `E:\Zonendpoint-tools\artifacts\0a55516\Zonendpoint_0.1.0_x64-setup.exe` in Qeen's independent worktree.
