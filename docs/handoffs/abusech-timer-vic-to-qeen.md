# Handoff: Task V-43 (ThreatFox Starvation Fix & Monotonic Timer — Q-31 Resolution) — Vic to Qeen

**Date**: 2026-11-12  
**From**: Victor (Vic)  
**To**: Qeen  
**Task**: V-43 (Sửa lỗi ThreatFox bị bỏ lượt trong lịch mỗi giờ — Q-31)  
**Approval**: Zon approved (2026-11-11 "Bật mỗi giờ", tiếp tục sửa Q-31)  
**Branch**: `vic/abusech-timer`  
**Base Commit**: `af972e7` (Task V-42)  
**Code Snapshot SHA**: `36697414e0bf733bf9e696c63f2df5eef0a0bb3d` (`3669741`)  
**Lab Environment**: Host `udevlevel73` (`100.96.250.245`, Ubuntu 26.04, Python 3.14.4, ClamAV 1.5.4)  
**Status**: COMPLETED, READY FOR QEEN AUDIT & QA (5/5 suites PASS, 0 real requests)  

---

## 1. Summary of Changes & Architecture

Hệ thống cập nhật tự động hàng giờ tích hợp hai nguồn cấp dữ liệu abuse.ch (MalwareBazaar và ThreatFox) được triển khai trên máy chủ Lab với mô hình gom cụm một lần reload duy nhất mỗi giờ (**Single Clamd Reload Architecture**).

Trong Task V-43, Vic đã khắc phục triệt để lỗi bất đối xứng làm ThreatFox bị nghẽn (Q-31) theo đúng yêu cầu:

### A. Endpoints Recent & Cơ chế Ingestion
- **MalwareBazaar**:
  - Endpoint: `https://mb-api.abuse.ch/v2/files/exports/{auth_key}/recent.csv` (~48h gần nhất, 1 request).
  - Ingestion vào `/opt/zonendpoint/malwarebazaar/malwarebazaar.db` bằng transaction SQLite `BEGIN IMMEDIATE ... COMMIT`.
  - Cập nhật dòng trùng bằng `update_last_seen=True`, làm mới `last_seen` và `updated_at`.
  - **Sửa log (Item 3)**: Đổi thông điệp log trong `import_dump_csv()` khi nhập `recent.csv` thành `Recent export imported successfully` thay vì `Full dump imported successfully`.
- **ThreatFox**:
  - Endpoint: `https://threatfox-api.abuse.ch/v2/files/exports/{auth_key}/recent.csv` (1 request).
  - Tải về giới hạn 512 MB, lưu bản recent dump tại `/opt/zonendpoint/threatfox/last-recent.csv` (`0600 root:root`) để replay ngoại tuyến.
  - Ingestion vào `/opt/zonendpoint/threatfox/threatfox.db` với cơ chế `ON CONFLICT(ioc, ioc_type) DO UPDATE`.

### B. Guard & Cơ chế Bảo lưu Slot dưới Khóa Chung (Item 1)
- **Vấn đề Q-31**: Với thứ tự cố định MB -> TF, nếu cửa sổ trượt chỉ còn 1 slot, MB lấy slot cuối rồi TF bị rate limiter từ chối (mô phỏng 100k lượt: MB 100%, TF 63,2%).
- **Thiết kế Guard mới**:
  - Trong `scripts/feeds/abusech_rate_limit.py`: bổ sung phương thức `reserve_slots(count=2, sources=("malwarebazaar", "threatfox"))`.
  - Kiểm tra và bảo lưu nguyên tử **ngay dưới khóa file `rate_limit.json.lock`** (`fcntl.flock(LOCK_EX)`).
  - Điều kiện: chỉ bắt đầu lượt khi còn **ĐỦ 2 slot** (`hourly_remaining >= 2` và `daily_remaining >= 2`).
  - Khi đủ slot: ghi các bản ghi tạm thời với cờ `reserved: True` và `token` định danh vào `rate_limit.json`. Bất kỳ request thủ công nào chạy đồng thời sẽ thấy số slot khả dụng đã hết và không thể chen vào giữa làm mất slot của TF.
  - Khi MB và TF thực thi: `check_and_record()` xác nhận khớp token và chuyển trạng thái slot thành fulfilled (`reserved: False`), không đếm trùng request.
  - Khi thiếu slot (e.g. do request tay chen vào trước đó): guard phát sinh `RateLimitError`, script **bỏ trọn toàn bộ lượt** (cả MB và TF đều không chạy, 0 requests), ghi nhận chi tiết vào `last-run.json`, tuyệt đối **KHÔNG retry**.
  - Xử lý dọn dẹp: `cancel_unconsumed_reservations()` trong khối `finally` tự động giải phóng các slot chưa dùng nếu có lỗi bất ngờ xảy ra giữa chừng.

