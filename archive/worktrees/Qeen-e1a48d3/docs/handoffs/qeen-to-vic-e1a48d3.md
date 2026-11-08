# Qeen to Vic Handoff — LAB-RESILIENCE Verification (e1a48d3)

- **Date:** 2026-11-07
- **From:** Qeen (Independent Tester & Security Reviewer)
- **To:** Vic (Lead Developer)
- **Target Snapshot:** Commit `e1a48d3` on `vic/lab-resilience-plan`
- **Verification Report:** `docs/qa/resilience-e1a48d3.md`

---

## Numbered Findings for Vic

### Finding 1 [Security / Architecture]: Race condition trong cơ chế `try_read` kiểm tra dữ liệu thừa sau NUL terminator (P0-01)
- **File:** `src-tauri/src/scanner.rs:262-267`
- **Chi tiết:**
  ```rust
  let response = read_nul_frame(&mut socket, MAX_RESPONSE_BYTES).await?;
  let mut extra = [0u8; 1];
  if let Ok(n) = socket.try_read(&mut extra)
      && n > 0
  {
      return Err("clamd sent unexpected extra data after response terminator".into());
  }
  ```
- **Vấn đề:**
  `socket.try_read` là lời gọi non-blocking kiểm tra trực tiếp buffer tầng socket tại đúng thời điểm vừa đọc xong ký tự NUL. Nếu phía clamd hoặc proxy trung gian gửi `stream: OK\0` và frame thứ hai (`stream: Threat FOUND\0`) bị trễ dù chỉ một vài mili-giây (hoặc tách qua TCP segment sau), `try_read` trả về `WouldBlock` (`n == 0`/error). Khối kiểm tra này sẽ bị vượt qua, và file bị nhận định sai là `Clean`, hiển thị `All Good` và lưu vào cache SQLite.
- **Hành động cần thiết:**
  Đối với giao thức phản hồi đơn (single response), sau khi nhận phản hồi kết thúc bằng NUL, cần đọc tới khi nhận được tín hiệu đóng socket (EOF / `read() == Ok(0)`) hoặc áp dụng timeout chờ đóng kết nối trước khi chấp nhận và lưu cache kết quả `Clean`.

---

### Finding 2 [Harness / Infrastructure]: Asynchronous connection race condition trong `scripts/resilience-proxy.mjs`
- **File:** `scripts/resilience-proxy.mjs:153`
- **Chi tiết:**
  Khi proxy phát hiện lệnh `zINSTREAM\0`, việc gọi `connectUpstream` diễn ra bất đồng bộ. Trong lúc chờ kết nối TCP tới upstream clamd hoàn tất, client socket đã bắt đầu đẩy tiếp các chunk dữ liệu của file. Nhánh `else` trong handler dữ liệu kiểm tra `if (upstreamSocket && !upstreamSocket.destroyed)` nhưng lúc này `upstreamSocket` chưa sẵn sàng, khiến các chunk dữ liệu ban đầu bị âm thầm đánh rơi (silently dropped). Khiến upstream clamd ngắt kết nối do không nhận đủ frame.
- **Hành động cần thiết:**
  Áp dụng cơ chế tạm dừng nhận dữ liệu từ client (`clientSocket.pause()`) và gom các chunk đến sớm vào bộ đệm `pendingChunks`, chỉ `clientSocket.resume()` sau khi socket upstream đã phát sinh sự kiện `'connect'`.

---

### Finding 3 [Automation / UI]: Hộp thoại chọn file Windows 11 không hỗ trợ `ValuePattern`
- **Chi tiết:**
  Điều khiển nhập đường dẫn trong `IFileOpenDialog` của Windows 11 là `ComboBoxEx32` (AutomationId 1148) không hỗ trợ `ValuePattern` của UIAutomation.
- **Hành động cần thiết:**
  Các bộ script test tự động trên Windows 11 cần gửi thông điệp `WM_SETTEXT` vào handle của cửa sổ `Edit` con bên trong ComboBoxEx32 và kích hoạt nút `Open` (AutomationId 1).
