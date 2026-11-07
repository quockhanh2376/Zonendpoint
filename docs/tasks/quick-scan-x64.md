# Quick scan / manual Scan / Windows x64

Requested by Zon2026-11-07. CheKi owns task creation/coordination; user does not need to repeat prompts for Vic/Qeen.

## Product decisions received
- Initial primary button is enabled Quick scan, scanning essential Windows11 system files.
- After native File/Folder selection, primary action reads Scan.
- Existing Stop/Stopping lifecycle, all-CLEAN All Good/check and bounded result-window growth remain.
- Application packages/builds target Windows x86_64 only. This does not whitelist or skip32-bit file contents.
- Existing English/Light-Dark/manual initiation/Guard OFF/no quarantine rules remain.

## Approved scope choice
Proposal A:fixed Windows core executable/library list plus System32/drivers, with a clearly stated scan boundary. Proposal B:whole System32 tree, broader and slower with protected files/errors expected. User asked which scope through one concise choice; Zon approved core files + drivers.

Proposed A file list for review:ntoskrnl.exe,ntdll.dll,kernel32.dll,kernelbase.dll,ci.dll,smss.exe,csrss.exe,wininit.exe,services.exe,lsass.exe,svchost.exe,winlogon.exe,winload.efi plus drivers subtree. Missing/denied/changed files yield explicit non-CLEAN outcomes, not silent skips. These are test targets, not an integrity/bootkit or whole-system safety guarantee.

## Vic task V-QUICK-X64
Status:Implementation complete; Vic Rust25/clippy/native mock15 PASS. Qeen validation in progress.
After scope is settled, read project-status/scope and relevant scanner/directory lifecycle code. Write the implementation plan; add failing regressions first. Resolve native64-bit System32 via Windows API rather than a frontend-provided path or untrusted environment variable. Add a dedicated Quick scan command with fixed approved targets, sharing bounded traversal/progress/cancel/state management with manual scan. Do not weaken native picker tokens for manual paths. Test fixtures must be compile-gated to explicit debug test builds; release hook rejection remains. Enable initial Quick scan, switch to Scan for picked file/folder, and provide a way in existing target menu to return to system target. Preserve positive findings when cancelled. Explicitly build/package x86_64 and reject unsupported Windows build architectures. Keep changes small; no auto-elevation, service/Guard/quarantine, public submission or AV policy changes.

Acceptance:initial Quick scan works with no file selection; native manual picks switch action to Scan; return-to-system selection clears prior results; concurrent scan denied; cancel/retry works; all typed outcomes maintained; zero/partial/error/limit/cancel never All Good. Do not claim full-system protection from quick scan.

## Qeen task Q-QUICK-X64
Status:In progress; use the new committed Quick scan snapshot.
Test exact snapshot in isolated QA checkout/app data. Verify fixed approved target scope and native Windows root resolution; test Quick/manual label transitions, Stop/rescan and cross-mode stale-event handling; verify file/folder picker token restrictions remain. Mock fixtures cover clean/threat/missing/access denied/timeout/malformed/cancel and bounded results. Inspect release fixture-hook exclusion and x64 packaging; run real ClamAV against benign fixtures and authorized Windows targets only after scope is confirmed. Record native observations separately from injected fixtures; preserve Defender/Sophos. Report short PASS/FAIL/BLOCKED plus SHA and actionable findings; no unsupported production-ready claim or automatic main merge.

