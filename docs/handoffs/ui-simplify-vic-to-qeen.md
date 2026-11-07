# UI Simplification & ClamD Preflight — Vic to Qeen

Date: 2026-11-07. **Build, unit tests, and native mock suite PASS; independent Qeen verification pending.**
No installer was executed by Vic. No merge to main.

## Exact source and artifacts

- Checkout: `E:\Zonendpoint-vic-ui`, branch `vic/ui-simplify`.
- Source code commit SHA: `8c687b9b4c9ded3cc762f55a62e117ef43bd88e6` (`8c687b9`).
- Base commit: `e1a48d3` (LAB-RESILIENCE on `vic/lab-resilience-plan`).
- Windows NSIS installer artifact: `E:\Zonendpoint-tools\artifacts\8c687b9\Zonendpoint_0.1.0_x64-setup.exe`
  - Size: 3,687,215 bytes
  - SHA256: `ABE6ACFF39C1F881375E289AD07F56184D91FD3433D1ED55832FF75848641CB0`
  - PE architecture: `0x014C` (x86 / i386 NSIS bootstrapper packaging x64 payload)
- Windows standalone executable artifact: `E:\Zonendpoint-tools\artifacts\8c687b9\zonendpoint.exe`
  - Size: 8,054,272 bytes
  - SHA256: `55AD3B9CF860CCA37A64B89A5C94554BF43D578974D6D1D3BD9C864E10F23CB1`
  - PE architecture: `0x8664` (x64 / AMD64)

## Implemented features & Zon specifications

1. **Scope update in `docs/v1.0.0-scope.md`**:
   - Updated before implementation with menu simplification, summary-first result view, auto-return to Quick Scan, preflight check, and header server status indicator.
2. **Target selection menu**:
   - "Select target" menu contains "File" and "Folder" options only. "Quick Scan" is removed from the dropdown menu.
   - "Quick Scan" is the default main action button when no target is explicitly selected.
   - Selecting File/Folder switches the main button to "Scan".
3. **Auto-return to Quick Scan**:
   - After any File or Folder scan finishes (any outcome: clean, threats, incomplete, error, cancelled), `selectedTarget` auto-clears to `null` and the primary action button returns to "Quick Scan".
   - The finished scan's results stay visible in the result view until the next scan starts.
4. **Summary-first result view**:
   - All files clean: single "All Good" line (`.all-good-line`), no per-file list rendered.
   - Threats detected: lists only INFECTED rows (file path + threat name / signature); clean rows omitted.
   - Incomplete / Error / Cancelled: "Scan incomplete" (or "Scan cancelled") heading, accompanied by mandatory warning ("Do not treat an incomplete scan as clean."), listing only non-clean rows (ERROR, UNKNOWN, SKIPPED, CANCELLED).
   - Styled using `ui-ux-pro-max` guidelines for high-contrast Light and Dark mode rendering.
5. **Server preflight verification**:
   - Short `zVERSION` check (3-second timeout) executed against `ZONENDPOINT_CLAMD` before any scan begins.
   - Validates ClamAV banner and signature segment. Unreachable or invalid banner returns notice `"Cannot connect to ZonEndpoint server"`, scan does not start, no CLEAN outcome emitted, no cache reuse.
   - Validated engine/signature identity is reused across all files for cache validation throughout that scan.
   - All user-facing strings never expose "ClamAV", IP, or port numbers.
6. **Server connection status indicator**:
   - Header status dot always visible with accessible 44×44px click target.
   - Green = connected ("Connected to ZonEndpoint server").
   - Red = not connected ("Cannot connect to ZonEndpoint server. Check your connection."). Clicking the red dot triggers an immediate re-check.
   - Grey = checking ("Checking connection…").
   - Checks on app start, before every scan (as part of preflight), and every 30 seconds while idle. Never blocks the UI.
   - Red status cannot coexist with an "All Good" verdict produced by a scan started while red.

## Observed verification results

| Verification Check | Result | Details |
|---|---|---|
| `cargo test --locked` | PASS | 41 tests passed (29 lib + 1 main + 11 resilience), 0 failed |
| `cargo fmt --check` | PASS | Codebase clean and formatted |
| `cargo clippy -- -D warnings` | PASS | 0 warnings |
| `npm.cmd run build` | PASS | TypeScript & Vite client build exit 0 |
| Native mock test suite (14 checks) | PASS | 14/14 checks pass including `menu items`, `auto-return`, `single All Good line`, `threat-only list`, `incomplete list`, `status icon green→red→green` |
| `npm.cmd run build:windows` | PASS | x64 NSIS bundle created in `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/` |

## Qeen verification scope

Independent verification in separate worktree:
- Preflight validation with lab tunnel stopped: verify notice "Cannot connect to ZonEndpoint server", verify offline previously-clean cached files are NOT shown CLEAN.
- Server status icon transitions: green when tunnel open, red when tunnel stopped + click re-check, green when tunnel reopened.
- Menu options: verify File and Folder only.
- Auto-return: verify button returns to "Quick Scan" after scan finishes.
- Result view: verify single "All Good" line on all-clean, verify threats-only list on threat detection, verify incomplete list on error/cancellation.
- Do not run installer in unisolated environments; preserve user configuration.
