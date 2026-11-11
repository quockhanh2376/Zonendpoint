# J-01 handoff: adaptive client concurrency

## Snapshot

- Code snapshot SHA: `bab14a0f936670cb58de302fc13352cb01f4f76a`
- Branch: `jacky/adaptive-concurrency`
- Base: `origin/main` at `0979f5fab4ea59912c5ad59a21f01c605f93eaab`
- Local commit: `bab14a0` — `feat(scan): adapt folder concurrency to client capacity (J-01)`
- Push status: **DONE** (Loki đã push lên `origin/jacky/adaptive-concurrency`).
- No changes to main, clamd configuration, protocol, or lab server.

## Changes for review

- Selects a scan concurrency from `std::thread::available_parallelism()` and physical RAM: 8 GiB -> 2; 16 GiB -> 3; 32/64 GiB -> 4. CPU count is also a ceiling and the client cap is 4.
- `max_scan_concurrency` in `%LOCALAPPDATA%\ZonEndpoint\config.json` can lower the automatically selected cap; invalid values are clamped to 1–4. Existing JSON without this field remains valid.
- Folder scans now run bounded file workers (maximum queued/in-flight worker tasks 32) using the shared scanner limiter; file outcomes and progress are still recorded by the folder task. Cancellation drains active results first so an already-reported INFECTED result is retained; remaining queued entries are CANCELLED.
- Timeout/busy server errors permanently reduce the current process limiter by one, down to one worker. Other file errors and stream size policy errors do not trigger backoff. A new process recalculates from machine/config settings.

## Verification

- `cargo fmt --check`: PASS
- `cargo test`: PASS, 49 library + 1 binary + 18 resilience tests
- `cargo clippy --all-targets -- -D warnings`: PASS
- Capacity matrix unit test: 8/16/32/64 GiB × 4/8/16 CPUs, plus low CPU/RAM bounds: PASS
- Current machine: Windows API reports 8 GiB RAM and `NUMBER_OF_PROCESSORS=4`; selected default is 2.
- Local tunnel preflight succeeded at `127.0.0.1:13310`. No protocol change was made.

## Same-folder measurement

Folder: `C:\Program Files` (12,886 files). Baseline is the exact base snapshot with its original serial directory scan and semaphore 2. Updated is the J-01 folder worker implementation on this same client/tunnel. Each snapshot used a separate cache directory. Process CPU is cumulative CPU time across threads; memory is peak working set.

| Snapshot | First scan | Cache scan | Scan summary |
|---|---:|---:|---|
| Baseline | 488.6 s wall; 248.7 s CPU; 8.3 MiB peak | 239.0 s wall; 207.6 s CPU; 8.0 MiB peak | 12,875 CLEAN; 3 ERROR; 8 SKIPPED |
| J-01 | 613.6 s wall; 259.0 s CPU; 11.2 MiB peak | 107.0 s wall; 209.9 s CPU; 9.1 MiB peak | 12,875 CLEAN; 3 ERROR; 8 SKIPPED |

Each row is one run per condition. First-scan time was 25.6% slower in the J-01 sample; cache scan was 55.3% faster (2.24x). The miss-run client CPU was about half a CPU core on average, with low memory use; the evidence points to clamd/tunnel wait rather than client CPU/RAM saturation on this 8 GiB host. The cache-hit path uses more client CPU under parallel hashing and completes sooner. Lab clamd CPU/RSS was **NOT RUN**: the SSH identity file was inaccessible to this session and host-key verification could not be completed. Therefore the server-side cause of the first-scan regression is unconfirmed; do not claim server saturation as measured fact.

The folder scan was incomplete in both runs due to the same 3 ERROR and 8 SKIPPED results. It did not produce an All Good verdict. No EICAR or malware fixture was used. Benchmark raw process outputs and cache DBs are under `E:\Zonendpoint-tools\artifacts\j01\`.

### QA Q-26 Independent Verification (PASS)

- **QA Status**: PASS (Qeen verified 2026-11-11, report at `E:\Zonendpoint-tools\artifacts\j01-verify\qeen-verify.md`).
- **Số đo xác minh Q-26** (3-run interleaved average on `C:\Program Files`, 12,886 files):
  - **Quét đầu (first scan / cache miss)**: 415 s → 194 s (415.39 s → 193.76 s, nhanh hơn 2.14x / giảm 53.4% thời gian; khắc phục kết quả đo ban đầu bị nhiễu do clamd nạp chữ ký MalwareBazaar).
  - **Quét qua cache (cache hit)**: 209 s → 109 s (208.53 s → 108.52 s, nhanh hơn 1.92x / giảm 48.0% thời gian).
- Lab clamd CPU max 5.3%, RSS 2302.4 MiB (ổn định tuyệt đối, không nghẽn server).
- Toàn bộ tiêu chí nghiệm thu Qeen đều PASS. Zon đã duyệt merge cả hai (2026-11-11).

## Qeen acceptance requested

1. Independently inspect the code snapshot and rerun `cargo test`, fmt, and clippy.
2. Verify the 8/16/32/64 GiB × 4/8/16 CPU selection matrix and config cap behavior.
3. Verify backoff on timeout/busy responses, floor at one, and no backoff for stream limits; verify cancellation preserves in-flight INFECTED evidence.
4. If read-only SSH access becomes available, capture clamd CPU/RSS before and during a repeated same-folder scan. Do not modify the server or its configuration.

## Limitations

- Only one miss and one cache measurement per snapshot were taken; load variation is possible.
- Real ClamAV was reached through the existing loopback tunnel, but SSH telemetry from the lab host was not available.
- The 3 file errors and 8 skipped entries make the folder summary incomplete; they were preserved and not treated as CLEAN.
