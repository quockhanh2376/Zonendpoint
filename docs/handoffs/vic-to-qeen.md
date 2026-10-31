# Vic to Qeen handoff

## Snapshot

- Branch: `vic/mvp-first-scan`
- Exact code snapshot to review: `cef0d9cb0ed35171e93f45680288e152d2df5178`
- This follow-up handoff is a documentation-only commit; retest the code SHA above, not the handoff commit.
- Parent MVP baseline: `2db42a8124f19384d154588fac41d7bab791bb97`
- Scope: manual file/folder scan MVP plus this UI theme/contrast update. No push, merge, deployment, or scan logic change in the theme commit.

## Findings addressed in prior MVP snapshot

- Cache hits require a fresh SHA-256 of current bytes, a second hash validation, current ClamAV `VERSION` (including signature version), matching policy version, and a 12-hour TTL. Only CLEAN/INFECTED verdicts are cacheable. Old metadata-only cache rows are dropped transactionally. Offline/error cannot reuse CLEAN.
- Clamd requests use bounded `zINSTREAM` frames and NUL-terminated replies. Replies are size-limited, timeout/cancel aware, and parsed as exact terminal protocol messages; malformed, empty, truncated, oversized, or daemon ERROR replies cannot become CLEAN.
- Folder traversal feeds a queue of 32, retains at most 1,000 result details, exposes aggregate counts and a truncation flag, and preserves per-path traversal/scan errors. Cancellation is explicit and summaries stay incomplete when errors/skips/cancellation occur.
- Scanner, CLI, and `cache_stats` use `%LOCALAPPDATA%\Zonendpoint\cache.db` on Windows.
- The UI uses native file/folder pickers and one-use selection tokens, shows progress/cancel and six typed outcomes, and states that real-time protection is not enabled. The reported Rust `String`/`&str` compile issue was fixed.

## UI theme changes in this snapshot

- Added a top-right sun/moon button with English `title` and `aria-label`; first launch defaults to Light and localStorage persists the selected theme.
- Added Light/Dark CSS tokens for background, text, panels, controls, progress, statuses, and notices. Start scan has explicit default/hover/pressed/disabled colors; no broad selector override or dependency added.
- Updated `docs/v1.0.0-scope.md`. English-only remains in scope; language selection stays deferred.
- No scanning or ClamAV code changed in `cef0d9c`.

## Checks on this UI snapshot

Environment: Windows 11 Enterprise x64, Rust/Cargo 1.98, Node 26.13, npm 11.20.

- `npm run build`: PASS (`tsc` + Vite build).
- `git diff --check`: PASS (line-ending conversion warnings only).
- Native Tauri was run from `E:\Zonendpoint-vic` using `npm run tauri dev -- --config C:\Users\ASW_AN~1\AppData\Local\Temp\zonendpoint-tauri-11d888bac7564cacace349b57cd69026.json` to isolate Vite on port 1421; the ordinary configured port 1420 was occupied by another app and was left untouched.
- Native Light and Dark rendering observed. Dark was selected, app closed normally and relaunched; Dark restored. Light was selected again and remains active in the open native window.
- Native disabled Start scan was observed in both themes and the label remained legible. Native keyboard focus ring observed on a control. Native mouse-hover of enabled Start scan and pressed state were NOT RUN: CUA offers click/keyboard but no safe hover-only input, and clicking Start scan would begin a scan.
- Contrast ratios calculated from CSS color tokens for Start scan text/background: Light default 6.39:1, hover 8.20:1, pressed 10.73:1, disabled 6.11:1; Dark default 7.88:1, hover 9.30:1, pressed 6.58:1, disabled 7.52:1. Verify visually in the retest as well.
- Rust fmt/check/clippy/tests were not rerun on this UI-only commit; the previous MVP handoff recorded their results for its distinct snapshot. No ClamAV daemon was available; real-engine detection is NOT RUN. No malware sample used.
- ClamAV connection status in the current native app: offline; no mock daemon and no scan was run.

## Qeen retest acceptance

1. Confirm exact checkout SHA `cef0d9cb0ed35171e93f45680288e152d2df5178` and no local modifications are mixed into the test snapshot.
2. Native Tauri: verify Light/Dark rendering, toggle accessibility label/tooltip, keyboard focus, persisted preference across restart, and Start scan default, hover, pressed, and disabled contrast. A selected safe text fixture may be used to enable Start scan; do not start a scan against an offline/untrusted engine.
3. Check English-only UI strings and that protection is not presented as enabled.
4. Scanner/cache/folder regressions belong to the earlier MVP acceptance and should be rerun against this code SHA if Qeen needs to certify the combined snapshot; do not infer their status from this UI-only diff. Use a local mock daemon for protocol/UI checks. Real ClamAV lab detection is NOT RUN until an isolated trusted lab is available.

## Migration and limitations

- No migration or dependency added by the theme update; preference is `zonendpoint-theme` in WebView localStorage.
- Cache schema v1 intentionally drops unverifiable legacy metadata-only entries; Windows cache location is `%LOCALAPPDATA%\Zonendpoint\cache.db`.
- File limit is 100,000,000 bytes; folder queue is 32; retained result detail cap is 1,000; shared active-scan limit is 2. Over-limit and incomplete work must not be treated as CLEAN.
- Manual scanning only. Guard/realtime protection, quarantine, sync/feed integration, and production deployment are deferred. Watcher is not enabled by this MVP UI.
- Native picker-to-scan/progress/cancel with daemon remains to be verified by Qeen. Native enabled-button hover/pressed interaction remains specifically unverified here.

## Host security status observed (read-only)

`Get-MpComputerStatus` returned `AntivirusEnabled=True`, `RealTimeProtectionEnabled=False`, `BehaviorMonitorEnabled=False`, `IoavProtectionEnabled=False`. No Defender/Sophos policy or machine security setting was changed.
