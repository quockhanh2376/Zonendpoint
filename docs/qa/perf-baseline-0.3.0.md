# Performance baseline 0.3.0 (J-06)

Date: 2026-11-13. Approved by Zon for J-06 on 2026-11-13.

## Scope and method

This is a repeatable development-only baseline; it makes no performance optimization or product behavior change. The benchmark is `src-tauri/examples/folder_bench.rs`, built with `cargo build --release --example folder_bench`. It invokes the same walker, `scanner::scan_file_with_engine_override`, `ScanLimiter`, verified cache and engine preflight as the application. Quick mode uses `quick::system_root`, `targets_for_root` and `exclusions_for_root`; default concurrency comes from `config::scan_concurrency()` (measured as 2). `--concurrency N` forces a level, and `--cancel-after N` sends the same watch-channel cancellation signal after N seconds.

Each run has a distinct `LOCALAPPDATA` at `E:\Zonendpoint-tools\artifacts\j06\runs\<slot>`. Cold runs use fresh directories. Warm runs have the cold baseline cache copied into their own directories. The benchmark JSON contains wall time (walker only, excluding process startup and clamd preflight), process CPU time, peak working set of the benchmark process only, typed outcome counts, p50/p95/p99/max per-file call duration, and the ten slowest file names, sizes and durations. Per-file duration includes limiter wait and the scanner operation. Cancellation latency is the interval from sending the cancellation signal until `scan_roots` returns. UTC start/end process times are in each slot's `run.meta`.

No Defender/Sophos configuration was changed, no EICAR/malware was used, and no lab SSH or GitHub access was used. Engine access was through loopback tunnel `127.0.0.1:13310`; read-only `zVERSION` returned `ClamAV 1.5.4/28181/Thu Nov 12 06:27:14 2026`.

## Machine and source

- Windows 10 IoT Enterprise LTSC 2024; 4 logical CPUs; 8 GB RAM (machine details supplied for this run; WMI hardware inventory was blocked).
- Branch: `jacky/perf-baseline`; starting base `f3e959d9ff2d573a321b7499fcbff2e548cb822a`.
- Source commit: `ba0e418f00aa7f3e6b65f49e0efe256eea9026a4` (benchmark source; commit on `jacky/perf-baseline`).
- Benchmark executable SHA-256: `1B2B6B909BE8E91140DC3CAB8382FCCDE46043973974C82F4FB8510A6A896DFE`.
- Application user's `%LOCALAPPDATA%\ZonEndpoint\cache.db` SHA-256 before and after: `B61B17A969F103BBFE173F70839BBED0BE7E14F0A0562525569352BC7829AB1C` (identical).

## Results

Pending completion of the full 15-run matrix. JSON and UTC run intervals are retained in each run slot beneath `E:\Zonendpoint-tools\artifacts\j06\runs`.

### Quick scan, automatic concurrency 2

| Run | Cache | Wall ms | CPU s | Peak process MB | Files | CLEAN | ERROR | SKIPPED | p50 ms | p95 ms | p99 ms | max ms |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| cold-1 | empty | 123653 | 4.17 | 6.9 | 589 | 589 | 0 | 0 | 6359 | 8721 | 12901 | 13812 |
| cold-2 | empty | 7128 | 4.28 | 7.1 | 589 | 589 | 0 | 0 | 362 | 517 | 753 | 805 |
| cold-3 | empty | 8771 | 4.55 | 7.2 | 589 | 589 | 0 | 0 | 387 | 1050 | 1578 | 1698 |
| warm-1 | seeded cache | 1440 | 2.45 | 6.9 | 589 | 589 | 0 | 0 | 74 | 123 | 182 | 189 |
| warm-2 | seeded cache | 1995 | 2.73 | 6.9 | 589 | 589 | 0 | 0 | 99 | 170 | 262 | 283 |

### Cancellation: `C:\Program Files`

| Delay | Run | Wall ms | CLEAN | INFECTED | ERROR | SKIPPED | CANCELLED | Cancel-to-return ms |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 10 s | 1 | 10002 | 316 | 0 | 0 | 3 | 66 | 0 |
| 10 s | 2 | 10013 | 427 | 0 | 0 | 3 | 65 | 0 |
| 10 s | 3 | 10003 | 445 | 0 | 0 | 3 | 65 | 0 |
| 60 s | 1 | 60001 | 2410 | 0 | 0 | 3 | 65 | 0 |
| 60 s | 2 | 60010 | 5024 | 0 | 1 | 3 | 66 | 0 |
| 60 s | 3 | 60002 | 5149 | 0 | 0 | 3 | 65 | 0 |

`INFECTED=0` is an observation for these `C:\Program Files` cancellation trials, not a detection claim.

