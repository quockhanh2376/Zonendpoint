# Handoff: Task V-61 (Zonendpoint 0.3.1 Client Performance Optimization) — Vic to Qeen

- **Date**: 2026-11-14
- **Author**: Victor (Vic)
- **Branch**: `vic/031-perf`
- **Base Commit**: `e35f4d7` (`origin/vic/031-client`, tip V-57, không phải `main`)
- **Version**: `0.3.1`
- **Artifacts Directory**: `E:\Zonendpoint-tools\artifacts\v61-release\`
- **Installer**: `Zonendpoint_0.3.1_x64-setup.exe` (3,667,853 bytes)
- **Installer SHA256**: `01793d12cede23228b4eeb3530b224ebdcd3264c22df261e3fd1b07d9af73fe7`
- **Worktree**: `E:\Zonendpoint-worktrees\v61-perf` (xóa sau khi hoàn tất handoff và push)

---

## 1. Mục tiêu và Phạm vi (Task V-61)

Thực hiện tối ưu hóa hiệu năng phía client theo tài liệu kiến trúc `E:\Zonendpoint\optimisse.md` do anh Zon giao ngày 2026-11-14:
- Triển khai đầy đủ các hạng mục: **Đo từng chặng (P-4 measurement)**, **P-2**, **P-3**, **P-6**, **P-7**, **S-10**.
- Đánh giá thực nghiệm hiệu năng từng chặng (trước vs sau) trên `Quick Scan` (cold + warm) và `C:\Program Files` (cold, 12,886 files) qua loopback tunnel `127.0.0.1:13310`.
- Phân tích vị trí tiêu tốn thời gian quét và đưa ra đề xuất chính thức về **IDSESSION (P-4)**.
- Các cam kết tuân thủ nghiêm ngặt:
  - Không đụng `scripts/feeds/`, `scripts/windows/`, `.github/`.
  - Không merge vào `main`.
  - Sử dụng `LOCALAPPDATA` tạm hoàn toàn độc lập, tuyệt đối không đụng tunnel hay dữ liệu cache thật của anh Zon.
  - Không sử dụng file mẫu EICAR hay mã độc trong quá trình đo đạc và kiểm thử.
  - Mỗi hạng mục tương ứng một commit độc lập có giải trình rõ ràng.

---

## 2. Chi tiết 6 hạng mục kỹ thuật triển khai

### Mục 1 (P-4 measurement): Đo lường thời gian từng chặng trong Scanner
- **Commit**: `a424739` `perf(scanner): add per-file stage timings measurement and record baseline (P-4)`
- **Files**: `src-tauri/src/scanner.rs`, `src-tauri/examples/folder_bench.rs`, `docs/qa/perf-stages-0.3.1.md`
- **Cơ chế**:
  - Bổ sung cấu trúc `StageTimings` đo đạc 7 chặng độc lập per-file:
    1. `wait_limiter`: Thời gian chờ cấp quota permit từ `ScanLimiter`.
    2. `open_and_hash`: Thời gian mở tệp, băm SHA-256 ban đầu và kiểm tra metadata snapshot trước/sau.
    3. `cache_lookup`: Thời gian mở kết nối/truy vấn `cache::get` và kiểm tra lại metadata khi hit.
    4. `tcp_connect`: Thời gian thiết lập socket TCP tới clamd (`TcpStream::connect`).
    5. `send_instream`: Thời gian gửi lệnh `zINSTREAM\0`, truyền dữ liệu theo chunk và frame kết thúc 0.
    6. `clamd_wait`: Thời gian chờ clamd xử lý và đọc frame kết quả NUL-terminated (`read_nul_frame` + `read_until_eof` + `parse_response`).
    7. `cache_write`: Thời gian ghi verdict vào cache SQLite (`cache::put`).
  - Sử dụng `std::time::Instant` chi phí overhead cực thấp (~vài nano-giây).
  - Không ghi log từng file trên đường chạy chính của ứng dụng để bảo toàn trọn vẹn hiệu năng.
  - Nâng cấp công cụ `folder_bench.rs` tính toán thống kê chi tiết: Tổng thời gian (sum), phân vị p50, phân vị p95, và thời gian lớn nhất (max) cho từng chặng.
  - Ghi nhận đầy đủ số liệu baseline "Trước" vào tài liệu `docs/qa/perf-stages-0.3.1.md`.

### Mục 2 (P-2): Tái sử dụng SQLite Connection & Gom lô giao dịch (Batch Transactions)
- **Commit**: `cec6819` `perf(cache): share sqlite connection with batch transactions and cached statements (P-2)`
- **Files**: `src-tauri/src/cache.rs`, `src-tauri/src/scanner.rs`, `src-tauri/src/directory.rs`, `src-tauri/src/lib.rs`
- **Cơ chế**:
  - **Shared Connection**: Thay vì mở và đóng kết nối SQLite cho từng file (chi phí I/O mở file và khởi tạo SQLite engine lặp lại hàng nghìn lần), toàn bộ phiên quét sử dụng chung một `Arc<CacheManager>` chứa `Mutex<Connection>`.
  - **Cached Statements**: Sử dụng `conn.prepare_cached(...)` cho các câu lệnh SQL tra cứu `SELECT` và ghi `INSERT OR REPLACE`, tận dụng statement handle đã biên dịch sẵn trong SQLite memory cache.
  - **Batch Transactions**: `cache::put` không commit từng bản ghi đơn lẻ (tốn fsync disk I/O) mà đưa vào bộ đệm lô (`batch_buffer`), tự động kích hoạt `COMMIT TRANSACTION` khi đạt $\le 200$ dòng hoặc sau $\le 1$ giây kể từ bản ghi đầu tiên của lô.
  - **Zero-Loss Flush**: Bộ đệm tự động được flush toàn bộ khi kết thúc phiên quét bình thường hoặc khi người dùng Hủy (`cancel`), bảo toàn 100% verdict đã có. Triển khai thêm trait `Drop` cho `CacheManager` để đảm bảo flush ngay cả khi thread/runtime bị hủy đột ngột.
  - **Thread Isolation**: Toàn bộ thao tác SQLite rusqlite đều chạy trong `tokio::task::spawn_blocking` để không làm nghẽn Tokio async executor.
  - Bảo toàn 100% các tính năng an toàn từ V-57: S-6 (auto-recovery DB hỏng), S-7 (treat read error as miss), X-5 (khóa thế hệ chữ ký).
  - Bổ sung unit tests kiểm chứng: `batch_flush_on_timeout`, `batch_flush_on_drop_preserves_verdicts`, `cancel_does_not_lose_verdicts`.

### Mục 3 (P-3): Băm tệp trong spawn_blocking với đệm 1 MiB & INSTREAM Chunking
- **Commit**: `98805fa` `perf(scanner): read and hash files with 1 MiB buffer in spawn_blocking (P-3)`
- **Files**: `src-tauri/src/scanner.rs`
- **Cơ chế**:
  - Quá trình đọc và băm file SHA-256 ban đầu được gói hoàn toàn vào trong một `spawn_blocking` duy nhất, sử dụng `std::fs::File` kết hợp với `std::io::BufReader::with_capacity(1024 * 1024, file)` (bộ đệm 1 MiB).
  - Tránh chi phí context switch liên tục của async file I/O trên Windows filesystem khi băm các file dung lượng từ vài trăm KiB đến 25 MiB.
  - Duy trì kiểm tra metadata snapshot trước và sau khi băm (`size` và `mtime`): nếu tệp bị sửa đổi trong lúc băm, lập tức đánh dấu `ScanStatus::Unknown` chống TOCTOU race condition.
  - Chặng gửi INSTREAM sang clamd daemon: đọc dữ liệu từ đĩa với khối lớn hơn (bộ đệm $\le 1$ MiB), sau đó cắt nhỏ và đóng frame clamd theo đúng chuẩn giao thức clamd (`StreamMaxLength`, mặc định $\le 64$ KiB / frame) với độ dài 4-byte big-endian và frame kết thúc 0.

### Mục 4 (P-6): Đọc NUL Frame theo khối với kiểm thực phân khung nghiêm ngặt
- **Commit**: `811e5f7` `perf(scanner): chunked read_nul_frame with framing validation (P-6)`
- **Files**: `src-tauri/src/scanner.rs`
- **Cơ chế**:
  - Thay thế vòng lặp đọc từng byte đơn lẻ (`read_exact(&mut [0u8; 1])`) bằng cơ chế đọc theo khối vào bộ đệm tái sử dụng `[u8; 512]`.
  - Giảm thiểu tới 99% số lượng lệnh gọi hàm syscall I/O trên socket TCP.
  - Vẫn siết chặt giới hạn trần `MAX_RESPONSE_BYTES = 4096` bytes chống tấn công làm cạn kiệt bộ nhớ.
  - Vẫn duy trì nguyên tắc bảo mật từ P0-01: từ chối tuyệt đối bất kỳ byte dữ liệu thừa nào xuất hiện sau byte kết thúc `\0` (NUL terminator). Nếu có dữ liệu thừa sau NUL, trả về lỗi framing và đánh dấu kết quả là `ScanStatus::Error`.
  - Giữ nguyên cơ chế timeout ban đầu 120 giây (phù hợp quét tệp lớn/nén sâu) và watchdog timeout giữa các khối.
  - Bổ sung unit tests: `chunked_read_nul_frame_multi_chunk`, `chunked_read_nul_frame_rejects_trailing_data`, `chunked_read_nul_frame_exceeds_max_bytes`.

### Mục 5 (P-7): Chuẩn hóa trước danh sách loại trừ cho Quick Scan
- **Commit**: `587131a` `perf(quick): pre-normalize exclusion list once before directory walk (P-7)`
- **Files**: `src-tauri/src/directory.rs`
- **Cơ chế**:
  - Trước khi duyệt thư mục hệ thống trong Quick Scan, toàn bộ danh sách đường dẫn loại trừ (bao gồm `%SystemRoot%\System32\drivers\DriverData`) được chuẩn hóa một lần duy nhất (`canonicalize`, loại bỏ prefix `\\?\`, chuyển chữ thường trên Windows).
  - Hàm `is_path_excluded` nhận danh sách đã chuẩn hóa dưới dạng slice `&[NormalizedPath]`, tránh việc lặp đi lặp lại thao tác chuẩn hóa, giải phóng và cấp phát bộ nhớ cho từng tệp trong suốt quá trình duyệt đệ quy cây thư mục.
  - Bổ sung unit test: `pre_normalized_exclusions_match_canonically`.

### Mục 6 (S-10): Phát hiện ClamAV Version thay đổi giữa phiên quét
- **Commit**: `e9bfc17` `fix(scanner): detect clamd version change mid-scan and invalidate stale cache keys (S-10)`
- **Files**: `src-tauri/src/scanner.rs`, `src-tauri/src/directory.rs`
- **Cơ chế**:
  - Trong các phiên quét thư mục lớn kéo dài hàng chục phút, nếu ClamAV daemon trên server được cập nhật phiên bản hoặc nạp database mới làm thay đổi chuỗi phiên bản (`zVERSION`), việc lưu verdict vào cache dưới `engine_version` cũ sẽ gây sai lệch dữ liệu cache (cache poisoning).
  - Scanner chủ động kiểm tra lại `zVERSION` của clamd định kỳ mỗi 100 tệp quét (`count.is_multiple_of(100)`) hoặc ngay khi gặp sự cố kết nối.
  - Nếu phát hiện chuỗi phiên bản thay đổi: lập tức cập nhật `engine_version` hoạt động cho các tệp tiếp theo; toàn bộ các kết quả quét trong lúc chuyển giao không được phép ghi đè vào cache với khóa phiên bản cũ.
  - Bổ sung unit test: `clamd_version_change_invalidates_cache_key`.

---

## 3. Bảng so sánh Hiệu năng Thực nghiệm (Trước vs Sau)

Được đo lường độc lập bằng công cụ `folder_bench` trên môi trường thực nghiệm lab qua tunnel `127.0.0.1:13310`:

### 3.1. So sánh trên tập lớn: `C:\Program Files` (Cold Scan, 12,886 files)

| Chỉ số / Chặng | Trước tối ưu | Sau tối ưu | Chênh lệch | Đánh giá |
|---|---:|---:|---:|---|
| **Tổng Wall Clock** | **920,445 ms** (~15.3 phút) | **613,718 ms** (~10.2 phút) | **-33.3%** | **Nhanh hơn 5.1 phút toàn bộ phiên quét** |
| **CPU Time** | **73.73 s** | **47.31 s** | **-35.8%** | **Giảm 35.8% tải CPU của client** |
| `cache_write` (Tổng) | 41,721 ms | 1,664 ms | **-96.0%** | **Nhanh gấp 25 lần** (nhờ gom lô SQLite transactions P-2) |
| `cache_write` (p50) | 2.63 ms | **0.046 ms** (46 $\mu$s) | **-98.2%** | Giảm gần như triệt để chi phí ghi cache |
| `send_instream` (Tổng) | 132,719 ms | 21,722 ms | **-83.6%** | **Nhanh gấp 6.1 lần** (nhờ đệm 1 MiB P-3) |
| `send_instream` (Max) | 23,641 ms | 718 ms | **-97.0%** | Loại bỏ hoàn toàn tình trạng nghẽn truyền file lớn |
| `cache_lookup` (Tổng) | 22,437 ms | 4,437 ms | **-80.2%** | **Nhanh gấp 5 lần** (nhờ `prepare_cached` P-2) |
| `cache_lookup` (p50) | 1.07 ms | **0.21 ms** | **-80.4%** | Tra cứu cache cực kỳ nhẹ |
| `wait_limiter` (Tổng) | 27,574,777 ms | 18,373,187 ms | **-33.4%** | Thời gian chờ hàng đợi giảm tương ứng khi client xử lý nhanh |
| `clamd_wait` (Tổng) | 1,444,957 ms | 1,035,501 ms | -28.3% | Phụ thuộc tốc độ phân tích của clamd daemon trên lab server |
| `tcp_connect` (Tổng) | 20,961 ms | 18,914 ms | -9.8% | Ổn định ~0.35 ms / file qua loopback socket |

### 3.2. So sánh Quick Scan Warm (100% Cache Hit, 589 files)

| Chỉ số / Chặng | Trước tối ưu | Sau tối ưu | Chênh lệch | Đánh giá |
|---|---:|---:|---:|---|
| **Tổng Wall Clock** | **1,443 ms** | **978 ms** | **-32.2%** | **Hoàn tất quét dưới 1 giây** |
| `cache_lookup` (Tổng) | 1,162.8 ms | 224.1 ms | **-80.7%** | **Nhanh gấp 5.2 lần** |
| `cache_lookup` (p50) | 1.02 ms | **0.23 ms** | **-77.5%** | Tốc độ tra cứu cache trung vị 230 microseconds |
| `open_and_hash` (Tổng) | 1,712.1 ms | 1,431.8 ms | -16.4% | Giảm nhẹ nhờ I/O tối ưu |

---

## 4. Phân tích & Đề xuất chính thức về IDSESSION (P-4)

### 4.1. Thời gian quét thực tế nằm ở đâu?
- Trên tập 12,886 files của `C:\Program Files`, **hơn 95% thời gian quét nằm ở `clamd_wait` và `wait_limiter`**:
  - `clamd_wait`: Chiếm 1,035.5 giây (~17.2 phút CPU xử lý của clamd daemon để giải nén PE, phân tích byte và đối chiếu tập chữ ký 8.6 triệu mẫu).
  - `wait_limiter`: Chiếm 18,373 giây (chia đều cho `concurrency = 2` luồng thích ứng = ~9,186 giây thời gian file nằm chờ permit trong hàng đợi).
- Toàn bộ chi phí I/O client phía SQLite (`cache_lookup` + `cache_write`) chỉ còn vỏn vẹn **6.1 giây** trên 12,886 files (**dưới 1% tổng thời gian**).

### 4.2. IDSESSION (P-4) có đáng làm không?
- **Số liệu đo đạc thực tế**: Tổng thời gian `tcp_connect` trên toàn bộ 12,886 files chỉ tốn **18.91 giây**, chiếm đúng **3.08%** tổng wall-clock (613.7 giây).
- **Rủi ro kỹ thuật nghiêm trọng nếu triển khai IDSESSION**:
  1. **Nguy cơ đứt gãy do `IdleTimeout 30`**: Cấu hình ClamAV lab quy định `IdleTimeout 30` (theo tài liệu Q-42). Nếu giữ kết nối persistent IDSESSION, bất kỳ khi nào client đọc một file dung lượng lớn hoặc hàng đợi I/O đĩa bị nghẽn quá 30 giây, clamd daemon sẽ chủ động đóng socket (`Broken pipe`). Việc này đòi hỏi client phải viết cơ chế tái kết nối, re-handshake IDSESSION và xử lý resend vô cùng phức tạp và dễ phát sinh lỗi chập chờn.
  2. **Chiếm dụng thread pool của clamd**: clamd cấu hình `MaxThreads 12`. Một kết nối IDSESSION kéo dài sẽ giữ chặt thread phục vụ trong suốt thời gian client quét, làm giảm khả năng đáp ứng của server cho các tiến trình khác.
  3. **Lợi ích cận biên quá nhỏ**: Ngay cả trong điều kiện lý tưởng triệt tiêu 100% thời gian bắt tay TCP, tổng thời gian quét 12,886 files chỉ giảm thêm tối đa **18 giây** trên 10 phút quét (~3%).
- **ĐỀ XUẤT CHÍNH THỨC**:
  - **KHÔNG NÊN LÀM IDSESSION (P-4)**. Kiến trúc kết nối TCP per-file kết hợp với `ScanLimiter` hiện tại là hoàn toàn tối ưu, cực kỳ tin cậy, không chịu rủi ro `IdleTimeout` và giải phóng tức thời tài nguyên mạng.

---

## 5. Kết quả Kiểm thử & Quality Gates

Toàn bộ các tiêu chuẩn kiểm thử đã được thực hiện nghiêm ngặt trên môi trường cô lập:
1. `cargo fmt --check`: **PASS** (100% chuẩn format Rust).
2. `cargo clippy --all-targets -- -D warnings`: **PASS** (0 warnings).
3. `cargo test`: **PASS 3 lần liên tiếp**:
   - Run 1: 81 lib tests + 1 bin test + 18 resilience tests = **100 tests PASSED** (0 failed).
   - Run 2: 81 lib tests + 1 bin test + 18 resilience tests = **100 tests PASSED** (0 failed).
   - Run 3: 81 lib tests + 1 bin test + 18 resilience tests = **100 tests PASSED** (0 failed).
4. `npm.cmd run build`: **PASS** (Vite + TypeScript biên dịch thành công trong 1.03s).
5. `node tests/native/run.mjs`: **PASS 2 lần liên tiếp**:
   - Run 1: 21 native UI webdriverio tests **PASSED** (100%).
   - Run 2: 21 native UI webdriverio tests **PASSED** (100%).
6. Đóng gói bộ cài NSIS:
   - Output: `E:\Zonendpoint-tools\artifacts\v61-release\Zonendpoint_0.3.1_x64-setup.exe`
   - Kích thước: 3,667,853 bytes
   - SHA256: `01793d12cede23228b4eeb3530b224ebdcd3264c22df261e3fd1b07d9af73fe7` (đã lưu kèm `.sha256`).

---

## 6. Danh sách Commits trên nhánh `vic/031-perf`

```
* be58cde test(native): ensure active scan in V-30 close test selects freshly written fixture
* 9966b2d docs(qa): record post-optimization perf benchmarks and P-4 IDSESSION analysis
* e9bfc17 fix(scanner): detect clamd version change mid-scan and invalidate stale cache keys (S-10)
* 587131a perf(quick): pre-normalize exclusion list once before directory walk (P-7)
* 811e5f7 perf(scanner): chunked read_nul_frame with framing validation (P-6)
* 98805fa perf(scanner): read and hash files with 1 MiB buffer in spawn_blocking (P-3)
* cec6819 perf(cache): share sqlite connection with batch transactions and cached statements (P-2)
* a424739 perf(scanner): add per-file stage timings measurement and record baseline (P-4)
* e35f4d7 chore(release): bump version to 0.3.1 and update documentation (tip V-57)
```
