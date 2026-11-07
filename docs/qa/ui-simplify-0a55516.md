# Báo Cáo Kiểm Thử Độc Lập UI-SIMPLIFY & P0-01 (Qeen) — Commit 0a55516

- **Người thực hiện:** Qeen (Independent Tester & Security Reviewer)
- **Đối tượng kiểm thử:** Nhánh `vic/ui-simplify` tại commit `0a55516` (bao gồm giao diện UI-SIMPLIFY và bản sửa lỗi P0-01)
- **Worktree kiểm thử:** `E:\Zonendpoint-Qeen-0a55516` (detached HEAD tại `0a555169fb6e72db82456e568f385982a8021e39`)
- **Thời gian thực hiện:** 2026-11-07
- **Lab ClamAV:** SSH tunnel `127.0.0.1:13310` nối tới lab daemon ClamAV (`100.96.250.245:3310`)
- **Tài liệu tham chiếu:**
  - Handoff của Vic: `docs/handoffs/p0-01-fix-vic-to-qeen.md` và `docs/handoffs/ui-simplify-vic-to-qeen.md`
  - Đặc tả: `docs/tasks/ui-simplify-preflight.md`

---

## 1. Xác Minh Cổ Tố & Tính Toàn Vẹn Của Bản Build (Artifact Provenance)

Toàn bộ artifact được kiểm tra mã băm SHA256 trước khi cài đặt vào sandbox:

| Artifact | Đường dẫn kiểm tra | SHA256 Kỳ vọng | SHA256 Thực tế | Kết quả |
|---|---|---|---|:---:|
| **NSIS Installer** | `E:\Zonendpoint-tools\artifacts\0a55516\Zonendpoint_0.1.0_x64-setup.exe` | `A0C9E63F3358C92E3EC56022C7DB16287D055CB190BAF4AD4DC45C3D40727252` | `A0C9E63F3358C92E3EC56022C7DB16287D055CB190BAF4AD4DC45C3D40727252` | **MATCH** |
| **Standalone Exe** | `E:\Zonendpoint-tools\artifacts\0a55516\zonendpoint.exe` | `AF414BC52E8D06015243CAFDDA5F0712ABB29F5AF7565F54B4A495E948367FD4` | `AF414BC52E8D06015243CAFDDA5F0712ABB29F5AF7565F54B4A495E948367FD4` | **MATCH** |

- **Provenance Check:** So sánh binary bên trong NSIS payload với standalone executable: Trùng khớp 100% ngoại trừ 3 bytes đánh dấu định dạng NSIS Tauri cho phép (`_TYPE_VAR_UNK` -> `_TYPE_VAR_NSS`).
- **Sandbox Install:** Cài đặt qua `/S /D=E:\Zonendpoint-tools\artifacts\0a55516\qeen\sandbox\app`. Dữ liệu người dùng tại `C:\Users\admzon\AppData\Local\Zonendpoint\cache.db` được bảo toàn nguyên vẹn sau khi gỡ cài đặt.

---

## 2. Kết Quả Kiểm Tra Tự Động & Mã Nguồn

- `cargo test --locked`: **PASS** (46 tests: 29 lib + 1 main + 16 resilience, bao gồm đầy đủ 5 test tích hợp P0-01).
- `cargo fmt --check`: **PASS** (Chuẩn định dạng Rust).
- `cargo clippy --locked --all-targets -- -D warnings`: **PASS** (0 warnings).
- `npm.cmd run build`: **PASS** (Frontend Vite + TypeScript build thành công).
- **DOM & String Sanitization:** Kiểm tra `dist/` và DOM runtime: Tuyệt đối không xuất hiện từ khóa "ClamAV", địa chỉ IP (`127.0.0.1`, `100.96.250.245`), hay cổng mạng (`13310`, `3310`, `13399`).

---

## 3. Nhật Ký Chạy Thực Quan Desktop (Visible Native Run)

