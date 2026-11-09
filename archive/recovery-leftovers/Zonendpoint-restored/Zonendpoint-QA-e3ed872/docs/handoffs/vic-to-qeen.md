# Vic to Qeen handoff

## Snapshot

- Branch: `vic/mvp-first-scan`
- Exact code snapshot to review: `48c616733f3ae4bee035a8e6cdc78d2a837e6b97`
- Previous UI theme snapshot: `cef0d9cb0ed35171e93f45680288e152d2df5178`
- This follow-up handoff is a documentation-only commit; retest the code SHA above, not the handoff commit.
- Parent MVP baseline: `2db42a8124f19384d154588fac41d7bab791bb97`
- Scope: manual file/folder scan MVP, persistent Light/Dark themes, and unified scan start/stop lifecycle. No push, merge, or deployment.

## Findings addressed in prior MVP snapshot

- Cache hits require a fresh SHA-256 of current bytes, a second hash validation, current ClamAV `VERSION` (including signature version), matching policy version, and a 12-hour TTL. Only CLEAN/INFECTED verdicts are cacheable. Old metadata-only cache rows are dropped transactionally. Offline/error cannot reuse CLEAN.
- Clamd requests use bounded `zINSTREAM` frames and NUL-terminated replies. Replies are size-limited, timeout/cancel aware, and parsed as exact terminal protocol messages; malformed, empty, truncated, oversized, or daemon ERROR replies cannot become CLEAN.
- Folder traversal feeds a queue of 32, retains at most 1,000 result details, exposes aggregate counts and a truncation flag, and preserves per-path traversal/scan errors. Cancellation is explicit and summaries stay incomplete when errors/skips/cancellation occur.
- Scanner, CLI, and `cache_stats` use `%LOCALAPPDATA%\Zonendpoint\cache.db` on Windows.
- The UI uses native file/folder pickers and time-limited selection tokens (reusable for five minutes in the current snapshot), shows progress/cancel and six typed outcomes, and states that real-time protection is not enabled. The reported Rust `String`/`&str` compile issue was fixed.

## UI theme changes in `cef0d9c`

- Added a top-right sun/moon button with English `title` and `aria-label`; first launch defaults to Light and localStorage persists the selected theme.
- Added Light/Dark CSS tokens for background, text, panels, controls, progress, statuses, and notices. Start scan has explicit default/hover/pressed/disabled colors; no broad selector override or dependency added.
- Updated `docs/v1.0.0-scope.md`. English-only remains in scope; language selection stays deferred.
- No scanning or ClamAV code changed in `cef0d9c`.

## Unified scan control changes in `48c6167`

- Replaced the separate Cancel scan control with one button: Start scan, Stop, Stopping… and disabled while cancellation is in flight.
- Rust `ScanManager` now permits only one active scan globally. It publishes `scanning`, `cancelling`, and `idle` events and retains the active slot until the scan worker finishes; a second start cannot overlap a stopping scan.
- The UI registers lifecycle/progress listeners before enabling Start, guards synchronous double-clicks with refs, filters late events/results by scan ID, and only clears the active UI request after the backend call returns.
- Accepted cancellation wins a last-moment single-file CLEAN/ERROR race: the returned single-file result is changed to CANCELLED and summary counts are adjusted. Folder partial results remain individually classified while the overall summary is cancelled/incomplete.
- Native picker selection remains reusable for five minutes, enabling Start again on the same target; expired tokens are removed.
- `Stopping…` is rendered through at least one frame and held briefly while the request settles, then Start scan returns after the backend has confirmed task completion.

## Checks on lifecycle snapshot `48c6167`

Environment: Windows 11 Enterprise x64, Rust/Cargo 1.98, Node 26.13, npm 11.20.

- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`: PASS.
- `cargo check --manifest-path src-tauri/Cargo.toml`: PASS.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`: PASS.
- `cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture`: PASS, 19 passed, 0 failed. New regressions cover backend single-active lifecycle, cancellation never returning a lone CLEAN, and reusable selected targets.
- `npm run build`: PASS (`tsc` + Vite build).
- `git diff --check`: PASS (line-ending conversion warnings only).
- Native Tauri ran from `E:\Zonendpoint-vic` using `npm run tauri dev -- --config C:\Users\ASW_AN~1\AppData\Local\Temp\zonendpoint-tauri-11d888bac7564cacace349b57cd69026.json` to isolate Vite on port 1421; configured port 1420 belonged to another app and was left untouched.
- Native AX/UI showed only Select file, Select folder, and one scan button. With a benign text fixture and a clearly identified loopback mock on `127.0.0.1:3310`: Start → Stop → disabled Stopping… → Scan cancelled with `clean 0`, `cancelled 1` → Start again on the same target → mock-only Scan complete with `clean 1`. A further native stop confirmed the Stopping… frame and then Start after cancellation.
- A rapid-response mock returned `stream: OK` once, producing the mock-only `clean 1` result. A later delayed mock accepted three INSTREAM requests (71 bytes each); all three were cancelled before its delayed reply. These were lifecycle/protocol checks only, not ClamAV detection. The mock process was stopped after testing and port 3310 is offline. No malware sample used.
- The live native window remains open at `http://localhost:1421/` in idle state with no target selected; the mock is stopped. Real ClamAV integration is NOT RUN.

## Qeen retest acceptance

1. Confirm exact checkout SHA `48c616733f3ae4bee035a8e6cdc78d2a837e6b97` and no local modifications are mixed into the test snapshot.
2. Native Tauri: independently repeat Start → Stop → disabled Stopping… → Cancelled → Start again; verify completion and error return to Start; double-click Start and check no overlapping worker; ensure late progress/results from the old scan cannot overwrite the new scan.
3. Verify the five-minute target reuse/expiry, English-only UI, and that protection is not presented as enabled.
4. Re-run scanner/cache/folder regressions against this exact combined code SHA if certifying the full MVP. The local mock can verify lifecycle only; it is not real-engine evidence. Real ClamAV lab detection remains NOT RUN until a trusted isolated lab is available.

## Migration and limitations

- No migration or dependency added by the theme update; preference is `zonendpoint-theme` in WebView localStorage.
- Cache schema v1 intentionally drops unverifiable legacy metadata-only entries; Windows cache location is `%LOCALAPPDATA%\Zonendpoint\cache.db`.
- File limit is 100,000,000 bytes; folder queue is 32; retained result detail cap is 1,000; shared active-scan limit is 2. Over-limit and incomplete work must not be treated as CLEAN.
- Manual scanning only. Guard/realtime protection, quarantine, sync/feed integration, and production deployment are deferred. Watcher is not enabled by this MVP UI.
- Native mock lifecycle was exercised, but real ClamAV integration and high-load resource measurements remain NOT RUN. Native enabled-button hover/pressed visual verification remains the prior UI-theme handoff limitation.

## Host security status observed (read-only)

`Get-MpComputerStatus` returned `AntivirusEnabled=True`, `RealTimeProtectionEnabled=False`, `BehaviorMonitorEnabled=False`, `IoavProtectionEnabled=False`. No Defender/Sophos policy or machine security setting was changed.
