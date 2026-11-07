# Qeen Quick scan verification — 2026-11-07

Snapshot: `c7f5fd98a9f1f81a47fae7571b9be3d0e135f884`, isolated checkout `E:\Zonendpoint-Qeen-quick`. Product code was not edited during QA.

## Observed checks

- Rust:25 PASS, including exact14-target manifest/native Windows API root, multiple roots with missing-file error, root junction exclusion, cancellation retaining a positive finding and bounded1000 retained details.
- Native mock:15 PASS through WebDriver → native WebView → Rust IPC → scanner. Covers default Quick scan,14 clean fixtures, missing core file without All Good, threat preservation, Stop/mode transition, manual Scan/menu return, theme persistence, folder fixture,51-file threat visibility, error/malformed/offline and rescan.
- Vic frontend build/clippy passed. Qeen independently rebuilt debug/native-test binary from the snapshot. Native fixture root/target overrides are explicit debug-feature-only; release+test-feature is compile-rejected. Windows non-x86_64 compilation is rejected and `npm run build:windows` explicitly targets x86_64. Unsupported-architecture cross-compilation was NOT RUN.
- Real ClamAV native fixture suite:6 PASS, including Quick14 fixtures, manual Scan/menu return, folder fixture and theme/native shell. Evidence:`C:\Users\admzon\AppData\Local\Temp\zonendpoint-native-sctybW\artifacts\report.json`; log:`E:\Zonendpoint-tools\qeen-quick-real.log`.
- Actual native Windows root, with both test overrides absent:589 outcomes,588 CLEAN,0 threats,1 ERROR. All13 core results present and all retained paths inside native System32. Protected `drivers\DriverData` enumeration returned Access denied/os error5. UI correctly showed Scan incomplete, no green All Good. This is an incomplete system scan, not a whole-scope CLEAN verdict. Final rerun used content-verified cache warmed by the first full scan, with live real-engine identity validation. First full run reached terminal state in approximately5 minutes but QA collection failed on WebdriverIO array handling; fixed collector and repeated successfully. No product change was needed for that collector error.

Actual-system evidence:`C:\Users\admzon\AppData\Local\Temp\zonendpoint-windows-quick-ztPX6I\report.json`, `result.png`; final log:`E:\Zonendpoint-tools\qeen-windows-quick-verified.log`. Isolated test data retained. Protected-directory traversal without elevated privileges remains incomplete by design; no elevation/permission or AV-policy changes made.

Mock evidence: `C:\Users\admzon\AppData\Local\Temp\zonendpoint-native-kKIBc8\artifacts\report.json`. Rust/native logs: `E:\Zonendpoint-tools\qeen-quick-rust.log`, `qeen-quick-native.log`. Reproduce: `npm.cmd ci`, `cargo test --locked --manifest-path src-tauri/Cargo.toml`, `npm.cmd run test:native`; set private `ZONENDPOINT_CLAMD` then `npm.cmd run test:native -- --real` for real fixture mode. Desktop session, matching Edge WebDriver and tauri-driver required as documented in native-ui-tests.md.

## Limits

An initial Vic bounds-test run transiently observed916/1002 outcomes; cause is unproven. Diagnostic assertion added; subsequent full Vic run25, three targeted repeats and independent Qeen25 passed. This is a recorded intermittent test observation, not a claimed production fix.

Actual Windows picker was verified on earlier8e8204a; it was not re-driven on this new snapshot. New native suite uses test-only target injection for manual file/folder cases. No new installer has yet been built/tested for this feature; the01e7449 installer remains older code. No main merge, elevation, AV exclusion or public upload.
