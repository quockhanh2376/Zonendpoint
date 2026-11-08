# Báo Cáo Kiểm Thử Độc Lập LAB-RESILIENCE (Qeen) — Commit e1a48d3

- **Người thực hiện:** Qeen (Independent Tester & Security Reviewer)
- **Đối tượng kiểm thử:** Nhánh `vic/lab-resilience-plan` tại commit `e1a48d3`
- **Worktree kiểm thử:** `E:\Zonendpoint-Qeen-e1a48d3` (detached HEAD)
- **Thời gian thực hiện:** 2026-11-07
- **Lab ClamAV:** Daemon ClamAV qua SSH tunnel `127.0.0.1:13310` (`ClamAV 1.5.4/28176`)
- **Tài liệu tham chiếu:**
  - Handoff của Vic: `docs/handoffs/resilience-vic-to-qeen.md`
  - Đánh giá kế hoạch & Checklist 7 bước của Qeen: `E:\Zonendpoint-tools\artifacts\resilience\qeen-plan-review.md`

---

## 1. Tóm Tắt Kết Quả (Executive Summary)

| Hạng mục kiểm tra | Trạng thái | Ghi chú |
|---|:---:|---|
| **Check A: Mã nguồn & Build** | **PASS / FINDING** | 37 unit/integration tests passed, fmt/clippy clean, build thành công. Phát hiện vấn đề kiến trúc tại P0-01 (`try_read`). |
| **Check B: Release Guard** | **PASS** | `native-ui-tests` bị chặn cứng bởi `compile_error!` trong release; hằng số 60s áp dụng vô điều kiện. |
| **Check C: Tier 2 Visible Run (7 bước)** | **PASS** | Chạy trực quan trên desktop, dừng ~3s tại các trạng thái lỗi/kết quả, chụp ảnh màn hình đầy đủ. Cache SQLite an toàn. |
| **Check D: Proxy Harness** | **PASS** | Binds loopback duy nhất (`127.0.0.1`), từ chối LAN IP (`ECONNREFUSED`), đóng cổng sạch khi `QUIT`. |

---

## 2. Chi Tiết Các Hạng Mục Kiểm Tra

### Check A: Mã nguồn, Static Analysis & Đánh giá các bản sửa lỗi của Vic
1. **Kiểm tra công cụ tự động:**
   - `cargo test --locked`: **PASS** (37 tests: 25 library unit tests, 1 CLI test, 11 resilience integration tests).
   - `cargo fmt --check`: **PASS** (code 0, code style chuẩn).
   - `cargo clippy --locked --all-targets -- -D warnings`: **PASS** (code 0, không có cảnh báo nào).
   - `npm run build`: **PASS** (Vite v6.4.1 + TypeScript compilation hoàn tất không lỗi).

2. **Đánh giá các bản sửa lỗi của Vic:**
   - **P0-02 (Offline Cached Clean False-All-Good):** Đã khắc phục triệt để. Hàm `scan_file_with_engine_timeout` bắt buộc truy vấn `clamd_version` trước khi kiểm tra cache SQLite. Khi daemon offline hoặc đứt kết nối, lỗi được trả về ngay lập tức và UI không thể hiển thị `All Good`.
   - **P1-01 (Single worker lock contention):** Đã tách biệt semaphore tải tập tin với `clamd_version` và `ScanManager`, loại bỏ triệt để nguy cơ deadlock.
   - **P1-02 (Recheck cache TOCTOU race):** Đã bổ sung bước recheck `hash_file` đối chiếu hash, size và mtime của file trước khi tái sử dụng cache.
   - **P1-03 (Cancelling UI latency):** Đã bổ sung transition `Stopping…` với thời lượng tối thiểu 250ms cho người dùng quan sát.
   - **P2-02 (Missing checkmark icon in All Good):** Đã thêm thẻ SVG `.scan-check` màu xanh lục khi hoàn tất quét sạch.

---

## 3. Check B: Release Guard Verification
1. **Kiểm tra cờ thử nghiệm:**
   - Chạy lệnh `cargo check --release --features native-ui-tests`: **PASS** (Biên dịch thất bại đúng thiết kế với thông báo: `compile_error!("native-ui-tests must never be enabled in release builds")`).
2. **Kiểm tra biến môi trường timeout trong bản release:**
   - Trong `src-tauri/src/scanner.rs`, hàm `default_scan_timeout()` trong bản release trả về trực tiếp `Duration::from_secs(60)` mà không đọc biến môi trường `ZONENDPOINT_TEST_SCAN_TIMEOUT_MS`.

---

