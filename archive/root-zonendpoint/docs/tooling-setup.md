# Local tooling setup — 2026-11-07

- Coordinator: CheKi; lead developer: Voctor; independent QA: Qeen.
- Source checkout: E:\Zonendpoint, main at 2db42a8. Existing user changes preserved.
- Historical implementation/QA worktrees referenced in project-status.md do not exist on this machine. Restore their exact snapshots before continuing MVP development or claiming previous QA applies locally.
- Installed Node.js 26.13.0, Rust stable 1.99.1 MSVC, rustfmt and clippy. Edge WebView2 157.0.4341.42 already present.
- npm ci PASS; npm run build PASS; npm audit reported zero vulnerabilities.
- Superpowers 7.0.1 installed via Codex curated marketplace; Ponytail 5.3.3 installed via DietrichGebert/ponytail marketplace. Start a fresh chat to load skills; Ponytail lifecycle hook trust is not verified. Review /hooks in Codex if prompted.
- Tool source checkouts: E:\Zonendpoint-tools\superpowers and E:\Zonendpoint-tools\ponytail.
- Visual C++ Build Tools installation and Rust dependency download are being verified separately.
- Real ClamAV lab, native app tests and installer tests NOT RUN. Need Ubuntu VM address/version and intended access before server setup. No public feed or gateway/licensing tooling installed outside current scope.

## Final verification
- Visual Studio 2022 Build Tools with VCTools and Windows SDK installed successfully.
- cargo fetch --locked PASS. cargo check reaches project code but FAILS with E0308 at src-tauri/src/lib.rs:24 (String supplied to scan_file expecting &str). This is a baseline code issue; no production code changed.
- Full build log: E:\Zonendpoint-tools\cargo-check.log. Native UI automation suite is not present in this baseline; driver/framework selection should follow restoration of the actual MVP snapshot, rather than modifying this old checkout.


## Ubuntu lab access
- Target supplied by Zon: admzon@100.96.250.245 (new Ubuntu VM).
- SSH reachable; first host key recorded via accept-new. Non-interactive authentication failed (publickey,password). OS version and sudo permission not yet verified; no server changes made.
- Next: user establishes SSH key authentication or an interactive login locally. Do not store passwords in project files/chat.


## UI UX Pro Max — 2026-11-07
Installed from nextlevelbuilder/ui-ux-pro-max-skill (.claude/skills/ui-ux-pro-max) into C:\Users\admzon\.codex\skills\ui-ux-pro-max using skill-installer. Source checkout HEAD b7d5353. Bundled Python 3.12 runtime ran search.py successfully against UX contrast guidelines. No UI code changed. Host discovery is expected on next turn/new chat; not claimed active in this existing skill catalog.

