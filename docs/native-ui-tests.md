# Native UI tests (Windows)

Run from this restored implementation checkout. Requires an interactive Windows desktop, Node/npm, Rust MSVC, Visual C++/Windows SDK, WebView2, tauri-driver 2.1.0 and Edge WebDriver matching installed Edge. Setup follows https://v2.tauri.app/develop/tests/webdriver/manual-setup/ .

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;C:\Program Files\nodejs;" + $env:Path
$env:EDGE_DRIVER = 'E:\Zonendpoint-tools\edgedriver\msedgedriver.exe'
npm ci
npm run test:native
```

One command builds frontend and debug native app with the explicit native-ui-tests feature, starts the loopback mock and native drivers, drives the app and writes artifacts/report.json plus screenshots in a new %TEMP%\zonendpoint-native-* folder. Paths are printed on completion. Return code is nonzero on failure. Driver ports 4445/4446 must be available. The mock is explicitly test-only, never ClamAV detection evidence.

Real-engine benign smoke (open the SSH tunnel from docs/clamav-lab.md in the coordinator checkout first):

```powershell
$env:ZONENDPOINT_CLAMD = '127.0.0.1:13310'
npm run test:native -- --real
```

The real mode checks native UI/core file and folder scans on unique benign fixtures. It does not test EICAR or the full network failure matrix against the real daemon. --skip-build is only for rerunning an already built matching debug test binary. CARGO_TARGET_DIR, EDGE_DRIVER and TAURI_DRIVER optionally override executable locations.

Isolation: each run gets unique fixture, cache/LOCALAPPDATA and WebView profile directories. WebView userDataFolder is passed through tauri:options, preserving it across relaunch inside that run only. Cleanup ends the owned WebDriver session, driver and mock sockets. Artifacts/fixtures remain for inspection; no existing user data is deleted.

Target injection is limited to feature native-ui-tests AND debug_assertions. Release builds with that feature deliberately fail compilation. Normal builds use the actual native picker; no production IPC command or frontend bypass was added. Test builds with ZONENDPOINT_TEST_FIXTURE select benign.txt or its parent in the normal select_scan_path command. This verifies real IPC/scanning, NOT the Windows picker itself.

Coverage limits: real picker automation, native taskbar icon inspection, full product-string inventory, contrast in every pseudo-state, daemon timeout duration, EICAR, signature-update invalidation and installer remain NOT RUN in this suite. Rust protocol/unit coverage complements these tests. Do not interpret passing mock tests as real detection acceptance.

## Windows 11 Native File Dialog Automation Notes

The standard Windows 11 file picker dialog (`IFileOpenDialog`) uses a `ComboBoxEx32` control (AutomationId `1148`) for the file path input box. This control does not support UIAutomation's `ValuePattern`. Test automation targeting Windows 11 must send `WM_SETTEXT` directly to the child `Edit` window within `ComboBoxEx32`, then invoke the `Open` button (AutomationId `1`).

