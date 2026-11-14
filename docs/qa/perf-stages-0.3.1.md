# Zonendpoint 0.3.1 — Đo lường hiệu năng từng chặng (V-61)

- Ngày: 2026-11-14 (UTC+7)
- Branch: `vic/031-perf`
- Base: `origin/vic/031-client` @ `e35f4d7` (tip V-57)
- Môi trường đo: Windows 10 IoT Enterprise LTSC 2024, 4 vCPU, 8 GB RAM.
- Engine: clamd qua loopback tunnel `127.0.0.1:13310` (`ClamAV 1.5.4/28183`, `signatures: 8616901`, `MaxThreads: 12`, `IdleTimeout: 30`, `StreamMaxLength: 25M`).
- Isolated `LOCALAPPDATA` tại `E:\Zonendpoint-tools\artifacts\v61-perf\runs\<slot>`.
- Các chặng được đo riêng biệt:
  1. `wait_limiter`: Chờ cấp permit từ ScanLimiter.
  2. `open_and_hash`: Mở file, băm SHA-256 ban đầu và kiểm tra metadata snapshot trước/sau.
  3. `cache_lookup`: Tra cứu SQLite cache (`cache::get`) và xác minh metadata snapshot khi hit.
  4. `tcp_connect`: Kết nối TCP tới clamd (`TcpStream::connect`).
  5. `send_instream`: Gửi lệnh `zINSTREAM\0`, truyền các chunk dữ liệu có gắn header frame, và frame kết thúc 0.
  6. `clamd_wait`: Chờ clamd xử lý và đọc frame kết quả NUL-terminated (`read_nul_frame` + `read_until_eof` + `parse_response`).
  7. `cache_write`: Ghi kết quả verdict vào SQLite cache (`cache::put`).

---

## 1. Số đo trước khi tối ưu (Baseline — Trước)

### 1.1. Tổng quan các lượt chạy

| Lượt chạy | Target | Cache | Wall (ms) | CPU (s) | Scanned | Clean | Errors | Skipped | Cache entries | p50 (ms) | p95 (ms) | Max (ms) |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Quick Cold | System Root | Empty (cold) | 9,181 | 4.38 | 589 | 589 | 0 | 0 | 589 | 435 | 889 | 1,262 |
| Quick Warm | System Root | Seeded (warm) | 1,443 | 1.73 | 589 | 589 | 0 | 0 | 589 | 72 | 120 | 170 |
| Program Files Cold | `C:\Program Files` | Empty (cold) | 920,445 | 73.73 | 12,886 | 12,865 | 13 | 8 | 11,036 | 685 | 7,710 | 184,160 |

### 1.2. Chi tiết từng chặng: Quick Cold (trước)

| Chặng | Tổng (ms) | p50 (ms) | p95 (ms) | Max (ms) |
|---|---:|---:|---:|---:|
| `wait_limiter` | 268,682.77 | 405.71 | 852.62 | 1,215.59 |
| `open_and_hash` | 1,218.41 | 1.05 | 5.76 | 87.63 |
| `cache_lookup` | 2,064.91 | 1.30 | 14.74 | 30.03 |
| `tcp_connect` | 1,280.02 | 0.34 | 17.54 | 25.05 |
| `send_instream` | 4,539.58 | 0.90 | 35.25 | 342.50 |
| `clamd_wait` | 6,476.58 | 6.51 | 30.14 | 229.38 |
| `cache_write` | 2,460.25 | 2.73 | 14.63 | 42.96 |

### 1.3. Chi tiết từng chặng: Quick Warm (trước)

| Chặng | Tổng (ms) | p50 (ms) | p95 (ms) | Max (ms) |
|---|---:|---:|---:|---:|
| `wait_limiter` | 42,006.99 | 66.93 | 113.32 | 152.77 |
| `open_and_hash` | 1,712.06 | 1.01 | 13.66 | 74.09 |
| `cache_lookup` | 1,162.76 | 1.02 | 9.08 | 33.15 |
| `tcp_connect` | 0.00 | 0.00 | 0.00 | 0.00 |
| `send_instream` | 0.00 | 0.00 | 0.00 | 0.00 |
| `clamd_wait` | 0.00 | 0.00 | 0.00 | 0.00 |
| `cache_write` | 0.00 | 0.00 | 0.00 | 0.00 |

### 1.4. Chi tiết từng chặng: Program Files Cold (trước)

| Chặng | Tổng (ms) | p50 (ms) | p95 (ms) | Max (ms) |
|---|---:|---:|---:|---:|
| `wait_limiter` | 27,574,776.70 | 641.98 | 7,078.84 | 184,120.52 |
| `open_and_hash` | 113,526.01 | 7.84 | 17.97 | 570.65 |
| `cache_lookup` | 22,437.06 | 1.07 | 3.40 | 1,007.89 |
| `tcp_connect` | 20,960.74 | 0.34 | 12.41 | 2,029.29 |
| `send_instream` | 132,718.62 | 0.24 | 1.59 | 23,641.92 |
| `clamd_wait` | 1,444,957.07 | 22.00 | 300.86 | 33,249.24 |
| `cache_write` | 41,720.99 | 2.63 | 7.48 | 1,476.23 |

*(Phần 2: Số đo sau khi tối ưu và So sánh trước/sau sẽ được cập nhật sau khi hoàn thành các mục P-2, P-3, P-6, P-7, S-10)*
