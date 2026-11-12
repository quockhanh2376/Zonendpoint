# Handoff: Task V-44 (Timer Monotonic Inactive Trigger for Long Run Safety — Q-32 Resolution) — Vic to Qeen

**Date**: 2026-11-12  
**From**: Victor (Vic)  
**To**: Qeen  
**Task**: V-44 (Sửa nhỏ timer theo Q-32: OnUnitActiveSec -> OnUnitInactiveSec)  
**Approval**: Zon approved (2026-11-11 "Bật mỗi giờ", tiếp tục sửa Q-31 & Q-32)  
**Branch**: `vic/abusech-timer`  
**Base Commit**: `b3771d2` (Task V-43 final)  
**Code Snapshot SHA**: `8edf51ee5f7122819c7fdda56eeb42793e4fb86f` (`8edf51e`)  
**Lab Environment**: Host `udevlevel73` (`100.96.250.245`, Ubuntu 26.04, Python 3.14.4, ClamAV 1.5.4)  
**Status**: COMPLETED, READY FOR QEEN AUDIT & QA (5/5 suites PASS, 67/67 tests, 0 real requests)  

---

## 1. Summary of Changes & Architecture

Hệ thống cập nhật tự động hàng giờ tích hợp hai nguồn cấp dữ liệu abuse.ch (MalwareBazaar và ThreatFox) được triển khai trên máy chủ Lab với mô hình gom cụm một lần reload duy nhất mỗi giờ (**Single Clamd Reload Architecture**).

Trong Task V-44, Vic đã hoàn thiện cơ chế định thời theo đúng khuyến nghị của Qeen tại Q-32:

### A. Systemd Monotonic Timer: Chuyển sang `OnUnitInactiveSec=61min` (Q-32 Fix)
- **Vấn đề Q-32**: Trước đó ở Task V-43, timer dùng `OnUnitActiveSec=61min` (đếm từ lúc service BẮT ĐẦU kích hoạt). Nếu một lượt chạy bị kéo dài (ví dụ request ThreatFox bị trễ mạng tới gần ngưỡng `TimeoutStartSec=600s`, phát sinh tại $t_0 + 580\text{s}$), lượt tiếp theo kích hoạt tại $t_0 + 3660\text{s}$ sẽ chỉ cách request đó $3080\text{s} < 3600\text{s}$. Khi đó request ThreatFox vẫn nằm trong sliding window 60 phút, làm mất 1 slot và có thể dẫn đến việc hủy cả lượt tiếp theo do guard chỉ thấy còn 1 slot.
- **Giải pháp Q-32 (Task V-44)**:
  - Thay đổi cấu hình `zonendpoint-abusech-update.timer`:
    ```ini
    [Timer]
    OnBootSec=5min
    OnUnitInactiveSec=61min
    AccuracySec=1min
    Persistent=false
    ```
  - Thay `OnUnitActiveSec=61min` bằng `OnUnitInactiveSec=61min`.
  - Bộ đếm 61 phút bắt đầu tính **từ thời điểm lượt trước KẾT THÚC hoàn toàn (deactivation)**.
  - Kể cả khi một lượt chạy lâu (request ThreatFox phát sinh ở giây thứ 580, lượt chạy kết thúc ở giây thứ 590), lượt chạy tiếp theo sẽ bắt đầu tại $590\text{s} + 3660\text{s} = 4250\text{s}$, cách request trễ nhất tới $4250 - 580 = 3670\text{s} > 3600\text{s}$ (vượt ngoài cửa sổ trượt 60 phút).
  - Đảm bảo khi lượt tiếp theo bắt đầu, toàn bộ request của lượt trước đều đã hết hạn trong sliding window $\rightarrow$ luôn còn đủ 3/3 slot $\rightarrow$ **cả MalwareBazaar và ThreatFox đều đạt tỷ lệ chạy 100.0%**.
  - Không sửa đổi thêm bất kỳ logic code nào khác ngoài timer và test mô phỏng.

