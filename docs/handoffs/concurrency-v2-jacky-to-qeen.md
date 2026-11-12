# J-05 handoff: restore application concurrency ceiling to 4

## Snapshot

- Code snapshot SHA: `bf4f79ef0cb307d43b6a1c46f9930adb3f6ef17c`
- Branch: `jacky/concurrency-v2` (base `22eb28f` = `origin/main`)
- Local code commit: `bf4f79e` - `fix(scan): cap app concurrency at four (J-05)`
- Push: not performed; Loki owns push. No merge to main.
## J-05 decision and changes (2026-11-12)

- Zon approved restoring the application ceiling to 4 after J-04 measured 4 vs 6: median 107.7 s vs 98.9 s, only 8.2% faster at 6, below the 10% threshold.
- Automatic selection is 8 GiB -> 2, 16 GiB -> 3, otherwise -> 4, limited by available CPU and hard-capped at 4.
- max_scan_concurrency is a cap only: effective concurrency is automatic.min(cap.clamp(1, 4)). Legacy JSON values 5, 6 or larger remain deserializable and resolve to at most 4; values 0/1 resolve to 1.
- Retained one-step backoff to floor 1, telemetry, and one-slot recovery after 50 consecutive successful CLEAN/INFECTED results, bounded by the selected concurrency. The J-04 cancellation test fix is retained.
- folder_bench remains a dev-only example and still accepts 1-6 worker slots directly for measurement; that argument bypasses application config. It is not the application's limit.
- Gates on code snapshot bf4f79e: cargo fmt --check PASS; cargo test 70/70 PASS (51 library + 1 binary + 18 resilience); cargo clippy --all-targets -- -D warnings PASS; release example build PASS. No benchmark rerun.

## J-03 changes (historical)

- Automatic concurrency: 8 GiB -> 2, 16 GiB -> 3; 32+ GiB and 8+ logical CPUs -> 6; 32+ GiB with fewer than 8 CPUs -> 4. CPU availability remains a ceiling. Hard cap: 6.
- `max_scan_concurrency` is now a clamped explicit override in range 1–6, so it can raise the automatic selection. Missing field remains compatible with old `config.json`.
- Timeout/busy backoff still drops the current limit by one, floor 1. Each 50 consecutive CLEAN/INFECTED file results recovers one permit up to the configured/selected ceiling. Other errors, SKIPPED and CANCELLED reset the streak and do not count as recovery successes.
- No changes to protocol, clamd, lab, Guard, or cache schema.

## J-03 verification (historical)

- `cargo fmt --check`: PASS
- `cargo test`: PASS, 51 library + 1 binary + 18 resilience tests
- `cargo clippy --all-targets -- -D warnings`: PASS
- Capacity matrix includes 8/16/32/64 GiB × 4/8/16 CPUs. Config clamp, legacy JSON, backoff, recovery threshold and ceiling are unit tested.
- Tunnel TCP 127.0.0.1:13310: reachable at check time. Lab clamd telemetry: NOT RUN (Qeen measures later).

## Requested real-folder measurement

Not run. The computer-use control refused access to the running Zonendpoint UI, so no scan was started. No claim is made for wall time, client CPU/RAM, CLEAN/ERROR/SKIPPED counts, pressure/backoff occurrence, or whether 6 is >10% faster than 4. Raw status: `E:\Zonendpoint-tools\artifacts\j03\measurement-status.md`.

Required follow-up on the authorized desktop: use `C:\Program Files`; levels 2, 4, 6; 3 cache-isolated first scans per level in interleaved order plus 1 cache scan; record wall, process CPU, peak working set, CLEAN/ERROR/SKIPPED and timeout/busy/backoff. No EICAR/malware. Qeen captures clamd metrics. Until measured, no evidence supports raising or lowering the default 6 based on performance; if 6 fails to beat 4 by >10%, recommend restoring default hardware cap 4.

## J-03 Qeen acceptance (historical)

1. Independently inspect snapshot `1e0ec28ba6bc5c0732b7d544836a51dddfdb5b51`; rerun fmt, tests and clippy.
2. Verify RAM/CPU matrix and config override behavior, including legacy JSON.
3. Verify backoff and 50-file recovery, including reset for non-pressure errors, and that recovery never exceeds chosen ceiling.
4. Run the documented interleaved real-folder measurement and provide clamd CPU/RSS separately. Do not change clamd configuration.

## J-04 benchmark runner

Development-only example: `src-tauri/examples/folder_bench.rs`. It calls the same `directory::scan_roots` traversal, ClamAV preflight, engine-pinned `scanner::scan_file_with_engine_override`, `ScanLimiter`, and `cache::default_path` as the application. It is not packaged with the application release and adds no dependencies. The selected concurrency is passed as an argument (1â€“6); config/cache paths follow the process `LOCALAPPDATA`.

Build and run in PowerShell:

```powershell
cargo build --manifest-path src-tauri/Cargo.toml --release --example folder_bench
$slot = 'E:\Zonendpoint-tools\artifacts\j04\runs\2-a'
New-Item -ItemType Directory -Force $slot | Out-Null
$env:LOCALAPPDATA = $slot
& .\src-tauri\target\release\examples\folder_bench.exe 'C:\Program Files' 2
```