### C. Systemd Monotonic Timer (Item 2)
- Thay thế `OnCalendar=hourly` + `RandomizedDelaySec=5min` bằng timer đơn điệu:
  - `OnBootSec=5min`: Lần chạy đầu tiên diễn ra 5 phút sau khi máy chủ khởi động.
  - `OnUnitActiveSec=61min`: Các lần chạy tiếp theo kích hoạt cách lần chạy trước **cố định $\ge 61$ phút**.
  - `AccuracySec=1min`: Giới hạn độ trôi kích hoạt trong biên độ an toàn 61–62 phút.
  - Bỏ `RandomizedDelaySec` (loại bỏ hoàn toàn rủi ro 2 lượt cách nhau $< 60$ phút).
  - `Persistent=false`: Không gọi bù dồn dập sau downtime.
- **Kết quả**: Khoảng cách giữa 2 lượt tự động luôn $\ge 61$ phút $> 60$ phút (cửa sổ trượt rate limit). Do đó, ở mỗi lượt tự động bình thường, toàn bộ request của lượt trước đã trôi khỏi cửa sổ 60 phút $\rightarrow$ luôn còn đủ 3 slot $\rightarrow$ **cả MalwareBazaar và ThreatFox đều đạt tỷ lệ thành công 100.0%**.

### D. Single Reload & An toàn Vận hành
- clamd reload tối đa 1 lần mỗi giờ; nếu 0 hash mới qua cả 2 feed $\rightarrow$ bỏ qua reload clamd.
- Kiểm tra host available RAM $\ge 3.0$ GB; kiểm tra pre-flight trên DB ClamAV tạm thời với 3 file lành tính (0 load errors, 0 False Positives).
- Sao lưu active file sang `/var/backups/`, thay thế nguyên tử, gửi đúng 1 lệnh `zRELOAD\0` tới `127.0.0.1:3310`, kiểm tra timeout $\le 120.0$ s và delta chữ ký. Tự động rollback khi có bất kỳ sự cố nào.
- Trạng thái mỗi lần chạy ghi vào `/opt/zonendpoint/abusech/last-run.json` (Auth-Key hoàn toàn không lộ).
- **Trạng thái trên Lab**: Unit files đã cài đặt (`systemctl daemon-reload`) nhưng timer **VẪN TẮT** (`disabled`, `inactive (dead)`). `clamd.conf` và `freshclam.conf` giữ nguyên 100%.

---

## 2. Live Verification on Lab Host (`100.96.250.245`)

### A. Tình trạng Hiện tại trên Máy chủ Lab
- **clamd daemon**: `active` (PID 2245, **8.614.555 chữ ký**, bind duy nhất `127.0.0.1:3310`).
- **File chữ ký active**:
  - `/var/lib/clamav/malwarebazaar.hsb`: 1.171.422 chữ ký.
  - `/var/lib/clamav/threatfox.hsb`: 21.625 chữ ký net unique.
- **Systemd Timer**:
  - File: `/etc/systemd/system/zonendpoint-abusech-update.timer`.
  - Cấu hình: `OnBootSec=5min`, `OnUnitActiveSec=61min`, `AccuracySec=1min`, `Persistent=false`.
  - Trạng thái: **`disabled`**, **`inactive (dead)`**.
- **Systemd Service**:
  - File: `/etc/systemd/system/zonendpoint-abusech-update.service`.
  - Trạng thái: **`inactive (dead)`**.
- **Cấu hình & Rules**:
  - `clamd.conf` và `freshclam.conf`: giữ nguyên 100%, không reload clamd.
  - Không start service ngoài lệnh, không chạy background scheduler.

---

## 3. Automated Test Suites Verification

### A. Python Test Suites trên Lab (`/tmp/v43_test`, Python 3.14.4)
Tất cả 5 test suites đều vượt qua 100% (**66/66 PASS**, 0 request thật tới abuse.ch):
1. `test_abusech_rate_limit.py`: **8/8 PASS** (0.008s):
   - `test_combined_hourly_limit_alternation_and_legacy_migration`: PASS.
   - `test_combined_daily_cap`: PASS.
   - `test_invalid_json_or_missing_requests_refuses_without_overwrite`: PASS.
   - `test_empty_hourly_with_other_waiting_prioritizes_malwarebazaar`: PASS.
   - `test_reserve_slots_success_and_fulfillment`: PASS (bảo lưu 2 slot, MB & TF fulfill thành công).
   - `test_reserve_slots_fails_when_insufficient_hourly`: PASS (còn 1 slot thì từ chối ngay).
   - `test_reserve_slots_blocks_intervening_manual_request`: PASS (request tay chen vào bị chặn 3/3, bảo toàn slot cho TF).
   - `test_cancel_unconsumed_reservations`: PASS (dọn dẹp slot chưa dùng khi lỗi).
