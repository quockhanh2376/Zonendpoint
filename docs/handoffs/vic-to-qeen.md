# Vic to Qeen handoff

## Snapshot

- Branch: `vic/mvp-first-scan`
- Code snapshot: `cf1cc8e601dd90b6876c7951adc068a3d95489a8`
- Parent/Qeen baseline: `2db42a8124f19384d154588fac41d7bab791bb97`
- Scope: manual file/folder scan MVP, ClamAV protocol, content-verifying cache, bounded folder traversal, native Tauri picker/UI and typed results. No push, merge, or deployment.

The code is committed separately from this handoff. This handoff is a documentation-only follow-up commit; retest the code SHA above.

## Findings addressed

- Cache hits now require a fresh SHA-256 of current bytes, a second hash validation, current ClamAV `VERSION` (including signature version), matching policy version, and a 12-hour TTL. Only CLEAN/INFECTED verdicts are cacheable. Old metadata-only cache rows are dropped transactionally. Offline/error cannot reuse CLEAN.
- Clamd requests use bounded `zINSTREAM` frames and NUL-terminated replies. Replies are size-limited, timeout/cancel aware, and parsed as exact terminal protocol messages; malformed, empty, truncated, oversized, or daemon ERROR replies cannot become CLEAN.
- Folder traversal feeds a queue of 32, retains at most 1,000 result details, exposes aggregate counts and a truncation flag, and preserves per-path traversal/scan errors. Cancellation is explicit and summaries stay incomplete when errors/skips/cancellation occur.
- Scanner, CLI, and `cache_stats` use `%LOCALAPPDATA%\Zonendpoint\cache.db` on Windows.
- The UI uses native file/folder pickers and one-use selection tokens, shows progress/cancel and all six typed outcomes, and states that real-time protection is not enabled. Development CSP permits Vite's injected React refresh script only in `devCsp`.
- The reported Rust `String`/`&str` compile issue is fixed.

## Checks run

Environment: Windows 11 Enterprise x64, Rust/Cargo 1.98, Node 26.13, npm 11.20.

Commands run from the worktree root:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture
npm run build
git diff --check
```

Results: fmt/check/clippy/frontend build passed; Rust tests passed (16 passed, 0 failed). Regression coverage includes same-size/same-mtime content replacement, stale/offline cache, malformed and fragmented protocol replies, strict response limits, mock-daemon ERROR, framing, cancellation, per-file error retention, Unicode path, result cap, and transactional cache migration.

Native Windows Tauri dev app launched and rendered after the CSP fix; the native file picker opened. The native picker-to-scan/progress/cancel flow was not completed in UI automation. No `clamd`/`clamdscan` executable was available; a real-engine integration run is NOT RUN. No malware sample was used. Native package/release build and resource measurement are NOT RUN.

## Retest acceptance

- Re-run all commands above against exactly `cf1cc8e601dd90b6876c7951adc068a3d95489a8`.
- Confirm native picker → selected item → scan → progress/cancel → final summary through Tauri IPC with a local mock daemon, then a trusted isolated ClamAV lab if available.
- Exercise cache mutation/signature/policy/TTL/DB migration, malformed/fragmented daemon replies, folder traversal errors/limits/cancellation, and verify no non-CLEAN outcome is rendered as CLEAN.
- Verify Scanner and `cache_stats` share the same database path.

## Migration and limitations

- Cache schema v1 intentionally drops unverifiable legacy metadata-only entries; cache location is `%LOCALAPPDATA%\Zonendpoint\cache.db` on Windows.
- File limit is 100,000,000 bytes; folder queue is 32; retained result detail cap is 1,000; shared active-scan limit is 2. Over-limit and incomplete work must not be treated as CLEAN.
- This is manual scanning only. Guard/realtime protection, quarantine, sync/feed integration, and production deployment are deferred. Watcher is not enabled by this MVP UI.
- Real ClamAV detection and native end-to-end scan remain unverified until Qeen runs the lab/UI checks.

## Host security status observed (read-only)

`Get-MpComputerStatus` returned `AntivirusEnabled=True`, `RealTimeProtectionEnabled=False`, `BehaviorMonitorEnabled=False`, `IoavProtectionEnabled=False`. No Defender/Sophos policy or machine security setting was changed.