Bộ kiểm thử `E:\Zonendpoint-tools\artifacts\0a55516\qeen\qeen-suite-0a55516.mjs` chạy trực tiếp trên desktop với ứng dụng sandbox, dừng ~3s tại các trạng thái chuyển tiếp chính:

1. **Check 1 (Menu Select target & Nút mặc định):**
   - Menu "Select target" hiển thị chính xác chỉ 2 tùy chọn: `File` và `Folder`. Không chứa "Quick Scan".
   - Nút hành động chính mặc định là `Quick Scan`.
   - Ảnh chụp: `check1-menu-open.png`.
2. **Check 2 & 3 (Quick Scan & Bố cục kết quả Scan incomplete):**
   - Chạy Quick Scan qua lab tunnel thực tế (589 mục).
   - Phát hiện lỗi truy cập `C:\WINDOWS\system32\drivers\DriverData: Access is denied. (os error 5)` dẫn tới kết quả `Scan incomplete` (Chi tiết tại Finding Q-4-01).
   - Bố cục kết quả `Scan incomplete` chuẩn xác: Chỉ hiển thị các dòng lỗi/non-clean (1 dòng lỗi DriverData), ẩn toàn bộ 588 file clean; hiển thị khối cảnh báo màu vàng: *"Do not treat an incomplete scan as clean"*.
   - Ảnh chụp: `check3-scan-incomplete-driverdata.png`.
3. **Check 4 (Preflight Server Check):**
   - **Invalid banner:** Proxy trả về banner không hợp lệ (`GARBAGE_BANNER 1.0`). Ứng dụng hiển thị thông báo lỗi duy nhất: `"Cannot connect to ZonEndpoint server"`. Quét không được bắt đầu. Ảnh: `check4-invalid-banner.png`.
   - **Tunnel stopped (Offline):** Dừng SSH tunnel trên 13310. Bấm Quick Scan: Ứng dụng hiển thị thông báo `"Cannot connect to ZonEndpoint server"`. File đã cache trước đó tuyệt đối không bị hiển thị giả mạo `All Good`. Quét không bắt đầu. Ảnh: `check4-stopped-tunnel-preflight.png`.
4. **Check 5 (Status Icon Transitions):**
   - Trạng thái kết nối ban đầu: Xanh lục, tooltip `"Connected to ZonEndpoint server"`.
   - Khi ngắt kết nối SSH tunnel: Trong vòng <30s, biểu tượng chuyển sang Đỏ (disconnected) với tooltip `"Cannot connect to ZonEndpoint server. Check your connection."`. Ảnh: `check5-status-red.png`.
   - Nhấp vào biểu tượng đỏ khi offline: Trigger kiểm tra ngay lập tức, UI hoàn toàn không bị đóng băng/treo.
   - Bật lại tunnel trên 13310 và nhấp biểu tượng: Lập tức chuyển lại Xanh lục (`connected`), tooltip `"Connected to ZonEndpoint server"`. Ảnh: `check5-status-green.png`.
5. **Check 6 (Copy Sanitization):**
   - Kiểm tra runtime HTML source: Toàn bộ thông báo, tooltip, text không chứa "ClamAV", IP hay cổng.
6. **Check 7 (P0-01 Live Resilience Verification trên 0a55516):**
   - **Delayed second frame (50ms):** Proxy gửi stream reply `stream: OK\0` sau đó delay 50ms gửi tiếp frame thứ 2 `stream: Win.Test.Threat FOUND\0`. Bộ đệm bounded EOF drain trong `scanner.rs` phát hiện extra data, từ chối file thành `ScanStatus::Error`. Kết quả UI: `Scan incomplete`, tuyệt đối không bao giờ hiển thị `All Good` hay `Clean`.
   - **Delayed second frame (500ms):** Tương tự, delay 500ms kích hoạt lỗi EOF drain, trả về `ScanStatus::Error`, UI hiển thị `Scan incomplete`.
   - **Never-closing socket:** Proxy giữ socket mở không đóng EOF. Sau đúng 1.0 giây timeout (`EOF_DRAIN_TIMEOUT`), ứng dụng ghi nhận lỗi timeout, trả về `ScanStatus::Error`, UI hiển thị `Scan incomplete`. Tuyệt đối không treo vô hạn hay chuyển thành `Clean`.