2. `test_abusech_timer_simulation.py`: **5/5 PASS** (0.073s):
   - `test_reproduce_old_timer_defect`: PASS (tái hiện chính xác bug Q-31: MB 100%, TF 63.2%).
   - `test_new_timer_guarantees_100_percent_both_feeds`: PASS (mô phỏng 100.000 lượt với timer mới: **MB 100.0%, TF 100.0%**).
   - `test_manual_interference_drops_entire_turn_preserving_fairness`: PASS (có request tay chen ngang $\rightarrow$ bỏ trọn lượt, MB ran == TF ran 100%).
   - `test_timer_unit_file_configuration`: PASS (xác thực `OnUnitActiveSec=61min`, `OnBootSec=5min`, `AccuracySec=1min`).
   - `test_guard_reserves_slots_under_lock`: PASS (khóa file và bảo lưu nguyên tử).
3. `test_zonendpoint_abusech_update.py`: **10/10 PASS** (6.445s):
   - `test_01_concurrent_run_blocked_by_lock`: PASS.
   - `test_02_rate_limit_exhausted_skips_turn_without_retry`: PASS.
   - `test_03_execution_order_mb_first_then_tf`: PASS.
   - `test_04_new_hashes_triggers_export_and_single_reload`: PASS.
   - `test_05_no_new_hashes_skips_clamd_reload`: PASS.
   - `test_06_deployment_failure_rolls_back`: PASS.
   - `test_07_last_run_json_never_contains_auth_key`: PASS.
   - `test_08_upstream_error_skips_turn_cleanly_no_continuous_retry`: PASS.
   - `test_09_manual_intervention_leaving_one_slot_skips_entire_turn`: PASS (bỏ trọn lượt khi còn 1 slot).
   - `test_10_guard_reserves_two_slots_under_lock_enables_both_feeds`: PASS.
4. `test_zonendpoint_threatfox.py`: **15/15 PASS** (0.022s).
5. `test_zonendpoint_malwarebazaar.py`: **28/28 PASS** (32.585s).

### B. Rust Quality Gates (Local Workspace)
- `cargo fmt --check`: PASS (0 diffs).
- `cargo clippy --all-targets -- -D warnings`: PASS (0 warnings).
- `cargo test`: 68/68 PASS (49 lib tests + 1 bin test + 18 resilience integration tests).

---

## 4. Deliverables & Documentation
1. `scripts/feeds/abusech_rate_limit.py`: Bổ sung `reserve_slots()`, `cancel_unconsumed_reservations()`, quản lý token bảo lưu slot dưới khóa `rate_limit.json.lock`.
2. `scripts/feeds/zonendpoint-abusech-update.py`: Bổ sung guard bảo lưu 2 slot trước khi chạy, bỏ trọn lượt nếu thiếu, ghi `last-run.json`.
3. `scripts/feeds/zonendpoint-abusech-update.timer`: Cập nhật cấu hình timer đơn điệu `OnBootSec=5min`, `OnUnitActiveSec=61min`, `AccuracySec=1min`.
4. `scripts/feeds/zonendpoint-malwarebazaar.py`: Sửa thông điệp log import recent thành "Recent export imported successfully".
5. `scripts/feeds/test_abusech_rate_limit.py`: Bổ sung 4 unit tests kiểm thử guard & reservation.
6. `scripts/feeds/test_zonendpoint_abusech_update.py`: Bổ sung 2 unit tests kiểm thử bỏ trọn lượt và bảo lưu slot.
7. `scripts/feeds/test_abusech_timer_simulation.py`: Suite mô phỏng 100k lượt kiểm chứng timer và guard.
8. `docs/ops/abusech-timer.md`: Cập nhật sơ đồ mermaid, cấu hình timer mới và tài liệu vận hành.
9. `docs/project-status.md` & `docs/daily.md`: Ghi nhận tiến độ Task V-43.
10. `docs/tasks/commit-notes.md`: Cập nhật nhật ký commit.

---

## 5. Lab Server Current Status & Next Actions
- **clamd daemon**: `active` (PID 2245, **8.614.555 chữ ký**, bind `127.0.0.1:3310`).
- **File chữ ký active**:
  - `/var/lib/clamav/malwarebazaar.hsb`: 1.171.422 chữ ký.
  - `/var/lib/clamav/threatfox.hsb`: 21.625 chữ ký net unique.
- **Systemd Timer**:
  - File: `/etc/systemd/system/zonendpoint-abusech-update.timer`.
  - Trạng thái: **`disabled`**, **`inactive (dead)`**.
- **Blockers**: Không có blocker kỹ thuật nào. Sẵn sàng để Qeen review lại (re-verify) trước khi anh Zon quyết định thời điểm bật timer.
