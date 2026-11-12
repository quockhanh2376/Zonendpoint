# Handoff: Task V-30M (Merge close-fix into main) — Vic to Qeen

**Date**: 2026-11-12  
**From**: Victor (Vic)  
**To**: Qeen  
**Task**: V-30M (Merge `origin/vic/close-fix` into `main`)  
**Approval**: Zon retested "app close ok" and approved merge on 2026-11-12 (QA V-30 ref: `E:\Zonendpoint-tools\artifacts\v30-verify\qeen-verify.md`)  
**Branch**: `vic/close-fix-merge`  
**Base Commit (main trước)**: `22eb28f7c58dd073aaa98ea11bc58d31a6392f39` (`22eb28f`)  
**Merged Branch**: `origin/vic/close-fix` (tip `0b48da0`, code `e8b249d`)  
**Code Snapshot SHA**: `17bdccf`  
**Status**: READY FOR FAST-FORWARD TO MAIN (Pushed branch `vic/close-fix-merge`, DO NOT push to main directly — awaiting quick check by Qeen, then Loki fast-forwards main)  

---

## 1. Context & Parallel Work Isolation

- **Bối cảnh**: Nhánh `origin/vic/close-fix` (0b48da0, commit sửa `e8b249d`) được tách từ `0979f5f` trước các nhánh J-01 (adaptive-concurrency), MalwareBazaar, ThreatFox, và timer scheduler.
- **Công việc song song**: Task V-46 (`docs/design/guard.md`, nhánh `vic/guard-design`) và Qeen Q-34 (đo đạc J-03) đang chạy độc lập — không chạm vào các nhánh/file này.
- **Phê duyệt**: Anh Zon đã test lại trên bản cài, xác nhận "app close ok" và duyệt merge vào main ngày 2026-11-12.

---

## 2. Merge Conflict & Resolution

| File | Bản chất xung đột | Cách giải quyết |
|---|---|---|
| `docs/tasks/commit-notes.md` | Xung đột nội dung dòng cuối commit notes giữa nhánh `vic/close-fix` (`e8b249d`) và chuỗi commit từ V-33 đến V-44 trên `main`. | Giữ đầy đủ tất cả mục commit của cả hai nhánh theo thứ tự thời gian chính xác (đưa commit V-30 `e8b249d` vào sau commit V-29 `d93d183` ngày 2026-11-10). |
| `src-tauri/src/lib.rs` | Tự động gộp thành công (Auto-merging clean). | Không có xung đột git cú pháp. Đã rà soát kỹ lưỡng và tinh chỉnh tương thích theo mục 3 bên dưới. |

---

## 3. Code Review & Compatibility Adjustments (Task V-30M)

### A. Tương thích với Kiến trúc Mới trên main
1. **`ScanManager` & Quá trình Hủy Quét**:
   - Khi nhận sự kiện `WindowEvent::CloseRequested` hoặc `RunEvent::ExitRequested`, nếu có scan đang hoạt động (`active_scan_id().is_some()`), ứng dụng lập tức ẩn cửa sổ (`window.hide()`) để tạo trải nghiệm phản hồi tức thì cho người dùng, đồng thời gửi tín hiệu hủy (`manager.request_cancel(&active_id)`).
   - Vòng lặp chờ tối đa 5 giây (polling mỗi 50ms) để `ScanManager` giải phóng `active_scan_id()` khi scan task hoàn tất.
2. **Bounded Folder Workers & Adaptive Limiter (J-01)**:
   - Trong `directory.rs`, worker tasks được quản lý qua `tokio::task::JoinSet` (`scans`). Khi nhận tín hiệu hủy, tất cả worker in-flight được join đầy đủ (`scans.join_next().await`).
   - Kết quả phát hiện mối đe dọa (`ScanStatus::Infected`) được bảo vệ nguyên vẹn qua `record_result()` và `mark_user_cancelled()` (chỉ chuyển trạng thái từ `Clean` sang `Cancelled`, không bao giờ ghi đè `Infected`).
   - Trong `scanner.rs`, semaphore acquire kiểm tra `wait_cancelled()` qua `tokio::select!` giúp thoát ngay lập tức nếu đang xếp hàng chờ slot.
3. **Guard (`watcher::Guard` / `set_guard`)**:
   - Trước khi gọi `cleanup_before_exit()` và `std::process::exit()`, ứng dụng gọi hàm helper `stop_guard(&app)` để dừng `RecommendedWatcher` một cách an toàn, đóng các file watch handle.

### B. Refactor Event Handling (Khắc phục phản ánh Q-21 của Qeen)
- Theo mục 5.1 trong báo cáo `E:\Zonendpoint-tools\artifacts\v30-verify\qeen-verify.md`, code hủy scan và thoát trong `WindowEvent::CloseRequested` và `RunEvent::ExitRequested` bị trùng lặp nguyên văn.
- **Giải pháp**: Gom toàn bộ logic vào hàm dùng chung:
  - `stop_guard(app: &tauri::AppHandle)`: dừng Guard watcher an toàn nếu đang chạy.
  - `cancel_and_exit(app: tauri::AppHandle, exit_code: i32)`: hủy scan, đợi tối đa 5s cho worker giải phóng, gọi `stop_guard`, `cleanup_before_exit()`, và thoát tiến trình với mã `exit_code`.
- Khử trùng lặp hoàn toàn, code gọn gàng, tuân thủ nghiêm ngặt quy chuẩn Rust clippy (`-D warnings`).

---

## 4. Quality Gates Verification

| Kiểm tra | Lệnh | Kết quả | Chi tiết |
|---|---|---|---|
| **Rust Format** | `cargo fmt -- --check` | **PASS** | 0 diffs |
| **Rust Clippy** | `cargo clippy --all-targets -- -D warnings` | **PASS** | 0 warnings |
| **Rust Unit & Resilience** | `cargo test` | **PASS** | **68/68 PASS** (49 lib tests + 1 main bin test + 18 resilience tests) |
| **Frontend Production Build** | `npm run build` | **PASS** | Vite v8.3.5, 23 modules transformed, 0 lỗi TS/CSS |
| **Native Mock Test Suite** | `node tests/native/run.mjs` | **PASS** | **20/20 PASS** (Bao gồm test đóng app lúc idle và lúc đang quét của V-30) |

---

## 5. Next Steps

1. Qeen kiểm tra nhanh nhánh `vic/close-fix-merge` (Code Snapshot SHA `17bdccf`).
2. Loki thực hiện fast-forward nhánh `main` lên `vic/close-fix-merge` và push lên `origin/main`.
3. Nhánh `origin/main` sẽ chính thức sở hữu bản sửa đóng app hoàn chỉnh, sẵn sàng cho các bước release tiếp theo.
