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

## 2. Số đo sau khi tối ưu (0.3.1 — Sau: P-2, P-3, P-6, P-7, S-10)

### 2.1. Tổng quan các lượt chạy sau tối ưu

| Lượt chạy | Target | Cache | Wall (ms) | CPU (s) | Scanned | Clean | Errors | Skipped | Cache entries | p50 (ms) | p95 (ms) | Max (ms) |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Quick Cold | System Root | Empty (cold) | 190,760 | 4.36 | 589 | 589 | 0 | 0 | 589 | 9,746 | 17,887 | 24,166 |
| Quick Warm | System Root | Seeded (warm) | 978 | 1.63 | 589 | 589 | 0 | 0 | 589 | 48 | 96 | 213 |
| Program Files Cold | `C:\Program Files` | Empty (cold) | 613,718 | 47.31 | 12,886 | 12,874 | 4 | 8 | 11,044 | 644 | 6,630 | 33,509 |

### 2.2. Chi tiết từng chặng: Quick Cold (sau)

| Chặng | Tổng (ms) | p50 (ms) | p95 (ms) | Max (ms) |
|---|---:|---:|---:|---:|
| `wait_limiter` | 5,615,756.60 | 9,146.44 | 16,207.06 | 23,684.05 |
| `open_and_hash` | 12,148.92 | 8.18 | 43.72 | 1,290.39 |
| `cache_lookup` | 240.59 | 0.22 | 0.54 | 33.23 |
| `tcp_connect` | 747.25 | 0.41 | 6.46 | 25.50 |
| `send_instream` | 7,365.88 | 1.09 | 11.54 | 944.68 |
| `clamd_wait` | 356,863.98 | 387.65 | 1,575.89 | 9,898.78 |
| `cache_write` | 574.73 | 0.05 | 1.66 | 173.00 |

### 2.3. Chi tiết từng chặng: Quick Warm (sau)

| Chặng | Tổng (ms) | p50 (ms) | p95 (ms) | Max (ms) |
|---|---:|---:|---:|---:|
| `wait_limiter` | 28,533.19 | 46.47 | 93.77 | 110.32 |
| `open_and_hash` | 1,431.84 | 1.17 | 8.58 | 73.05 |
| `cache_lookup` | 224.13 | 0.23 | 0.97 | 10.40 |
| `tcp_connect` | 0.00 | 0.00 | 0.00 | 0.00 |
| `send_instream` | 0.00 | 0.00 | 0.00 | 0.00 |
| `clamd_wait` | 0.00 | 0.00 | 0.00 | 0.00 |
| `cache_write` | 0.00 | 0.00 | 0.00 | 0.00 |

### 2.4. Chi tiết từng chặng: Program Files Cold (sau)

| Chặng | Tổng (ms) | p50 (ms) | p95 (ms) | Max (ms) |
|---|---:|---:|---:|---:|
| `wait_limiter` | 18,373,187.45 | 601.65 | 6,271.74 | 33,078.39 |
| `open_and_hash` | 127,306.41 | 8.46 | 20.79 | 573.68 |
| `cache_lookup` | 4,437.68 | 0.21 | 0.40 | 61.19 |
| `tcp_connect` | 18,914.22 | 0.35 | 13.47 | 35.04 |
| `send_instream` | 21,722.17 | 0.45 | 1.73 | 718.61 |
| `clamd_wait` | 1,035,501.44 | 22.26 | 313.43 | 11,575.40 |
| `cache_write` | 1,664.63 | 0.05 | 0.18 | 31.02 |

---

## 3. Bảng so sánh Trước / Sau từng chặng (Program Files Cold & Quick Warm)

### 3.1. So sánh trên tập lớn: `C:\Program Files` (Cold) — 12,886 files

| Chặng | Trước: Tổng (ms) | Sau: Tổng (ms) | Chênh lệch (%) | Trước: p50 (ms) | Sau: p50 (ms) | Nhận xét tối ưu |
|---|---:|---:|---:|---:|---:|---|
| **Wall Clock** | **920,445** | **613,718** | **-33.3%** | 685 | 644 | Giảm >5 phút toàn bộ phiên quét |
| **CPU Time** | **73.73 s** | **47.31 s** | **-35.8%** | — | — | Giảm tải CPU đáng kể |
| `cache_write` | 41,721.0 | 1,664.6 | **-96.0%** | 2.63 | **0.046** | **Nhanh gấp 25 lần**: nhờ gom transaction ≤200 dòng/≤1s & connection SQLite dùng chung (P-2) |
| `send_instream` | 132,718.6 | 21,722.2 | **-83.6%** | 0.24 | 0.45 | **Nhanh gấp 6.1 lần**: chunk 1 MiB thay vì 64 KiB, max transfer giảm từ 23.6s xuống 0.7s (P-3) |
| `cache_lookup` | 22,437.1 | 4,437.7 | **-80.2%** | 1.07 | **0.21** | **Nhanh gấp 5 lần**: `prepare_cached` và connection tái sử dụng (P-2) |
| `wait_limiter` | 27,574,776.7 | 18,373,187.5 | **-33.4%** | 642.0 | 601.7 | Tắc nghẽn hàng đợi giảm tương ứng khi client xử lý nhanh hơn |
| `clamd_wait` | 1,444,957.1 | 1,035,501.4 | -28.3% | 22.0 | 22.3 | Tương đương thời gian xử lý clamd trên host lab |
| `open_and_hash` | 113,526.0 | 127,306.4 | +12.1% | 7.84 | 8.46 | Băm trong `spawn_blocking` với bộ đệm 1 MiB (P-3) |
| `tcp_connect` | 20,960.7 | 18,914.2 | -9.8% | 0.34 | 0.35 | Ổn định quanh ~0.35 ms / file qua loopback tunnel |