---

## 4. Danh Sách Lỗi Phát Hiện (Findings)

### Finding Q-4-01 (High / Usability Blocker)
- **Thành phần:** `src-tauri/src/quick.rs` và `directory.rs`
- **Mô tả:** Trong bản release, hàm `scan_quick` quét thư mục hệ thống `C:\Windows\System32\drivers`. Trên các phiên bản Windows 10/11 tiêu chuẩn, thư mục con `DriverData` (`C:\Windows\System32\drivers\DriverData`) được bảo vệ bởi ACL đặc quyền của Windows (chỉ cấp quyền cho `NT AUTHORITY\SYSTEM`). Người dùng chạy ứng dụng không có quyền truy cập thư mục này (`os error 5: Access is denied`).
- **Hệ quả:** Mọi lần chạy `Quick Scan` mặc định trên một máy Windows bình thường đều quét 588 file sạch nhưng dính 1 lỗi tại `DriverData`, khiến Quick Scan **luôn luôn kết thúc với trạng thái `Scan incomplete` (`clean 588 · errors 1`) và không bao giờ có thể đạt được trạng thái `All Good`**.
- **Khuyến nghị khắc phục:** Bổ sung cơ chế bỏ qua hoặc xử lý riêng đối với các thư mục hệ thống có ACL bảo vệ đặc thù như `DriverData` trong phạm vi Quick Scan, hoặc cho phép quét đệ quy bỏ qua các thư mục không có quyền truy cập mà không đánh dấu toàn bộ đợt quét là incomplete nếu người dùng chỉ chạy Quick Scan.

---

## 5. Các Mục Cần Anh Zon Kiểm Thử Thủ Công (Needs Zon Manual Test)

Do hộp thoại file Windows 11 (`IFileOpenDialog`) sử dụng control `ComboBoxEx32` (AutomationId `1148`) không hỗ trợ `ValuePattern` và cơ chế PowerShell Script ExecutionPolicy Restricted của hệ thống không cho phép chạy script can thiệp cửa sổ native từ bên ngoài (tuân thủ nguyên tắc DEFENDER-SAFE), các mục phụ thuộc vào tương tác hộp thoại chọn file/thư mục cần anh Zon nhấp thử thủ công:

1. **Chọn File:** Mở menu "Select target" -> bấm "File" -> hộp thoại mở ra, chọn 1 file -> nút chính đổi từ `Quick Scan` thành `Scan`.
2. **Chọn Folder:** Mở menu "Select target" -> bấm "Folder" -> hộp thoại mở ra, chọn 1 thư mục -> nút chính đổi từ `Quick Scan` thành `Scan`.
3. **Auto-return sau khi quét File:** Sau khi quét file hoàn tất, đường dẫn target tự động xóa, nút chính quay lại `Quick Scan`, kết quả quét vẫn giữ nguyên trên màn hình cho tới lượt quét tiếp theo.
4. **Auto-return sau khi quét Folder:** Tương tự đối với lượt quét folder.
5. **Dòng "All Good" duy nhất cho file sạch:** Quét thủ công 1 file sạch bất kỳ để chiêm ngưỡng giao diện kết quả chuẩn `All Good` dạng 1 dòng duy nhất có icon dấu kiểm xanh lục (vì Quick Scan hiện tại luôn dính lỗi Q-4-01 tại `DriverData`).
6. **Bảng danh sách mối đe dọa (Threat List):** Đã được kiểm thử và xác minh tự động trong Task Q-5 (chi tiết tại Mục 6 bên dưới).

---

## 6. Q-5 Threat View Verification (Kiểm Thử Hiển Thị Danh Sách Mối Đe Dọa)

