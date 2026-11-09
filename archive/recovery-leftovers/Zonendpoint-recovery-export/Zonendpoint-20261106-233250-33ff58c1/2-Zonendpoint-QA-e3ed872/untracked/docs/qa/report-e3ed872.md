# QA report — e3ed872

## Snapshot and environment

- Code snapshot: `e3ed872ddfd4a95d9bb28a3cc6da53a656e0b0ae` (verified against the `e3ed872` handoff entry).
- Handoff snapshot: `24876647ebf1f621b3c0a567e60efd40500d6154`.
- QA checkout: detached worktree `E:\Zonendpoint-QA-e3ed872`, clean at the code SHA before testing. Vic's `E:\Zonendpoint-vic` worktree was not changed.
- Windows native Tauri app, Rust/Cargo 1.98, Node 26.13, npm 11.20. QA Vite/Tauri used isolated port 1421.

## Results

| Area | Status | Evidence |
|---|---|---|
| Snapshot identity | PASS | Full code and handoff SHAs verified; QA ran the code commit itself. |
| Theme logos | PASS | Native Light and Dark screenshots matched the supplied palettes. Both source PNGs are RGBA with zero-alpha corners; their original dimensions are preserved (789×684 and 777×667) and `object-fit: contain` showed no crop or stretch. |
| Window icon and theme restore | PASS | Windows `WM_GETICON / ICON_SMALL` handle changed when switching themes. Closed and relaunched the QA app after selecting Dark; Dark logo and toggle state restored, and the reopened window received an icon. |
| Template UI icons | PASS | Accessibility tree showed ZonEndpoint branding. No `vite.svg`, `tauri.svg`, Vite/React starter text, or Tauri logo references were found in the implemented UI/config. The unused starter SVG files remain on disk but are not displayed. |
| File/folder pickers and scan control | PASS (mock protocol only) | Native file and folder pickers both selected the benign fixture. The UI exposed one scan button: Start scan → Stop → disabled Stopping… → Start scan after cancellation; no separate Cancel scan control appeared. A subsequent folder scan completed and returned to Start scan. |
| Cancellation and late response | PASS (mock protocol only) | The first mock stream response was delayed. Stop produced `Scan cancelled`, `clean 0`, `cancelled 1`; the client disconnected before the delayed response. The next folder scan used a separate mock connection and completed without an old result overwriting it. |
| Frontend/Rust checks | PASS | `npm ci` (44 packages, 0 reported vulnerabilities); `npm run build`; `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`; `cargo check --manifest-path src-tauri/Cargo.toml`; `cargo test --manifest-path src-tauri/Cargo.toml` (19 passed, 0 failed). Vite output contained both `dist/brand` logos. |
| Windows installer/shortcut | NOT RUN | No release bundle existed in the QA checkout; no installer or shortcut was built. This is not a PASS. |
| Real ClamAV integration | NOT RUN | Lifecycle testing used only a clearly identified local protocol mock on `127.0.0.1:3311`. Its `stream: OK` caused the UI to show one CLEAN result; this is mock output, not ClamAV detection. The mock was stopped and the port is offline. |

## Findings

No actionable issue found in the tested icon, theme persistence, picker, and unified scan-control scope. Installer/shortcut appearance and real-engine scanning remain unverified as noted above.