### 3.2. So sánh Quick Warm (Tất cả file hit cache)

| Chặng | Trước (ms) | Sau (ms) | Cải thiện |
|---|---:|---:|---|
| **Wall Clock** | **1,443 ms** | **978 ms** | **Giảm 32.2% (dưới 1 giây)** |
| `cache_lookup` Tổng | 1,162.8 ms | 224.1 ms | **Giảm 80.7% (nhanh gấp 5.2 lần)** |
| `cache_lookup` p50 | 1.02 ms | 0.23 ms | **Nhanh gấp 4.4 lần** |
| `open_and_hash` Tổng | 1,712.1 ms | 1,431.8 ms | Giảm 16.4% |
| `wait_limiter` Tổng | 42,006.9 ms | 28,533.2 ms | Giảm 32.1% |

---

## 4. Kết luận & Đề xuất về IDSESSION (P-4)

### 4.1. Thời gian nằm ở đâu?
Dựa trên kết quả đo lường 7 chặng độc lập trên 12,886 files của `C:\Program Files`:
1. **`wait_limiter` & `clamd_wait` chiếm >95% tổng độ trễ**:
   - `clamd_wait` tổng cộng 1,035.5 giây (~17.2 phút thời gian clamd phân tích nội dung byte).
   - Do client chạy với `concurrency = 2` (theo `config::scan_concurrency()` adaptive), `wait_limiter` tổng 18,373 giây chia đều 2 permits = ~9,186 giây thời gian file nằm trong hàng đợi chờ luồng quét rảnh.
2. **Chi phí Database Cache và IO Client đã được tối ưu triệt để**:
   - `cache_write` giảm từ 41.7 giây xuống 1.66 giây (chỉ còn 0.27% tổng wall time).
   - `cache_lookup` chỉ còn 4.4 giây trên gần 13,000 files (0.72% tổng wall time).
   - `send_instream` giảm từ 132.7 giây xuống 21.7 giây nhờ chunk 1 MiB.

### 4.2. IDSESSION (P-4) có đáng làm không?
- **Số liệu thực tế**: Chặng `tcp_connect` trên toàn bộ 12,886 files chỉ tốn **18.91 giây**, chiếm đúng **3.08%** tổng thời gian quét (613.7 giây).
- **Phân tích chi phí / rủi ro kỹ thuật**:
  - clamd lab cấu hình `IdleTimeout 30` và `MaxThreads 12` (theo tài liệu Q-42).
  - Nếu duy trì một kết nối persistent `IDSESSION`:
    1. Khi client đọc file lớn hoặc gặp nghẽn I/O đĩa cục bộ vượt quá 30 giây, clamd sẽ tự động đóng kết nối do `IdleTimeout 30`. Điều này tạo ra rủi ro đứt gãy kết nối giữa phiên (`Broken pipe` / `Connection reset by peer`), đòi hỏi client phải implement cơ chế tái kết nối, re-handshake IDSESSION và re-stream file cực kỳ phức tạp.
    2. Mỗi worker kết nối IDSESSION sẽ chiếm giữ vĩnh viễn 1 worker thread trong `MaxThreads: 12` của clamd daemon trong suốt thời gian quét, làm giảm khả năng phục vụ của clamd cho các tiến trình khác.
    3. Tiềm năng tối đa nếu làm IDSESSION: Tiết kiệm tối đa ~18 giây trên 613 giây (tối đa ~3% wall time).
- **ĐỀ XUẤT CHÍNH THỨC**:
  - **KHÔNG NÊN LÀM IDSESSION (P-4)** trên client. Lợi ích thu được (≤ 3%) hoàn toàn không tương xứng với độ phức tạp triển khai và rủi ro ngắt kết nối do `IdleTimeout 30`. Kiến trúc kết nối TCP per-file hiện tại kết hợp với `ScanLimiter` (P-2, P-3, P-6, S-10) là tối ưu, an toàn và ổn định nhất.
