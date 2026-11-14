# Handoff: Task V-57 (Zonendpoint 0.3.1 Client Optimizations & Security) — Vic to Qeen

- **Date**: 2026-11-14
- **Author**: Victor (Vic)
- **Branch**: `vic/031-client`
- **Base Commit**: `908535e` (`origin/main`, tag `v0.3.0`)
- **Version**: `0.3.1`
- **Artifacts Directory**: `E:\Zonendpoint-tools\artifacts\v57-release\`

---

## 1. Mục tiêu và Phạm vi (Task V-57)

Thực hiện toàn bộ các hạng mục ưu tiên phía client theo tài liệu kiến trúc tối ưu `E:\Zonendpoint\optimisse.md` do anh Zon giao ngày 2026-11-14, nâng cấp ứng dụng lên phiên bản `0.3.1`.

Các hạng mục loại trừ đã được tuân thủ nghiêm ngặt (không làm trong V-57):
- Không đụng `scripts/feeds/` (thuộc task song song V-58).
- Không làm P-2, P-4, X-1, S-8 (dành cho các task tiếp theo).
- Giữ nguyên các cam kết: không tạo git tag, không ký số, không publish release, không push/merge vào main, không cài đè lên máy của anh Zon.

---

## 2. Chi tiết 8 hạng mục triển khai

### Mục 1 (S-1): Giới hạn lưu trữ ScanSummary ưu tiên mối đe dọa INFECTED
- **Commit**: `b72d7e6` `fix(scan): retain only non-clean scan results prioritizing infected items (S-1)`
- **File**: `src-tauri/src/directory.rs`, `src-tauri/src/lib.rs`
- **Cơ chế**:
  - `ScanSummary.results` chỉ lưu trữ chi tiết các file không-CLEAN (`Infected`, `Error`, `Skipped`, `Unknown`, `Cancelled`), tối đa trần `MAX_RETAINED_RESULTS = 1000`.
  - Toàn bộ kết quả `INFECTED` luôn được giữ lại. Nếu danh sách đầy (đạt 1000), thuật toán sẽ chủ động đẩy bớt (evict) các mục không phải `INFECTED` (chẳng hạn file `Error` hoặc `Skipped`) để nhường chỗ cho `INFECTED`. Nếu đầy 1000 mối đe dọa `INFECTED`, các mục `INFECTED` mới vẫn tiếp tục được lưu.
  - Các bộ đếm tổng hợp (`total_scanned`, `clean`, `infected`, `errors`, `skipped`, `unknown`, `cancelled_files`) vẫn tăng chính xác 100%.
  - Unit tests bổ sung: `clean_files_do_not_crowd_out_subsequent_infected_result`, `infected_result_evicts_non_infected_when_retained_is_full`.

### Mục 2 (S-7 + S-6): Tự động phục hồi cache hỏng và dọn dẹp bản ghi quá hạn TTL
- **Commit**: `0e2bd9b` `fix(cache): treat cache read failure as miss and auto-recover corrupt db with ttl cleanup (S-7, S-6)`
- **File**: `src-tauri/src/cache.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/scanner.rs`
- **Cơ chế**:
  - **S-7 (Cache Read Failure = Miss)**: Khi mở/đọc cache gặp lỗi I/O hoặc truy vấn, hàm `scanner::scan_file_with_engine_override` bắt lỗi, coi như cache miss và tiếp tục quét trực tiếp qua daemon bình thường, ghi rõ lý do vào `ScanResult.reason` (ví dụ `cache read failed: ...`) mà không biến kết quả thành `ERROR`.
  - **S-6 (Corrupt DB Auto-Recovery)**: Trong `cache::open()`, nếu SQLite gặp lỗi hỏng (corrupted database, `DatabaseCorrupt`, `NotADatabase`, hoặc sai schema migration), hàm tự động đổi tên tệp hỏng thành `cache.db.corrupt-YYYYMMDD-HHMMSS` (sử dụng chuẩn giờ UTC+7 an toàn cho tên file Windows) và khởi tạo một database cache mới tinh.
  - **S-6 (TTL Cleanup lúc mở app)**: Bổ sung hàm `cache::cleanup_expired(path)` xóa các bản ghi có `created_at < now - 7 ngày` (`SCAN_CACHE_TTL_DAYS`), được tự động gọi trong `.setup()` khi ứng dụng khởi động.
  - Unit tests bổ sung: `corrupt_cache_db_auto_recovers`, `incompatible_schema_version_auto_recovers`, `cleanup_expired_removes_old_entries`, `cache_read_failure_treated_as_miss_and_scans_normally`.

### Mục 3 (X-5): Khóa cache gắn liền với thế hệ chữ ký máy chủ
- **Commit**: `fde9a5b` `feat(cache): bind cache validity to server signature generation (X-5)`
- **File**: `src-tauri/src/server_status.rs`, `src-tauri/src/cache.rs`, `src-tauri/src/scanner.rs`, `src-tauri/tests/resilience.rs`
- **Cơ chế**:
  - `server_status.rs` cung cấp phương thức `ServerStatusDto::signature_generation(&self) -> Option<String>`, trả về chuỗi định danh thế hệ chữ ký dạng `"{signatures}:{loaded_at}"` (ví dụ `"8616464:2026-11-14T11:41:49+07:00"`).
  - Nâng cấp schema cache SQLite lên **v3** với trường `sig_generation TEXT NOT NULL` trong bảng `scan_cache`. Dữ liệu cache cũ từ v2 tự động bị hủy và xóa sạch (drop table) để tránh dùng lại kết quả của thế hệ chữ ký cũ.
  - Nâng cấp `POLICY_VERSION` thành `zonendpoint-0.3.1-v1`.
  - Khi `server-status.json` không tồn tại, bị stale (>2 giờ), hoặc không đọc được chữ ký hợp lệ: client **hoàn toàn vô hiệu hóa cache** (không đọc cache, không ghi cache mới, quét trực tiếp 100% qua daemon).
  - Khi lab server reload DB chữ ký mới (`signatures` hoặc `loaded_at` thay đổi): các truy vấn cache cũ sẽ tự động miss và quét lại.
  - Unit tests bổ sung: `signature_generation_change_invalidates_cache_entry`, `invalid_or_missing_server_status_completely_disables_cache`, `valid_server_status_caches_and_sig_generation_change_bypasses_cache`.

### Mục 4 (X-7 + S-4): Thu gọn bề mặt tấn công ứng dụng (Security Surface Pruning)
- **Commit**: `aebb5d4` `chore(security): prune unused opener plugin and uninvoked guard commands (X-7, S-4)`
- **File**: `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/capabilities/default.json`, `src-tauri/src/lib.rs`
- **Cơ chế**:
  - Gỡ bỏ plugin `tauri-plugin-opener` và `@tauri-apps/plugin-opener` (gỡ khỏi dependencies, capabilities JSON và `.plugin()` registration).
  - Loại bỏ việc đăng ký hai lệnh `set_guard` và `cache_stats` khỏi `tauri::generate_handler![]` (tránh lộ attack surface khi UI chưa dùng).
  - Giữ nguyên mã nguồn `watcher.rs` và `Guard` nội bộ cho kế hoạch Guard v1.1 sau này.

### Mục 5 (P-1): Tối ưu hóa Cache Hit bằng siêu dữ liệu
- **Commit**: `5039343` `perf(scanner): replace second full hash with lightweight metadata verification on cache hit (P-1)`
- **File**: `src-tauri/src/scanner.rs`
- **Cơ chế**:
  - Trong logic quét file trước đây: khi cache hit, client thực hiện đọc toàn bộ nội dung file và tính SHA-256 lần thứ hai để chống TOCTOU.
  - Cải tiến P-1: Lưu lại snapshot `(size, mtime)` ngay tại thời điểm mở file băm SHA-256 lần 1. Khi cache hit, client chỉ cần đọc lại siêu dữ liệu (`symlink_metadata`) và so khớp `size` + `mtime`.
  - Nếu `size` hoặc `mtime` thay đổi trong lúc quét: trả về `ScanStatus::Unknown` ("file modified during verification").
  - Loại bỏ hoàn toàn lượt đọc đĩa và tính hash SHA-256 lần 2 trên toàn bộ các file sạch có sẵn trong cache, giúp tăng tốc vượt trội.
  - Unit tests bổ sung: `cache_hit_detects_metadata_mtime_or_size_change`.

### Mục 6 (S-5): Ghi cấu hình Server an toàn nguyên tử và cảnh báo lỗi tệp
- **Commit**: `1124ab8` `fix(config): ensure atomic configuration writes and surface corruption errors (S-5)`
- **File**: `src-tauri/src/config.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/scanner.rs`, `src/App.tsx`
- **Cơ chế**:
  - Hàm `save_server_address` ghi cấu hình ra file tạm cùng thư mục `config.json.tmp.<uuid>`, gọi `sync_all()` để flush đĩa cứng, sau đó đổi tên nguyên tử (`rename`) sang `config.json`.
  - Hàm `get_server_config` trả về `Result<ServerConfigDto, String>`. Nếu tệp `config.json` bị hỏng cấu trúc JSON, hàm trả về lỗi chi tiết `Err(...)` thay vì âm thầm rơi về cấu hình mặc định.
  - Giao diện `App.tsx` bắt lỗi cấu hình hỏng, hiển thị thông báo lỗi màu đỏ `Config error: ...` và tự động mở hàng Server để người dùng kịp thời nhận biết và cấu hình lại.
  - Đồng thời bổ sung biến khóa `TEST_ENV_LOCK` trong `lib.rs` bảo vệ đa luồng cho các test can thiệp vào biến môi trường hệ thống.
  - Unit tests bổ sung: `corrupted_config_file_returns_explicit_error_and_does_not_silently_fallback`, `save_server_address_writes_atomically_and_recovers_address`.

### Mục 7 (P-5): Điều tiết sự kiện tiến trình quét UI
- **Commit**: `73e672d` `perf(events): throttle clean progress events to 10Hz while emitting non-clean immediately (P-5)`
- **File**: `src-tauri/src/lib.rs`
- **Cơ chế**:
  - Xây dựng cấu trúc `ProgressThrottler` bọc ngoài `ProgressFn`.
  - Với các file `Clean`: điều tiết phát sự kiện `scan-progress` tối đa ~10 lần/giây (khoảng cách tối thiểu 100ms giữa 2 lần phát), lưu trữ kết quả mới nhất vào bộ đệm pending.
  - Với các kết quả không-CLEAN (`Infected`, `Error`, `Skipped`, `Unknown`, `Cancelled`): lập tức phát sự kiện lên UI ngay lập tức không bị throttle.
  - Khi tiến trình quét thư mục hoàn tất: tự động gọi `throttler.flush()` để phát sự kiện cuối cùng chứa đầy đủ tổng số file và thời gian quét cho UI.
  - Unit tests bổ sung: `throttles_rapid_clean_progress_and_flushes_tail`, `emits_non_clean_progress_immediately_even_within_throttle_window`.

### Mục 8 (X-6): Huy hiệu ghi đè môi trường "Env active" cố định
- **Commit**: `0595ac9` `feat(ui): display environment override badge alongside server status icon (X-6)`
- **File**: `src/App.tsx`, `src/App.css`
- **Cơ chế**:
  - Di chuyển huy hiệu "Env active" ra cạnh biểu tượng trạng thái kết nối server ở thanh tiêu đề bên trái (`.server-status-row`).
  - Huy hiệu luôn luôn hiển thị khi có biến môi trường `ZONENDPOINT_CLAMD` được kích hoạt, hoàn toàn độc lập với việc hàng Server đang đóng hay mở (từ V-54 hàng Server tự ẩn khi kết nối OK).
  - Tối ưu CSS với viền `1px solid var(--button-border)`, căn chỉnh chuẩn xác trên cả 2 theme Sáng và Tối, không bị tràn hay vỡ giao diện trong kích thước compact 800×240px.

---

## 3. Nâng cấp phiên bản 0.3.1

Phiên bản đã được nâng cấp đồng bộ từ `0.3.0` lên `0.3.1` tại 5 vị trí:
1. `package.json`: `"version": "0.3.1"`
2. `package-lock.json`: `"version": "0.3.1"` (chỉ mục gói `zonendpoint`)
3. `src-tauri/tauri.conf.json`: `"version": "0.3.1"`
4. `src-tauri/Cargo.toml`: `version = "0.3.1"`
5. `src-tauri/Cargo.lock`: `name = "zonendpoint"` `version = "0.3.1"`
6. `CHANGELOG.md`: Thêm mục `[0.3.1] - 2026-11-14` tóm tắt đầy đủ 8 cải tiến.

---

## 4. Quality Gates & Verification Evidence

Thực hiện kiểm thử độc lập trong worktree `E:\Zonendpoint-worktrees\v57-client`:

1. **Rust Code Formatting**:
   ```powershell
   cargo fmt -- --check
   ```
   $\rightarrow$ **PASS** (100% compliant).

2. **Rust Linter**:
   ```powershell
   cargo clippy --all-targets -- -D warnings
   ```
   $\rightarrow$ **PASS** (0 warnings, 0 errors).

3. **Rust Test Suite**:
   ```powershell
   cargo test
   ```
   $\rightarrow$ **PASS** (chạy 3 lần liên tiếp đều PASS 100%):
   - 76 lib tests PASS.
   - 1 bin test PASS.
   - 18 resilience integration tests PASS.
   - Tổng cộng: **95/95 tests PASS**.

4. **Frontend TypeScript & Vite Build**:
   ```powershell
   npm.cmd run build
   ```
   $\rightarrow$ **PASS** (tsc 0 lỗi, vite bundle 0 lỗi).

5. **Native End-to-End Test Suite**:
   ```powershell
   node tests/native/run.mjs
   ```
   $\rightarrow$ **PASS** (chạy 2 lần liên tiếp đều PASS 100%, 21/21 checks PASS).

---

## 5. Artifacts & Checksums

- **Thư mục lưu trữ**: `E:\Zonendpoint-tools\artifacts\v57-release\`

| Artifact | File Size | SHA256 Checksum | Description |
|---|---|---|---|
| `Zonendpoint_0.3.1_x64-setup.exe` | 3,650,792 bytes | `1305f89913bfef92e65dc86175dde9628fa978c5b43a8ead557af70d6448f126` | Bản cài đặt NSIS x64 chính thức 0.3.1 |
| `zonendpoint.exe` | 7,931,392 bytes | `7bc7dfda0bc811618d64610c2434110dd6a313458c6db4670a09f0ff0723aff7` | Tệp thực thi standalone release x64 PE32+ |
| `Zonendpoint_0.3.1_x64-setup.exe.sha256` | 99 bytes | N/A | Tệp đối soát SHA256 cho NSIS setup |

---

## 6. Hướng dẫn kiểm tra cho Qeen

1. **Kiểm tra S-1**:
   - Quét thư mục có > 1.000 file sạch (ví dụ 1.200 file) và kèm theo 1 file EICAR test hoặc 1 file mã độc giả định.
   - Xác nhận: trên giao diện, chi tiết đường dẫn của file nhiễm mã độc vẫn được hiển thị và giữ nguyên trong kết quả; tổng số file quét hiển thị đúng > 1.200.
2. **Kiểm tra S-7 & S-6**:
   - Tắt app, cố tình làm hỏng file `%LOCALAPPDATA%\ZonEndpoint\cache.db` (ví dụ ghi đè chuỗi rác "corrupted header").
   - Mở lại app: xác nhận file hỏng đã được tự động đổi tên thành `cache.db.corrupt-...`, file `cache.db` mới được tự động tạo và quét bình thường.
3. **Kiểm tra X-5**:
   - Khi `server-status.json` bị mất hoặc stale (> 2 giờ): xác nhận quét không dùng cache (mỗi lần quét đều gọi trực tiếp sang server).
   - Khi server cập nhật số chữ ký mới: cache thế hệ cũ tự động bị bỏ qua.
4. **Kiểm tra X-6**:
   - Đặt biến môi trường tạm thời `$env:ZONENDPOINT_CLAMD="127.0.0.1:13310"` và mở app.
   - Xác nhận: huy hiệu `ENV ACTIVE` xuất hiện ngay cạnh icon trạng thái server ở góc trên bên trái, dù hàng Server đang ẩn do kết nối OK.
   - Chuyển đổi giữa 2 theme Sáng / Tối: huy hiệu hiển thị rõ nét, đúng độ tương phản, kích thước cửa sổ 800×240px không bị vỡ layout.
5. **Kiểm tra P-5**:
   - Quét thư mục có hàng nghìn file sạch: xác nhận giao diện không bị giật/lag do sự kiện tiến trình đã được điều tiết 10Hz; khi phát hiện mã độc hoặc kết thúc quét, trạng thái cập nhật tức thì.