### B. Hành vi kích hoạt khi bật timer (`enable --now`)
- Đã ghi nhận rõ trong `docs/ops/abusech-timer.md`:
  - Khi quản trị viên thực thi `sudo systemctl enable --now zonendpoint-abusech-update.timer` trên máy chủ đã boot $> 5$ phút (`uptime > 5min`), mốc `OnBootSec=5min` đã quá hạn trong quá khứ nên systemd sẽ kích hoạt **CHẠY NGAY LẬP TỨC** lượt đầu tiên.
  - Lượt đầu này sẽ tiêu thụ 2 request (1 MalwareBazaar, 1 ThreatFox).
  - Nếu có hash mới, clamd sẽ được reload một lần duy nhất (khoảng 25 giây).
  - Sau khi lượt đầu tiên hoàn tất, timer bắt đầu đếm $61$ phút (`OnUnitInactiveSec=61min`) để kích hoạt lượt tiếp theo.

### C. Endpoints Recent & Cơ chế Ingestion
- **MalwareBazaar**: Endpoint `https://mb-api.abuse.ch/v2/files/exports/{auth_key}/recent.csv` (~48h gần nhất, 1 request). Transaction SQLite `BEGIN IMMEDIATE ... COMMIT`. Log ghi nhận "Recent export imported successfully".
- **ThreatFox**: Endpoint `https://threatfox-api.abuse.ch/v2/files/exports/{auth_key}/recent.csv` (1 request). Ingestion `ON CONFLICT(ioc, ioc_type) DO UPDATE`.
- **Guard bảo lưu 2 slot**: Kiểm tra và bảo lưu nguyên tử 2 slot dưới khóa `rate_limit.json.lock`. Nếu còn $< 2$ slot $\rightarrow$ bỏ trọn toàn bộ lượt, ghi `last-run.json`, tuyệt đối không retry.
- **Single Reload Clamd**: Tối đa 1 lần/giờ; 0 hash mới $\rightarrow$ bỏ qua reload clamd; pre-flight kiểm tra DB tạm $\le 120$ s.

---

## 2. Live Verification on Lab Host (`100.96.250.245`)

### A. Tình trạng Hiện tại trên Máy chủ Lab
- **clamd daemon**: `active` (PID 2245, **8.614.555 chữ ký**, bind duy nhất `127.0.0.1:3310`, chạy liên tục $> 3$ ngày không ngắt quãng).
- **File chữ ký active**:
  - `/var/lib/clamav/malwarebazaar.hsb`: 1.171.422 chữ ký.
  - `/var/lib/clamav/threatfox.hsb`: 21.625 chữ ký net unique.
- **Systemd Timer**:
  - File: `/etc/systemd/system/zonendpoint-abusech-update.timer`.
  - Cấu hình đã cài đặt:
    ```ini
    [Unit]
    Description=Zonendpoint Hourly abuse.ch (MalwareBazaar + ThreatFox) Feed Update Timer
    Documentation=https://bazaar.abuse.ch/ https://threatfox.abuse.ch/

    [Timer]
    OnBootSec=5min
    OnUnitInactiveSec=61min
    AccuracySec=1min
    Persistent=false

    [Install]
    WantedBy=timers.target
    ```
  - Trạng thái: **`disabled`**, **`inactive (dead)`** (chỉ cài đặt và `daemon-reload`, TUYỆT ĐỐI KHÔNG BẬT).
- **Systemd Service**:
  - File: `/etc/systemd/system/zonendpoint-abusech-update.service`.
  - Trạng thái: **`inactive (dead)`**.
- **Cấu hình & Rules**:
  - `clamd.conf` và `freshclam.conf`: giữ nguyên 100%, không reload clamd.
  - 0 request thật ra ngoài trong quá trình kiểm thử Task V-44.

---

## 3. Automated Test Suites Verification

