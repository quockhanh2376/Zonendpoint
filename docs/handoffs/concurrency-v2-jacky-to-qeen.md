# J-03 handoff: adaptive concurrency v2

## Snapshot

- Code snapshot SHA: `1e0ec28ba6bc5c0732b7d544836a51dddfdb5b51`
- Branch: `jacky/concurrency-v2` (base `22eb28f` = `origin/main`)
- Local commit: `1e0ec28` — `feat(scan): raise concurrency ceiling and recover after backoff (J-03)`
- Push: not performed; Loki owns push. No merge to main.

## Changes

- Automatic concurrency: 8 GiB -> 2, 16 GiB -> 3; 32+ GiB and 8+ logical CPUs -> 6; 32+ GiB with fewer than 8 CPUs -> 4. CPU availability remains a ceiling. Hard cap: 6.
- `max_scan_concurrency` is now a clamped explicit override in range 1–6, so it can raise the automatic selection. Missing field remains compatible with old `config.json`.
- Timeout/busy backoff still drops the current limit by one, floor 1. Each 50 consecutive CLEAN/INFECTED file results recovers one permit up to the configured/selected ceiling. Other errors, SKIPPED and CANCELLED reset the streak and do not count as recovery successes.
- No changes to protocol, clamd, lab, Guard, or cache schema.

## Verification

- `cargo fmt --check`: PASS
- `cargo test`: PASS, 51 library + 1 binary + 18 resilience tests
- `cargo clippy --all-targets -- -D warnings`: PASS
- Capacity matrix includes 8/16/32/64 GiB × 4/8/16 CPUs. Config clamp, legacy JSON, backoff, recovery threshold and ceiling are unit tested.
- Tunnel TCP 127.0.0.1:13310: reachable at check time. Lab clamd telemetry: NOT RUN (Qeen measures later).

## Requested real-folder measurement

Not run. The computer-use control refused access to the running Zonendpoint UI, so no scan was started. No claim is made for wall time, client CPU/RAM, CLEAN/ERROR/SKIPPED counts, pressure/backoff occurrence, or whether 6 is >10% faster than 4. Raw status: `E:\Zonendpoint-tools\artifacts\j03\measurement-status.md`.

Required follow-up on the authorized desktop: use `C:\Program Files`; levels 2, 4, 6; 3 cache-isolated first scans per level in interleaved order plus 1 cache scan; record wall, process CPU, peak working set, CLEAN/ERROR/SKIPPED and timeout/busy/backoff. No EICAR/malware. Qeen captures clamd metrics. Until measured, no evidence supports raising or lowering the default 6 based on performance; if 6 fails to beat 4 by >10%, recommend restoring default hardware cap 4.

## Qeen acceptance requested

1. Independently inspect snapshot `1e0ec28ba6bc5c0732b7d544836a51dddfdb5b51`; rerun fmt, tests and clippy.
2. Verify RAM/CPU matrix and config override behavior, including legacy JSON.
3. Verify backoff and 50-file recovery, including reset for non-pressure errors, and that recovery never exceeds chosen ceiling.
4. Run the documented interleaved real-folder measurement and provide clamd CPU/RSS separately. Do not change clamd configuration.