Each cold run must use a new empty slot such as `runs\2-a`; its cache run reuses that slot. The runner emits JSON with elapsed wall seconds, process CPU seconds, peak working-set bytes, typed outcome counts, completeness, cache path/entry count, final concurrency, backoffs, and recoveries. Keep cache slots separate and never point `LOCALAPPDATA` at the user's normal directory for cold runs.

## J-04 measurement and flaky test update (2026-11-12)

- Checkout: `E:\Zonendpoint-Jacky\j04`, branch `jacky/concurrency-v2`.
- Merged local `origin/main` at `2464aee167cfb1c9780afc98557d5854d85ab98e`; merge commit `0b0ed14`. The only conflict, `docs/daily.md`, retains both J-03 and V-30M entries.
- Code snapshot SHA: `3efdefd2e701d26498759ca0c7a04634ac4c2728` (local; no push or merge to main).
- Windows 8 GiB / 4 CPU, `C:\Program Files` (12,886 files), tunnel `127.0.0.1:13310`. Cold-run order was interleaved 2/4/6, 4/6/2, 6/2/4; 3 cold runs per level and 1 cache run per level.

| Level | Cold runs (wall seconds) | Process CPU (seconds) | Peak RAM (MiB) | CLEAN / ERROR / SKIPPED | Backoff / recovery |
|---:|---|---|---|---|---|
| 2 | 529.5 (`2-a`), 427.9 (`2-b`), 115.5 (`2-c`) | 65.9 / 64.1 / 64.5 | 7.72 / 7.76 / 7.82 | 12,875/3/8; 12,875/3/8; 12,874/4/8 | 0/0 all |
| 4 | 107.7 (`4-a`), 104.0 (`4-b`), 111.4 (`4-c`) | 50.0 / 51.8 / 50.4 | 8.47 / 8.42 / 8.40 | 12,875/3/8; 12,874/4/8; 12,874/4/8 | 0/0; 1/1; 0/0 |
| 6 | 98.9 (`6-a`), 98.6 (`6-b`), 127.6 (`6-c`) | 48.3 / 46.3 / 52.4 | 8.76 / 8.68 / 8.69 | 12,875/3/8; 12,874/4/8; 12,874/4/8 | 0/0; 1/1; 1/1 |

All folder scans had `scanned=12886`, no INFECTED/UNKNOWN/CANCELLED, and `complete=false` because ERROR/SKIPPED were present. Cache-hit 2/4/6: wall 17.3/10.8/8.9 s, CPU 30.2/24.0/22.4 s, peak RAM 7.53/8.28/8.79 MiB; each had 12,875 CLEAN, 3 ERROR, 8 SKIPPED, and no backoff.

Outlier repeats in new cache slots: level 2 slow runs 529.5/427.9 s repeated at 117.4/112.6 s (`2-fast-repeat`, `2-slow-repeat`); the slow condition did not recur. Level 6 `6-c` at 127.6 s with one backoff repeated at 99.3 s, consistent with `6-a/b`. The ~28 s excess is close to the reported ~25 s reload duration, but lab reload timing/logs were not observed, so no cause is assigned.

**4 vs 6 conclusion:** median level 4 = 107.7 s; representative level-6 runs after repeating the outlier = 98.9 s. Level 6 was about **8.2% faster**, below the >10% threshold. Recommend keeping the client concurrency ceiling at **4**.

- Flaky test expectation now allows one or more already-started INFECTED results and asserts every counted INFECTED remains in `results` with positive evidence. Product cancellation behavior is unchanged. Targeted test loop: 50/50 PASS.
- Gates: fmt PASS; `cargo test` 70/70 PASS (51 library + 1 binary + 18 resilience); clippy `--all-targets -- -D warnings` PASS; release example build PASS.
- SHA-256 of files under `%LOCALAPPDATA%\ZonEndpoint` was identical before/after. No EICAR/malware. Benchmark campaign lasted under 61 minutes; lab timer/reload telemetry and clamd CPU/RSS are NOT RUN; Qeen acceptance is NOT RUN.
- Report and raw evidence: `E:\Zonendpoint-tools\artifacts\j04\jacky-report.md`.

### J-04 Qeen acceptance (historical)

1. Independently inspect code snapshot `3efdefd2e701d26498759ca0c7a04634ac4c2728` and rerun fmt, full Rust tests, and clippy.
2. Review the benchmark runner, cache-isolated run procedure and limiter telemetry; verify incomplete outcomes remain visible for ERROR/SKIPPED.
3. Independently verify the 4-vs-6 recommendation and rerun treatment. Capture clamd CPU/RSS and timer/reload state separately; do not change clamd configuration.

### J-05 Qeen acceptance requested

1. Independently inspect the exact code snapshot bf4f79ef0cb307d43b6a1c46f9930adb3f6ef17c and run fmt, cargo test and clippy.
2. Verify automatic RAM/CPU selection, legacy config caps, backoff/recovery, and the direct 1-6 dev-only benchmark argument.
3. No performance remeasurement is requested for J-05.
