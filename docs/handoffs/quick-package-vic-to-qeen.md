# Quick Scan package — Vic to Qeen

Date:2026-11-07. **Build/static checks PASS; package runtime acceptance pending Qeen.** No installer/native GUI was run by Vic. This handoff supersedes preliminary 4243b8f logs and the packaged84c4dfa candidate. Preserve the older artifacts as evidence; they are not the final acceptance package.

## Exact source and artifacts

- Checkout:E:\Zonendpoint-restored\Zonendpoint-vic, branch `vic/mvp-first-scan`.
- Source SHA:`0de3208a6ce209b52f885c75053e97e09597c81c`, tracked product source clean before and after building; the sole untracked file was this handoff document. No product-source patch. This is Quick scan/x64 baseline c7f5fd9 plus the approved **Quick Scan** capitalization in default/menu, backend message and native assertions, plus the CLI exit-status fix described below; manual **Scan** is unchanged. Documentation-only changes precede the label commit.
- Stable installer:`E:\Zonendpoint-tools\artifacts\0de3208\Zonendpoint_0.1.0_x64-setup.exe` (3,676,007 bytes).
- Installer SHA256:`EE35537DBF8ABCBA2038F68B63A4396568F5D00AE757B2D1EBBC6F9CACD7AECD`.
- Stable application:`E:\Zonendpoint-tools\artifacts\0de3208\zonendpoint.exe` (8,031,232 bytes).
- Application SHA256:`86E31039692EAAD7EDB7E0580297D982B0B7EA3A3638011062B99EC2AEFEAC3C`.
- Manifest, logs, source/lock identities and source guards are alongside the stable artifact in `manifest.json`, `log-hashes.json`, `release-hook-source-guards.txt` and named logs below. Artifact storage is machine-local, not a repository/public release upload.

## Observed checks

All commands ran from the implementation checkout with Node/Rust executable paths prepended to PATH; no CARGO_TARGET_DIR, RUSTFLAGS, Tauri feature or ZONENDPOINT test/root/target overrides were set.

| Command | Result | Evidence filename under artifact directory |
| --- | --- | --- |
| `npm.cmd ci` | PASS, exit0; audit0 vulnerabilities | `npm-ci.log` |
| `npm.cmd run build` | PASS, exit0 | `frontend-build.log` |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | PASS,26 tests/0 failures (25 library+1 CLI); no retry needed | `cargo-test.log` |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | PASS, exit0 | `cargo-fmt.log` |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | PASS, exit0 | `cargo-clippy.log` |
| `npm.cmd run build:windows` | PASS, exit0; release optimized build2m36s; one NSIS bundle | `build-windows.log` |

The explicit Windows script is `tauri build --target x86_64-pc-windows-msvc --bundles nsis`. It used the implementation's `src-tauri/target/x86_64-pc-windows-msvc/release` tree; no concurrent build shared this cache.

Node26.13.0, npm11.20.0, rustc1.99.1 (58cf8f9c3,2026-10-13), cargo1.99.1 (5f94df478,2026-08-27), stable-x86_64-pc-windows-msvc; NSIS v3.13. Full rustc/OS/tool hashes are in artifact evidence. `npm ci` warned about deprecated whatwg-encoding and approval coverage for edgedriver/geckodriver install scripts; install/build exited0. Vic did not change dependency policy or install additional drivers.

Lockfiles unchanged before/after:

- `package-lock.json`:SHA256 `C2A4A6525A2881B3083B7320EF8DB7B8EC9A7B249C4D897A04B209F959FE6EAA`.
- `src-tauri/Cargo.lock`:SHA256 `4ACFFD0B3A440B834DC1916B5C0C2CEA6419DD5782BB78679D99D68F5C0FCB7E`.

## Static package identity / release hook boundary

The application PE is AMD64 (`0x8664`); the NSIS bootstrapper PE is I386 (`0x014C`), packaging the explicit x64 payload. Both report file/product version0.1.0, ProductName/FileDescription Zonendpoint and Authenticode `NotSigned`. App CompanyName is zonendpoint; installer CompanyName is blank. This is an unsigned internal candidate, not a signed1.0 release.

No `native-ui-tests` feature is passed by the package command and the feature has no default activation. `src-tauri/src/lib.rs` rejects the feature in release with `compile_error!`; fixture selection is gated on both the feature and debug_assertions. `src-tauri/src/quick.rs` gates `ZONENDPOINT_TEST_SYSTEM_ROOT` identically and resolves production System32 using native `GetSystemDirectoryW`. The source evidence is static; Qeen must still observe that release ignores bogus fixture/root overrides. No release-with-test-feature negative compile was run by Vic.

## CLI defect and regression

Qeen/root found that the84c4dfa release printed typed Infected/Skipped but exited0 because `main.rs` treated only Error/Unknown as failures. The source-only commit0de3208 changes the private CLI status mapping:Clean→0; Infected/Unknown/Error/Skipped/Cancelled→1. The typed stdout format is unchanged.

The new table regression uses all six real enum outcomes and literal expected exit codes. A behavior-preserving extraction of the old mapping was tested first:the regression failed on Infected (actual0/expected1), recorded in `red-cli-test.log`. After the minimal mapping fix, the full Rust suite passed25 library tests+1 CLI regression, then fmt/clippy/frontend passed. These checks ran before committing the same source; `npm ci` and the explicit x64 NSIS command ran again on the committed SHA. Root/Qeen owns independent process-level and installed-release acceptance; this document makes no external-process/runtime PASS claim from Vic.
## Qeen acceptance still required

Use this exact source SHA and installer hash in an independent checkout and isolated profile/fixtures. Verify actual File/Folder dialogs (Unicode/Cancel), default/menu **Quick Scan**, manual **Scan**, system-target scope, Stop/Stopping/rescan, retained typed errors and no false All Good, bounded details/window, theme/icon persistence, real lab benign/positive/offline behavior, sandbox current-user install/shortcut/repair/uninstall/data preservation. Runtime fixture/mock evidence must remain separate from the installed release observations.

The prior strict payload helper in `docs/qa/release-nsis-01e7449/qeen-check-nsis-payload.py` accepts only the known three-byte UNK→NSS bundle marker change with the Tauri prefix; inspect/adapt paths for this artifact and do not weaken provenance checks.

NOT RUN by Vic:all native/runtime/installer actions, real-engine scans, signing, different-version upgrade, multi-user deployment, resource benchmark and full network-resilience matrix. No installer was executed, no push/merge/publication occurred, the sole product source change was the committed CLI fix, and no user installation/profile/cache, AV policy or VM was changed. Root coordinates final status/daily/PR updates after Qeen's exact-artifact results; this handoff remains uncommitted for that coordinated documentation commit.
