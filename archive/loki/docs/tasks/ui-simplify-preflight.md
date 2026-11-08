# Task UI-SIMPLIFY + PREFLIGHT (requested by Zon 2026-11-07 after manual test of 0de3208)

Manual test by Zon on 0de3208 (lab tunnel 127.0.0.1:13310): File/Folder dialogs, Unicode file, folder 3 files → CLEAN/All Good PASS; wrong port → 3× Scan error, "Scan incomplete", no All Good PASS.

## Requested changes
1. **Select target menu = File, Folder only.** Remove "Quick Scan" from the menu. Quick Scan stays as the default main button when no target is selected. **Zon decision:** after a File/Folder scan finishes (any outcome, incl. cancelled), the target auto-clears and the main button returns to "Quick Scan"; results of the finished scan stay visible until the next scan starts.
2. **Result view = summary-first.**
   - All files clean → one line "All Good" only; no per-file list.
   - Threats → list only INFECTED files as alerts (path + signature) for later handling (handling/quarantine = later phase, not v1.0.0).
   - Errors/unknown/skipped/cancelled → "Scan incomplete" + list only those files (safety rule: incomplete never shown as clean; cannot be hidden).
3. **Preflight server check.** Before any scan, a short `zVERSION` check (few-second timeout) against ZONENDPOINT_CLAMD. If unreachable: do not start the scan; show one notice "Cannot connect to ZonEndpoint server" (no per-file error flood; user-facing copy says ZonEndpoint server, never ClamAV); Start button returns to idle. Engine identity from preflight is reused for cache validation. A server drop mid-scan still yields per-file ERROR / Scan incomplete.
4. **Offline + cache.** With the tunnel truly stopped, previously-clean cached files must NOT be shown CLEAN (preflight fails → no scan). Qeen to verify explicitly.

## Constraints
v1.0.0 scope update required in docs/v1.0.0-scope.md (menu, result view, preflight). Typed outcomes unchanged. No auto-quarantine. English UI copy. Use ui-ux-pro-max for the result view layout.

## 5. Server connection status icon (Zon 2026-11-07)
- Small status dot/icon in the header area, always visible.
- **Green** = connected (last check succeeded). Tooltip: "Connected to ZonEndpoint server".
- **Red** = not connected. Tooltip: "Cannot connect to ZonEndpoint server. Check your connection." Clicking it (or its tooltip action) re-checks now.
- Neutral/grey only while the first check is running ("Checking connection…").
- Checks: on app start, before every scan (the preflight), and periodically while idle (proposal: every 30 s; no checks during a running scan beyond the scan itself). Lightweight `zVERSION` with short timeout; never blocks the UI.
- Do not show the server IP/port in the UI. Red status must never coexist with an "All Good" result from a scan started while red.
