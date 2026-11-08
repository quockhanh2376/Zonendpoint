# QA Report: UI Compact Layout Verification (Task Q-7)

- **Reviewer:** Qeen (Independent Tester & Security Reviewer)
- **Commit Under Test:** `321d32b2024a0676937cf1cd6946c6189674ed64` (`321d32b`)
- **Base Commit:** `ed1088c` (`origin/main`, fast-forwarded from `2db42a8`)
- **Worktree:** `E:\Zonendpoint-Qeen-321d32b` (detached HEAD)
- **Date:** 2026-11-08
- **Lab Environment:** Real ClamAV daemon via SSH tunnel `127.0.0.1:13310` (`ClamAV 1.5.4/28176`)
- **Evidence Directory:** `E:\Zonendpoint-tools\artifacts\321d32b\qeen\`
- **Reference Handoff:** `E:\Zonendpoint-vic-compact\docs\handoffs\ui-compact-vic-to-qeen.md`

---

## 1. Executive Summary

| Quality Gate | Status | Details |
|---|:---:|---|
| **Artifact Checksums & Provenance** | **PASS** | Tất cả 6 file trong `E:\Zonendpoint-tools\artifacts\321d32b\` khớp SHA256 với handoff của Vic. Provenance payload NSIS đạt chuẩn (`UNK` → `NSS`). |
| **Check A: Static & Mock Suite** | **PASS** | `cargo test` (51/51 pass), `cargo fmt` (clean), `cargo clippy` (0 warnings), `npm run build` (pass), Native Mock Suite (17/17 checks pass). |
| **Check B: Compact Layout & Wrapping** | **PASS** | Chấm trạng thái nằm cạnh logo, theme toggle nằm trong thẻ quét (top-right trên hàng tiêu đề `h1`), không còn nút trôi nổi ở góc cửa sổ. Không tràn/cuộn ở kích thước mặc định 800×240. Kết quả quét mở rộng sạch sẽ, tên mã độc dài 150+ ký tự tự động ngắt dòng không sinh thanh cuộn ngang cả Light và Dark. |
| **Check C: Status Dot Lifecycle** | **PASS** | Khởi động xanh (connected), khi ngắt tunnel chuyển đỏ kèm tooltip `"Cannot connect to ZonEndpoint server. Check your connection."`, click re-check khi tunnel mở lại chuyển xanh `"Connected to ZonEndpoint server"`. |
| **Check D: Theme Persistence** | **PASS** | Chuyển đổi Light ↔ Dark hoạt động mượt mà. Trạng thái Dark theme và logo dark được lưu trữ bền vững qua các lần khởi động lại ứng dụng. |
| **Check E: Accessibility & Focus** | **PASS** | Hit targets 44×44px cho status dot và theme toggle. Thứ tự phím Tab tự nhiên: `server-status` → `theme-toggle` → `Select target ▾` → nút quét chính. Viền focus 3px nổi bật. |
| **Check F: Regression & Real Smoke** | **PASS** | Quick Scan với ClamAV lab thật trả về dòng đơn `All Good` kèm icon SVG checkmark. Khi tunnel ngắt, quét báo lỗi tiền kiểm `"Cannot connect to ZonEndpoint server"` và tuyệt đối không báo All Good. DOM hoàn toàn sạch: không lộ "ClamAV", IP hay port. |
| **Check G: Cleanup & Tunnel** | **PASS** | Gỡ cài đặt sandbox sạch sẽ (xoá exe, registry HKCU và toàn bộ shortcut). Lab tunnel 13310 được duy trì liên tục và lắng nghe bình thường. |

---

## 2. Artifacts & SHA256 Checksums

Tất cả checksums được kiểm chứng độc lập tại thư mục `E:\Zonendpoint-tools\artifacts\321d32b\`:

| Artifact | Kích thước (bytes) | SHA256 Checksum | Đối chiếu Handoff |
|---|---|---|:---:|
| `zonendpoint.exe` | 8,065,024 | `DEA2944DBF1C6203927D5C3328D1BA5D125E259D19D545F3C60D349C4AA22AE2` | MATCH |
| `Zonendpoint_0.1.0_x64-setup.exe` | 3,689,916 | `A29C1ABB19F97E666CA674492029D854A987A3A1C829EDB84FDF42ED6D4092CF` | MATCH |
| `idle-light.png` | 34,492 | `F3098358A758E225ECFF35B7E0E39CB7C7393E50F34882884309E9802CF3EB54` | MATCH |
| `idle-dark.png` | 35,012 | `5798E0C689D0329C90183E7F52011C1A2FCF24902266C4CE1F4E6739210F6CEF` | MATCH |
| `all-good-light.png` | 34,829 | `98AC85E4D7FF676232553A5A8C10CB01AF36B76B2902E72F2E3820EDAF3FCEFE` | MATCH |
| `all-good-dark.png` | 35,467 | `DAB70F93773ECEF7017655A1C34D13B35CBD9939C0D618FA017720C1EB0E0322` | MATCH |

Kiểm tra payload provenance (`check-nsis-payload.mjs`): Khớp 100% với bản build ngoại trừ đúng 3 byte marker `UNK` → `NSS` tại offset 6169799 do bộ đóng gói NSIS của Tauri sinh ra.

---

## 3. Chi Tiết Thực Thi & Bằng Chứng Trực Quan (Visible Run)

Kiểm thử chạy trực quan ở tiền cảnh desktop với các khoảng dừng ~3 giây để quan sát:

### Check B: Layout thu gọn & Ngắt dòng tên mối đe dọa dài
- **Status dot**: Nằm bên cạnh logo thương hiệu trong cột trái (`.brand-identity`), căn giữa theo chiều dọc với logo.
- **Theme toggle**: Nằm gọn trong thẻ quét (`.selection-panel`), góc trên bên phải trên cùng hàng với tiêu đề `<h1>Scan files and folders</h1>`.
- **Top-right cửa sổ**: Hoàn toàn sạch, không còn container `.header-controls` hay nút bấm lơ lửng.
- **Kích thước mặc định 800×240**: `scrollHeight <= clientHeight` và `scrollWidth <= clientWidth` cả Light và Dark. Không xuất hiện thanh cuộn.
- **Threat Wrapping (150+ chars)**: Chạy quét với chữ ký giả lập 155 ký tự:
  `Win.Test.LongName.Trojan.Dropper.Generic.VeryLongSignatureStringWithNoSpacesToTestTextWrappingAndLayoutIntegrityUnderExtremeLengthConditions.Variant1234567890.End`
  Kết quả hiển thị trong danh sách mở rộng, áp dụng `overflow-wrap: anywhere;`. Xác nhận `scrollWidth <= clientWidth` (không sinh thanh cuộn ngang) trong cả Light theme (`11-threat-wrap-light.png`) và Dark theme (`12-threat-wrap-dark.png`).

### Check C: Hoạt động của chấm trạng thái máy chủ
- Khi kết nối tunnel `127.0.0.1:13310` hoạt động: chấm xanh (`data-status="connected"`), tooltip `"Connected to ZonEndpoint server"`.
- Khi dừng tunnel: click chấm trạng thái chuyển sang đỏ (`data-status="disconnected"`), tooltip `"Cannot connect to ZonEndpoint server. Check your connection."`.
- Khi khôi phục tunnel: click chấm trạng thái lập tức kiểm tra lại và chuyển về xanh (`data-status="connected"`).

### Check D: Lưu giữ Theme qua các lần khởi động lại
- Chuyển sang Dark mode (`data-theme="dark"`), logo đổi sang `logo-dark-slogan-transparent.png`.
- Đóng phiên ứng dụng và khởi động lại phiên native mới với cùng thư mục dữ liệu người dùng: ứng dụng mở lên giữ nguyên Dark theme và logo dark (`09-relaunch-persisted-dark.png`).
- Bấm toggle chuyển lại Light theme thành công (`10-relaunch-back-to-light.png`).

### Check E: Khả năng tiếp cận & Phím điều hướng (Accessibility)
- Kích thước vùng bấm (Hit targets):
  - `.server-status`: 44×44 px.
  - `.theme-toggle`: 44×44 px.
  - `button.primary`: 93.6×36 px (chiều cao 36px > mức tối thiểu 24px theo WCAG 2.5.8 Level AA).
- Thứ tự điều hướng phím Tab:
  1. `server-status` (chấm trạng thái)
  2. `theme-toggle` (nút đổi giao diện sáng/tối)
  3. `Select target ▾` (menu chọn mục tiêu)
  4. `Quick Scan` / `Scan` (nút quét chính)
- Viền focus ring: `outline: 3px solid var(--focus-ring); outline-offset: 3px;` (`02-focus-ring.png`).

### Check F: Hồi quy & Quét thực nghiệm với ClamAV lab
- Chạy `Quick Scan` trên bản cài đặt sandbox kết nối với lab ClamAV thật qua tunnel `127.0.0.1:13310`: Quét hoàn tất với tiêu đề `All Good`, dòng đơn `.all-good-line` và icon SVG `.all-good-check` màu xanh lục (`07-all-good-light.png`, `08-all-good-dark.png`).
- Thử nghiệm ngắt tunnel rồi nhấn quét: Hiển thị thông báo lỗi tiền kiểm `.notice.error`: `"Cannot connect to ZonEndpoint server"` (`05-offline-preflight-notice.png`), tuyệt đối không hiển thị All Good.
- Kiểm tra chuỗi giao diện (String hygiene): Không tìm thấy từ khóa "ClamAV", không lộ địa chỉ IP nội bộ (`127.0.0.1`, `100.96.*`), không lộ port (`13310`, `3310`) trên DOM.

### Check G: Dọn dẹp môi trường (Cleanup)
- Chạy trình gỡ cài đặt NSIS: `E:\Zonendpoint-tools\installer-sandbox\app\uninstall.exe /S`.
- Xác nhận tập tin `zonendpoint.exe` đã được xoá.
- Xác nhận khoá đăng ký gỡ cài đặt trong `HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*` đã được xoá sạch.
- Xác nhận các shortcut tại Desktop và Start Menu đã được xoá hoàn toàn.
- Xác minh lab tunnel `127.0.0.1:13310` vẫn đang hoạt động và lắng nghe bình thường.

---

## 4. Danh Mục Ảnh Bằng Chứng (Evidence Files)

Thư mục lưu trữ: `E:\Zonendpoint-tools\artifacts\321d32b\qeen\`

1. `01-idle-light.png`: Trạng thái chờ ban đầu ở Light theme.
2. `02-focus-ring.png`: Vòng viền focus khi điều hướng phím Tab.
3. `03-idle-dark.png`: Giao diện thu gọn ở Dark theme.
4. `04-status-disconnected.png`: Chấm trạng thái màu đỏ khi mất kết nối tunnel.
5. `05-offline-preflight-notice.png`: Thông báo lỗi tiền kiểm khi quét trong trạng thái offline.
6. `06-status-reconnected.png`: Chấm trạng thái phục hồi màu xanh sau khi khởi động lại tunnel.
7. `07-all-good-light.png`: Kết quả Quick Scan thành công All Good ở Light theme.
8. `08-all-good-dark.png`: Kết quả All Good khi chuyển sang Dark theme.
9. `09-relaunch-persisted-dark.png`: Khởi động lại ứng dụng duy trì giao diện Dark theme.
10. `10-relaunch-back-to-light.png`: Chuyển đổi lại về Light theme sau khi khởi động lại.
11. `11-threat-wrap-light.png`: Ngắt dòng chữ ký mã độc dài 150+ ký tự ở Light theme (không cuộn ngang).
12. `12-threat-wrap-dark.png`: Ngắt dòng chữ ký mã độc dài 150+ ký tự ở Dark theme (không cuộn ngang).

---

## 5. Kết Luận & Đánh Giá

- **Đánh giá tổng thể:** **PASS**. Bản cập nhật layout thu gọn tại commit `321d32b` đáp ứng trọn vẹn yêu cầu thiết kế của Zon và tuân thủ các ràng buộc về an toàn, khả năng tiếp cận và độ ổn định của v1.0.0.
- **Không phát hiện lỗi sản phẩm (No product bugs).** Không cần phát hành `qeen-to-vic-321d32b.md`.
- **Ghi chú nhỏ về tài liệu bàn giao của Vic:** Trong `docs/handoffs/ui-compact-vic-to-qeen.md`, Vic ghi SHA đầy đủ là `321d32b8fa58890aa974bfa58882ca6c934f07a7` trong khi commit thực tế trên git là `321d32b2024a0676937cf1cd6946c6189674ed64` (chung 7 ký tự đầu `321d32b`). Điểm này không ảnh hưởng đến mã nguồn hay kết quả kiểm thử.
- **Môi trường:** Không đẩy git (`no push`), không hợp nhất main (`no merge`). Tunnel 13310 duy trì hoạt động.
