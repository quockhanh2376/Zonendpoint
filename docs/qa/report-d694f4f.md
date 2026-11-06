# Verification — d694f4f

Code: d694f4f547c46755dbf4fcf41a7195b0b83fd99c. Tested source subsequently committed unchanged. CheKi development verification; independent Qeen runtime acceptance pending.

- npm run test:native: PASS, 9 checks. Build frontend/debug native included. Log E:\Zonendpoint-tools\viewport-green.log; JSON C:\Users\admzon\AppData\Local\Temp\zonendpoint-native-m8rDlo\artifacts\report.json.
- npm run test:native -- --skip-build --real: PASS, 4 checks, ZONENDPOINT_CLAMD=127.0.0.1:13310. Benign real ClamAV only. Log E:\Zonendpoint-tools\approved-ui-real.log; JSON/screenshots C:\Users\admzon\AppData\Local\Temp\zonendpoint-native-Cqohfc\artifacts.
- Viewport regression: new bounding-rectangle assertion failed on old offscreen UI (viewport-red.log), then passed after feedback moved into panel and final heading ordered before verbose warnings.
- Folder assertion now changes fixture bytes before scanning and verifies expected path, processed count and CLEAN verdict; avoids immediate cache reuse from preceding file test.
- Static reviewer independently identified both issues and re-reviewed their corrections. No remaining important findings in that review scope.
- Final native Light screenshot inspected: outcome heading visible at 800x240, orange slogan right-aligned; detailed results scroll below.

NOT RUN: actual native picker, real EICAR, real timeout/failure/cancel matrix, installer, runtime window/taskbar icon verification for current snapshot, release feature guard execution. Prior Rust tests/clippy results remain historical evidence for unchanged backend.