- **Thời gian thực hiện:** 2026-11-07
- **Worktree:** `E:\Zonendpoint-Qeen-0a55516` (commit `0a55516`)
- **Evidence:** `E:\Zonendpoint-tools\artifacts\0a55516\qeen\threat\`
- **Kịch bản thực hiện:** Proxy `qeen-proxy-threat.mjs` chạy tại `127.0.0.1:13399` nối tới lab clamd `127.0.0.1:13310`. Khi quét Quick Scan, proxy chặn và ghi đè kết quả phản hồi `INSTREAM` cho 3 file:
  - File thứ 30: `stream: Win.Test.ZonQA FOUND\0` (Tên mã độc chuẩn)
  - File thứ 60: `stream: Win.Test.<b>x</b> FOUND\0` (Kiểm tra chèn mã HTML / XSS probe)
  - File thứ 90: `stream: Win.Test.LongName.Trojan.Dropper.Generic.VeryLongSignatureStringWithNoSpacesToTestTextWrappingAndLayoutIntegrityUnderExtremeLengthConditions.Variant1234567890.End FOUND\0` (Kiểm tra độ dài cực hạn & bố cục layout)

### 6.1. Kết Quả Kiểm Tra Giao Diện & Bộ Đệm

1. **Hiển thị chỉ các dòng mối đe dọa (Threat-only view):**
   - Tiêu đề kết thúc quét: `"Scan incomplete"`, tuyệt đối không bao giờ hiển thị `"All Good"`.
   - Dòng thống kê: `Processed 589 items · clean 585 · threats 3 · unknown 0 · errors 1 · skipped 0 · cancelled 0`.
   - Cảnh báo: `Do not treat an incomplete scan as clean.`.
   - Danh sách `.results li`: Hiển thị đúng 4 dòng (3 dòng `Threat detected` + 1 dòng `Scan error` do `DriverData` [Q-4-01]). Toàn bộ 585 file `Clean` bị ẩn hoàn toàn, không xuất hiện bất kỳ dòng Clean nào trong danh sách.
   - Ảnh chụp giao diện thực tế: `E:\Zonendpoint-tools\artifacts\0a55516\qeen\threat\threat-list-view.png`.

2. **Chống chèn HTML & Render nguyên văn (HTML Sanitization):**
   - Tên mã độc `Win.Test.<b>x</b>` hiển thị nguyên văn dạng plain text `Win.Test.<b>x</b>`.
   - DOM runtime: Thẻ `<b>` hoàn toàn **không** tồn tại trong DOM (`$('.results small b')` = `false`).
   - Mã nguồn innerHTML của `<small>`: `Win.Test.&lt;b&gt;x&lt;/b&gt;` do React escape an toàn. Không có nguy cơ chèn mã HTML / XSS.

3. **Tính toàn vẹn cơ sở dữ liệu `cache.db`:**
   - Cơ sở dữ liệu SQLite tại `sandbox/appdata/Zonendpoint/cache.db` ghi nhận 588 bản ghi: `CLEAN = 585`, `INFECTED = 3`.
   - Kiểm tra đối chiếu mã băm SHA256: 100% các file bị nhiễm mã độc (`INFECTED`) tuyệt đối không có bản ghi nào bị ghi nhầm thành `CLEAN`.

4. **Kiểm tra vỡ bố cục khi tên mã độc cực dài (Finding Q-5-01):**
   - Phát hiện: Phần tử `.results small` trong `src/App.css` thiếu thuộc tính `overflow-wrap: anywhere;` (hoặc `word-break: break-word`), hiện đang nhận `overflow-wrap: normal`.
   - Hệ quả: Với tên chữ ký dài không có khoảng trắng, phần tử `<small>` bị phình rộng từ `clientWidth: 217px` lên `scrollWidth: 1006px`, khiến danh sách `.results` phình rộng từ `419px` lên `1208px` và làm xuất hiện thanh cuộn ngang (horizontal scrollbar) bên trong danh sách kết quả (dù khung `.app-shell` chính vẫn được neo cố định 800px nhờ `overflow: auto`).