## 4. Check D: Proxy Harness Isolation
- Script: `E:\Zonendpoint-tools\artifacts\resilience\qeen\test-proxy-harness.mjs`
- Đã xác minh:
  - Proxy lắng nghe trên `127.0.0.1:13399` và Control port `127.0.0.1:13398`.
  - Kết nối thử nghiệm từ IP mạng nội bộ (`100.96.250.76:13399`) bị từ chối kết nối ngay lập tức (`ECONNREFUSED`).
  - Lệnh điều khiển `STATUS`, `SET FAULT=...`, `QUIT` hoạt động chính xác và dọn dẹp tiến trình khi kết thúc.

---

## 5. Check C: Tier 2 Visible Verification (7 Bước)
- **Bộ kiểm thử:** `E:\Zonendpoint-tools\artifacts\resilience\qeen\resilience-tier2-visible-suite.mjs`
- **File mẫu:** `E:\Zonendpoint-tools\artifacts\resilience\qeen\fixtures\audit-sample-02.txt`
- **Tài khoản / Môi trường:** Windows Defender chạy song song, không tắt AV, không thêm exclusion.

### Nhật ký 7 bước thực thi trực quan:
1. **Bước 1: Khởi động & Tiền cảnh (Startup & Foreground):**
   - App khởi chạy với Light theme (`data-theme="light"`).
   - Mở menu chọn File qua UIAutomation và gán đường dẫn test fixture an toàn.
   - Nút chính đổi từ `Quick Scan` thành `Scan`.
   - Dừng ~3s quan sát. Ảnh chụp: `step1-startup-foreground.png`.
   - **Kết quả: PASS**.

2. **Bước 2: Fault: Mid-stream Reset (`reset-instream`):**
   - Proxy reset socket client ngay khi nhận `zINSTREAM`.
   - Tiêu đề UI hiển thị `Scan incomplete`, cảnh báo: *"Do not treat an incomplete scan as clean."*
   - Nút bấm giữ nguyên trạng thái `Scan`, tuyệt đối KHÔNG xuất hiện `All Good`.
   - Dừng ~3s quan sát. Ảnh chụp: `step2-midstream-reset.png`.
   - **Kết quả: PASS**.

3. **Bước 3: Fault: Stall & User Cancel (`stall-instream`):**
   - Proxy giữ socket treo không phản hồi.
   - Nút bấm chuyển sang `Stop`. Người dùng nhấn nút `Stop`.
   - Giao diện chuyển qua trạng thái huỷ quét, hiển thị tiêu đề `Scan cancelled`.
   - Tuyệt đối KHÔNG hiển thị `All Good`.
   - Dừng ~3s quan sát. Ảnh chụp: `step3-stall-cancel.png`.
   - **Kết quả: PASS**.

4. **Bước 4: Fault: Malformed Response (`corrupt-response`):**
   - Proxy trả về frame lỗi `garbage OK\0`.
   - Parser phát hiện phản hồi bất hợp lệ: `unrecognized clamd response`.
   - UI hiển thị `Scan incomplete`, lỗi không biến thành Clean.
   - Dừng ~3s quan sát. Ảnh chụp: `step4-corrupt-response.png`.
   - **Kết quả: PASS**.

5. **Bước 5: Khôi phục: Clean Scan (`passthrough`):**
   - Proxy chuyển về chế độ chuyển tiếp trong suốt tới ClamAV lab thật (`127.0.0.1:13310`).
   - Quét hoàn tất với 1 file clean.
   - Nút chính chuyển sang `All Good` màu xanh lục sáng kèm icon SVG checkmark `.scan-check`.
   - Dừng ~3s quan sát. Ảnh chụp: `step5-clean-allgood.png`.
   - **Kết quả: PASS**.

6. **Bước 6: Fault: Bump Version & Offline (Tunnel stopped + Cached clean file):**
   - Dừng hoàn toàn tiến trình SSH tunnel lab `13310`. Xác nhận cổng `13310` đã đóng (`isOffline = true`).
   - Proxy chuyển sang chế độ `bump-identity`.
   - Nhấn `Scan` trên chính file vừa quét sạch ở Bước 5.
   - App phát hiện lỗi kết nối tới ClamAV (`Could not connect to ClamAV`), **KHÔNG tái sử dụng cache Clean trước đó**.
   - Nút bấm KHÔNG hiển thị `All Good`.
   - Khởi động lại SSH tunnel `13310` và xác nhận cổng `13310` hoạt động trở lại.
   - Dừng ~3s quan sát. Ảnh chụp: `step6-offline-cache-miss.png`.
   - **Kết quả: PASS**.