### `C:\Program Files` cache-cold, automatic concurrency 2

| Run | UTC start | UTC end | Wall ms | Files | CLEAN | ERROR | SKIPPED | p50 ms | p95 ms | max ms |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 2026-11-13 04:18:43Z | 2026-11-13 04:25:12Z | 388106 | 12886 | 12874 | 4 | 8 | 578 | 3592 | 32711 |
| 2 | 2026-11-13 04:25:12Z | 2026-11-13 04:27:09Z | 117813 | 12886 | 12875 | 3 | 8 | 271 | 516 | 2517 |
| 3 | 2026-11-13 04:27:09Z | 2026-11-13 04:29:05Z | 115575 | 12886 | 12874 | 4 | 8 | 270 | 460 | 2571 |
| 4 | 2026-11-13 04:29:05Z | 2026-11-13 04:30:57Z | 111618 | 12886 | 12875 | 3 | 8 | 261 | 443 | 2652 |
| 5 | 2026-11-13 04:30:57Z | 2026-11-13 04:32:52Z | 115634 | 12886 | 12875 | 3 | 8 | 271 | 479 | 2623 |

## Slow-run review

Quick cold run 1 is an outlier: 123.653 s versus a median of 8.771 s (>50% deviation). Its p95/max were 8721/13812 ms, versus 517/805 ms in cold-2 and 1050/1698 ms in cold-3. Its ten slowest calls were driver paths in `C:\Windows\System32\drivers` at 12310–13812 ms; the normal-run top ten were also driver paths at 708–805 ms (cold-2) and 1474–1698 ms (cold-3).

Program Files run 1 is also an outlier: 388.106 s versus a median of 115.634 s (>50% deviation). Its p95/max were 3592/32711 ms, versus p95 443–516 ms and max 2517–2652 ms in normal runs 2–5. Run 1's ten slowest calls were 32481–32711 ms (`wabimp.dll`, `WATPCSP.dll`, `wab.exe`, `mpvis.dll.mui`, `SenseNdr.exe`, `wmplayer.exe.mui`, `WMPMediaSharing.dll.mui`, `wabmig.exe`, `setup_wm.exe.mui`, `wmlaunch.exe.mui`). In normal run 2, the ten slowest calls were 2225–2517 ms (`SenseNdr.exe`, `SenseDlpService.dll`, `MsSense.dll`, `SenseDlpProcessor.exe`, `SenseTVM.exe`, `SenseIdentity.exe`, `wabimp.dll`, `ThirdPartyNotice`, `SenseGPParser.exe`, `wab.exe`). Full paths, sizes and timings for both runs and the other samples are in each slot's `result.json`. The repeated ~32-second per-call tail in outlier 1 is an observation only; no clamd journal/timer evidence was collected to establish why it happened.

No Program Files sample returned INFECTED. Counts per run were 12,886 results, 12,874–12,875 CLEAN, 3–4 ERROR and 8 SKIPPED. This records the observed scan only; it is not a claim about all files or a test of infected-result preservation.

## Limits and unverified work

- Peak RAM covers only the benchmark process; app plus WebView2 process-tree RAM is **NOT RUN** (Qeen must measure on the installed application).
- No clamd host CPU/RSS or lab journal/timer telemetry was collected; no SSH was used. Tunnel-side attribution of slow runs is pending Loki's lab review.
- Cache isolation is per run; warm caches were copied between isolated run slots. Filesystem and daemon state may still vary during measurement.
- Qeen acceptance is **NOT RUN**.
- Initial harness attempt results were invalid because dropping its cancellation sender caused the shared scanner to treat the closed channel as cancellation. That sender lifetime was corrected before the final build and all reported runs above were repeated with 0 unsolicited CANCELLED results in non-cancellation runs.

## Quality gates

- `cargo fmt --check`: PASS.
- `cargo test`: PASS, 70 tests (51 library, 1 binary, 18 resilience).
- `cargo clippy --all-targets -- -D warnings`: PASS.
- `cargo build --release --example folder_bench`: PASS.

## Reproduce

From `src-tauri` in a Windows x64 checkout:

```powershell
cargo build --release --example folder_bench
$env:LOCALAPPDATA = 'E:\Zonendpoint-tools\artifacts\j06\runs\<slot>'
$env:ZONENDPOINT_CLAMD = '127.0.0.1:13310'
cargo run --release --example folder_bench -- --quick
cargo run --release --example folder_bench -- 'C:\Program Files' --cancel-after 10
```

For a cold run, use a new slot. For a warm run, copy a compatible `ZonEndpoint\cache.db` into a distinct slot first. Omit `--concurrency` to measure the application's automatic setting; supply `--concurrency N` only when explicitly comparing forced levels.
