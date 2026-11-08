# Qeen QA Report: Task Q-10 — Smoke Test Integrated Snapshot 69d8577 (UI-Compact + Cache-Reason)

**Date**: 2026-11-08  
**QA Agent**: Qeen  
**Code Snapshot SHA**: `69d8577e0ec7dbc54fd638f0fb1a328d2fe6c470` (`69d8577`)  
**Worktree**: `E:\Zonendpoint-Qeen-69d8577` (detached at `69d8577`)  
**Artifacts Directory**: `E:\Zonendpoint-tools\artifacts\69d8577\`  
**Evidence Directory**: `E:\Zonendpoint-tools\artifacts\69d8577\qeen\`  
**Verdict**: **PASS** (Tất cả 5 tiêu chí đều đạt chuẩn; 0 lỗi/defect; sẵn sàng chuyển giao cho anh Zon và Loki fast-forward lên `main`)

---

## 1. Artifact Verification & Build Integrity

| Artifact | File Size | SHA256 Checksum | Result |
|---|---|---|---|
| `zonendpoint.exe` | 8,067,584 bytes | `8FC2193767F747E66B71214667DCCEA0E773A2629410E0D7D059EB82B83D913C` | **PASS** (Khớp chính xác với handoff) |
| `Zonendpoint_0.1.0_x64-setup.exe` | 3,691,233 bytes | `9B4E8D693E9D47B59C928B2124E0F01A9C17B9A5A74E28515D0A8CC6C052C728` | **PASS** (Khớp chính xác với handoff) |

- **Unit & Integration Tests**: `cargo test --manifest-path src-tauri/Cargo.toml`
  - 39/39 lib unit tests PASS
  - 1/1 main unit tests PASS
  - 18/18 resilience integration tests PASS
  - **Total**: 58 passed; 0 failed; 0 ignored; finished in ~6.6s.
- **Code Formatting**: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` → **PASS** (0 diffs).
- **Linter**: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` → **PASS** (0 warnings).
- **Frontend Build**: `cmd /c "npm run build"` (`tsc && vite build`) → **PASS** (0 errors; built in 406ms).

---

## 2. Layout gọn: Chấm cạnh Logo, Toggle trong Thẻ & Light/Dark

- **Server Status Dot (`.server-status`)**:
  - Nằm ở cột bên trái, cạnh `.brand-logo` và nằm bên trái của scan card (`.selection-panel`).
  - Căn chỉnh theo trục dọc với logo (vertical alignment).
  - Vùng bấm (hit target) đạt chuẩn khả dụng: `width >= 44px`, `height >= 44px`.
  - Hiển thị đúng trạng thái kết nối `data-status="connected"` (chấm xanh lá).
- **Theme Toggle (`.theme-toggle`)**:
  - Nằm gọn bên trong thẻ quét (`.selection-panel`), ở hàng tiêu đề cùng cấp với thẻ `<h1>`.
  - Nằm ở góc trên bên phải của thẻ quét (`t.right > h.right`).
  - Vùng bấm đạt chuẩn khả dụng: `width >= 44px`, `height >= 44px`.
- **Top Bar**: Không còn nút trôi nổi ở góc trên bên phải (`.header-controls` đã được loại bỏ hoàn toàn).
- **Light & Dark Mode**:
  - Giao diện khởi động mặc định ở chế độ Light (`document.documentElement.dataset.theme === 'light'`).
  - Nhấp vào theme toggle: chuyển sang Dark (`data-theme="dark"`), toàn bộ bố cục vẫn giữ nguyên độ chuẩn xác.
  - Nhấp lại: quay về Light (`data-theme="light"`).
  - Ảnh chụp bằng chứng: `step1-layout-light.png` (Light mode), `step2-layout-dark.png` (Dark mode).

---

## 3. Cache Hit giữ tên Threat (Fix P1-01) trên Layout Mới

- **Lần quét 1 (Live Detection & DB Migration)**:
  - Khởi tạo sandbox với SQLite schema v1 (`PRAGMA user_version = 1`, 593 dòng Clean và 1 dòng Infected chưa có tên threat).
  - Proxy loopback ở chế độ `threat-inject` chèn signature giả định `Win.Test.CustomThreat.P101`.
  - Quét Quick Scan lần 1 (hoàn thành trong 3.0s):
    - Dòng Infected cũ không có tên threat được xử lý an toàn như cache miss và quét lại.
    - Giao diện hiển thị: `Scan complete`, `1 threat detected`.
    - Chi tiết dòng: Status `Threat detected`, Reason `Win.Test.CustomThreat.P101`.
    - SQLite `cache.db` tự động nâng cấp giao dịch sang `user_version = 2`, bổ sung cột `threat_name TEXT` và lưu giá trị `Win.Test.CustomThreat.P101`.
  - Ảnh chụp: `step3-first-scan-threat.png`.
- **Lần quét 2 (Cache Hit Verification)**:
  - Chuyển proxy sang `passthrough` (không inject mã độc; nếu quét thật sẽ trả về CLEAN).
  - Bấm Quick Scan lần 2 (hoàn thành siêu tốc 3.0s nhờ cache hit):
    - Giao diện hiển thị: `Scan complete`, `1 threat detected`.
    - Chi tiết dòng: Status `Threat detected`, Reason giữ nguyên chuẩn xác `Win.Test.CustomThreat.P101`.
    - **Khẳng định cốt lõi**: Tên threat được giữ nguyên 100%, **tuyệt đối không bị ghi đè** bởi chuỗi `"verified content cache hit"`.
  - Ảnh chụp: `step4-cache-hit-preserves-threat.png`.

---

## 4. Quick Scan All Good & Tunnel Off (Bảo vệ bất biến)

- **Quick Scan All Good**:
  - Loại bỏ dòng Infected khỏi database sandbox, giữ lại toàn bộ cache sạch.
  - Chạy Quick Scan trực tiếp trên SSH tunnel lab 13310:
    - Hoàn tất thành công với màn hình xanh `All Good`.
    - Hiển thị `.all-good-card`, icon `.all-good-check` và tiêu đề `All Good`.
    - Nút chính hiển thị `Quick Scan` sẵn sàng cho lượt quét tiếp theo.
  - Ảnh chụp: `step6-allgood.png`.
- **Tunnel Off (Mất kết nối Daemon)**:
  - Giả lập ngắt kết nối (proxy chuyển sang `drop-connect`).
  - Chấm trạng thái server chuyển sang đỏ/ngắt kết nối (`data-status="disconnected"`).
  - Banner thông báo lỗi xuất hiện: `"Cannot connect to ZonEndpoint server"`.
  - Bấm Quick Scan: preflight chặn ngay lập tức, **tuyệt đối không bao giờ trả về CLEAN** khi server ngoại tuyến.
  - Ảnh chụp: `step5-tunnel-off-notice.png`.

---

## 5. Dọn dẹp Sandbox & Bảo toàn SSH Tunnel

- **Gỡ bỏ sandbox**:
  - Đã thực thi uninstaller `uninstall.exe /S` của ứng dụng trong sandbox.
  - Xóa bỏ hoàn toàn thư mục tạm sandbox `E:\Zonendpoint-tools\artifacts\69d8577\qeen\sandbox\`.
- **Bảo toàn SSH Tunnel Lab 13310**:
  - SSH tunnel loopback `127.0.0.1:13310` được kiểm tra sau khi kết thúc bộ test: **STILL LISTENING** và phản hồi `PONG` chuẩn xác.
  - Không có bất kỳ malware/EICAR thực tế nào được tạo trên ổ đĩa trong suốt quá trình test.

---

## 6. Danh mục Bằng chứng (Evidence Files)

Thư mục lưu trữ bằng chứng: `E:\Zonendpoint-tools\artifacts\69d8577\qeen\`
- `step1-layout-light.png`: Giao diện Light, chấm cạnh logo, toggle trong thẻ.
- `step2-layout-dark.png`: Giao diện Dark, căn chỉnh chuẩn xác.
- `step3-first-scan-threat.png`: Lần quét 1 phát hiện `Win.Test.CustomThreat.P101`.
- `step4-cache-hit-preserves-threat.png`: Lần quét 2 cache hit bảo toàn tên threat trên layout mới.
- `step5-tunnel-off-notice.png`: Tunnel ngắt kết nối, chấm đỏ, banner thông báo lỗi, không CLEAN.
- `step6-allgood.png`: Màn hình All Good sạch sẽ sau khi quét clean.
- `qeen-report-69d8577.json`: Báo cáo dữ liệu chi tiết máy đọc được.
- `qeen-verify-69d8577.mjs`: Script tự động hóa kiểm thử bằng WebDriverIO.
- `qeen-proxy.mjs`: Loopback proxy harness kiểm soát kịch bản mạng an toàn.