7. **Bước 7: Hậu kiểm Cache SQLite:**
   - Kiểm tra cơ sở dữ liệu `cache.db` tại `sandbox-suite/appdata/Zonendpoint/cache.db`.
   - Truy vấn `scan_cache`:
     - **Số lượng bản ghi:** Đúng 1 bản ghi duy nhất (từ lần quét sạch ở Bước 5).
     - **Trạng thái:** `CLEAN`.
     - **Policy version:** `zonendpoint-mvp-1`.
     - **Engine version:** `ClamAV 1.5.4/28176/Sat Nov  7 06:24:18 2026`.
     - Tuyệt đối KHÔNG có bản ghi nào của các lần quét lỗi, bị huỷ, hoặc mất kết nối.
   - Xuất file báo cáo: `tier2-visible-report.json`.
   - **Kết quả: PASS**.

---

## 6. Danh Sách Phát Hiện & Khuyến Nghị (Findings)

### Finding 1 [Security / Architecture]: Lỗ hổng Race Condition của `try_read` khi kiểm tra frame thừa sau NUL terminator (P0-01)
- **Vị trí:** `src-tauri/src/scanner.rs:262-267`
```rust
let response = read_nul_frame(&mut socket, MAX_RESPONSE_BYTES).await?;
let mut extra = [0u8; 1];
if let Ok(n) = socket.try_read(&mut extra)
    && n > 0
{
    return Err("clamd sent unexpected extra data after response terminator".into());
}
```
- **Phân tích rủi ro:**
  `socket.try_read` chỉ kiểm tra dữ liệu tức thời đang nằm sẵn trong buffer nhận TCP của hệ điều hành ngay tại thời điểm vừa đọc xong ký tự NUL. Nếu một proxy trung gian, mạng bị trễ packet, hoặc daemon ClamAV bị lỗi gửi `stream: OK\0` và sau đó vài mili-giây mới gửi tiếp frame thứ hai (ví dụ: `stream: Threat FOUND\0`), lệnh `try_read` sẽ nhận kết quả `WouldBlock` (`n == 0`/error) và điều kiện kiểm tra bị bỏ qua. Kết quả là Zonendpoint chấp nhận kết quả `Clean`, hiển thị `All Good` và ghi vào cache SQLite.
- **Khuyến nghị cho Vic:**
  Đối với giao thức single-response như `INSTREAM`, sau khi ghi nhận phản hồi, luồng socket phải được xác nhận đóng bởi phía gửi (đọc cho tới khi gặp EOF / `read() == Ok(0)`) trước khi cam kết kết luận `Clean`.

### Finding 2 [Harness / Infrastructure]: Race condition mất chunk dữ liệu trong `scripts/resilience-proxy.mjs`
- **Vị trí:** `scripts/resilience-proxy.mjs:153`
- **Phân tích:**
  Khi proxy nhận diện lệnh `zINSTREAM\0`, hàm `connectUpstream` được gọi bất đồng bộ. Trong khoảng thời gian chờ upstream TCP bắt tay thành công, nếu client gửi tiếp các chunk dữ liệu của file, các chunk này bị rơi vào nhánh `else` và bị bỏ qua do `upstreamSocket` chưa được gán hoặc chưa kết nối, dẫn đến upstream clamd đóng kết nối với lỗi `clamd closed before response terminator`.
- **Khắc phục:** Đã bổ sung cơ chế `clientSocket.pause()` và bộ đệm `pendingChunks` trong proxy của worktree kiểm thử. Vic cần cập nhật bản vá này vào branch chính.

### Finding 3 [Automation / UI]: Cấu trúc điều khiển hộp thoại file Windows 11
- Hộp thoại `IFileOpenDialog` trên Windows 11 sử dụng `ComboBoxEx32` cho control `1148` không hỗ trợ `ValuePattern` của UIAutomation. Script kiểm thử cần gán đường dẫn qua `WM_SETTEXT` vào handle của `Edit` con và kích hoạt nút Open.

---

## 7. Danh Mục Minh Chứng (Artifacts & Evidence)

Tất cả các tệp chứng cứ được lưu trữ tại `E:\Zonendpoint-tools\artifacts\resilience\qeen\`:
1. `step1-startup-foreground.png`: Ảnh chụp giao diện khởi động tiền cảnh, theme sáng, đã chọn target.
2. `step2-midstream-reset.png`: Ảnh chụp thông báo `Scan incomplete` khi bị reset stream.
3. `step3-stall-cancel.png`: Ảnh chụp trạng thái `Scan cancelled` khi người dùng nhấn Stop.
4. `step4-corrupt-response.png`: Ảnh chụp thông báo `Scan incomplete` khi nhận phản hồi dị dạng.
5. `step5-clean-allgood.png`: Ảnh chụp nút `All Good` màu xanh lục kèm icon checkmark.
6. `step6-offline-cache-miss.png`: Ảnh chụp thông báo lỗi ClamAV khi daemon offline, không tái sử dụng cache.
7. `tier2-visible-report.json`: Báo cáo chi tiết 7 bước và nội dung kiểm tra SQLite cache.