### A. Python Test Suites trên Lab (`/tmp/v44_test`, Python 3.14.4)
Tất cả 5 test suites đều vượt qua 100% (**67/67 PASS**, 0 request thật tới abuse.ch):
1. `test_abusech_rate_limit.py`: **8/8 PASS** (0.008s):
   - `test_combined_hourly_limit_alternation_and_legacy_migration`: PASS.
   - `test_combined_daily_cap`: PASS.
   - `test_invalid_json_or_missing_requests_refuses_without_overwrite`: PASS.
   - `test_empty_hourly_with_other_waiting_prioritizes_malwarebazaar`: PASS.
   - `test_reserve_slots_success_and_fulfillment`: PASS.
   - `test_reserve_slots_fails_when_insufficient_hourly`: PASS.
   - `test_reserve_slots_blocks_intervening_manual_request`: PASS.
   - `test_cancel_unconsumed_reservations`: PASS.
2. `test_abusech_timer_simulation.py`: **6/6 PASS** (0.082s):
   - `test_reproduce_old_timer_defect`: PASS (tái hiện bug Q-31 cũ: MB 100%, TF 63.2%).
   - `test_new_timer_guarantees_100_percent_both_feeds`: PASS (mô phỏng 100.000 lượt timer mới với max TF delay 580s: **MB 100.0%, TF 100.0%**).
   - `test_long_run_delayed_tf_near_timeout_guarantees_full_slots_next_run`: PASS (kiểm tra trường hợp cực đoan lượt chạy kéo dài tới 590s sát timeout 600s $\rightarrow$ lượt sau vẫn cách request TF $\ge 61$ phút, đủ cả 3 slot).
   - `test_manual_interference_drops_entire_turn_preserving_fairness`: PASS (có request tay chen ngang $\rightarrow$ bỏ trọn lượt, MB ran == TF ran 100%).
   - `test_timer_unit_file_configuration`: PASS (xác thực cấu hình `OnUnitInactiveSec=61min`, `OnBootSec=5min`, `AccuracySec=1min`, không có `OnUnitActiveSec`).
   - `test_guard_reserves_slots_under_lock`: PASS.
3. `test_zonendpoint_abusech_update.py`: **10/10 PASS** (6.445s):
   - 10 test kịch bản luồng cập nhật, khóa flock, guard bảo lưu 2 slot, xử lý hash mới, rollback, sanitization.
4. `test_zonendpoint_threatfox.py`: **15/15 PASS** (0.023s).
5. `test_zonendpoint_malwarebazaar.py`: **28/28 PASS** (32.579s).

---

## 4. Deliverables & Documentation
1. `scripts/feeds/zonendpoint-abusech-update.timer`: Cập nhật `OnUnitInactiveSec=61min` thay cho `OnUnitActiveSec=61min`.
2. `scripts/feeds/test_abusech_timer_simulation.py`: Bổ sung test case lượt chạy dài (TF trễ sát timeout 600s) và cập nhật hàm mô phỏng tính theo `t_finish`.
3. `docs/ops/abusech-timer.md`: Cập nhật sơ đồ mermaid, giải thích chi tiết `OnUnitInactiveSec=61min`, ghi rõ lưu ý quan trọng khi bật `systemctl enable --now` trên máy đã boot $> 5$ phút.
4. `docs/handoffs/abusech-timer-vic-to-qeen.md`: Cập nhật chi tiết kết quả Task V-44 và Code Snapshot SHA.
5. `docs/project-status.md` & `docs/daily.md`: Ghi nhận tiến độ Task V-44.
6. `docs/tasks/commit-notes.md`: Cập nhật nhật ký commit.

---

## 5. Lab Server Current Status & Next Actions
- **clamd daemon**: `active` (PID 2245, **8.614.555 chữ ký**, bind `127.0.0.1:3310`).
- **File chữ ký active**:
  - `/var/lib/clamav/malwarebazaar.hsb`: 1.171.422 chữ ký.
  - `/var/lib/clamav/threatfox.hsb`: 21.625 chữ ký net unique.
- **Systemd Timer**:
  - File: `/etc/systemd/system/zonendpoint-abusech-update.timer` (đã cập nhật `OnUnitInactiveSec=61min`, đã `daemon-reload`).
  - Trạng thái: **`disabled`**, **`inactive (dead)`**.
- **Blockers**: Không có blocker kỹ thuật nào. Sẵn sàng để Qeen audit & QA lại theo tiêu chí Q-32.
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
