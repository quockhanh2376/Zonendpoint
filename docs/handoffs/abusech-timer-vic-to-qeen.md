# Handoff: Task V-42 (Hourly Automatic Schedule for MalwareBazaar + ThreatFox) — Vic to Qeen

**Date**: 2026-11-11  
**From**: Victor (Vic)  
**To**: Qeen  
**Task**: V-42 (Hourly automatic schedule for MalwareBazaar + ThreatFox)  
**Approval**: Zon approved 2026-11-11 ("Bật mỗi giờ")  
**Branch**: `vic/abusech-timer`  
**Base Commit**: `origin/main 1399604` (Q-30 PASS)  
**Code Snapshot SHA**: `1b6834c2c5fba0a0281e116cb1baf4f22420237a` (`1b6834c`)  
**Lab Environment**: Host `udevlevel73` (`100.96.250.245`, Ubuntu 26.04, Python 3.14.4, ClamAV 1.5.4)  
**Status**: COMPLETED, READY FOR QEEN AUDIT & QA  

---

## 1. Summary of Changes & Architecture

Hệ thống cập nhật tự động hàng giờ tích hợp hai nguồn cấp dữ liệu abuse.ch (MalwareBazaar và ThreatFox) được triển khai trên máy chủ Lab với mô hình gom cụm một lần reload duy nhất mỗi giờ (**Single Clamd Reload Architecture**):

### A. Endpoints Recent & Cơ chế Ingestion
- **MalwareBazaar**:
  - Endpoint sử dụng: `https://mb-api.abuse.ch/v2/files/exports/{auth_key}/recent.csv` (~48h gần nhất, cập nhật mỗi 5 phút, 1 request).
  - Tải về và phân tích trực tiếp theo định dạng comment header của abuse.ch.
  - Ingestion vào `/opt/zonendpoint/malwarebazaar/malwarebazaar.db` bằng transaction SQLite `BEGIN IMMEDIATE ... COMMIT`.
  - Cập nhật dòng trùng bằng `update_last_seen=True`, làm mới `last_seen` và `updated_at`.
- **ThreatFox**:
  - Endpoint sử dụng: `https://threatfox-api.abuse.ch/v2/files/exports/{auth_key}/recent.csv` (1 request, định dạng CSV với header `#` tương đồng với full export).
  - Tải về có giới hạn kích thước (512 MB), lưu bản recent dump tại `/opt/zonendpoint/threatfox/last-recent.csv` (`0600 root:root`) để hỗ trợ debug/replay ngoại tuyến.
  - Ingestion vào `/opt/zonendpoint/threatfox/threatfox.db` với cơ chế gộp `ON CONFLICT(ioc, ioc_type) DO UPDATE` đã được Zon phê duyệt.

### B. Master Update Script (`zonendpoint-abusech-update.py`)
- Chạy dưới quyền `root`, sử dụng `flock` trên file `/opt/zonendpoint/abusech/update.lock` chống chạy chồng chéo.
- Kiểm tra hạn mức chung tại `/opt/zonendpoint/abusech/rate_limit.json`: tối đa 3 requests/giờ (cửa sổ trượt 3600s) và 400 requests/24h.
- **Thứ tự thực thi nghiêm ngặt**: MalwareBazaar chạy trước (1 request) $\rightarrow$ ThreatFox chạy sau (1 request).
- **Chính sách lỗi**: Nếu gặp hết hạn mức, mã lỗi HTTP 429, 403, blacklist, hoặc lỗi mạng $\rightarrow$ Bỏ lượt ngay lập tức, ghi log cảnh báo và đợi giờ sau; tuyệt đối **KHÔNG retry liên tục**.
- **Cơ chế Single Reload**:
  - Nếu cả hai feed đều có 0 hash mới: **Không reload clamd** (tránh lãng phí tài nguyên và rủi ro gián đoạn).
  - Nếu có ít nhất 1 hash mới: Cả 2 file `.hsb` được sinh vào thư mục tạm, chạy kiểm tra pre-flight trên DB ClamAV tạm thời với 3 file lành tính, sao lưu file hiện tại vào `/var/backups/`, thay thế nguyên tử vào `/var/lib/clamav/`, và chỉ gửi đúng **1 lệnh `zRELOAD\0`** tới `127.0.0.1:3310`.
  - Kiểm soát ngưỡng an toàn: Host khả dụng RAM $\ge 3.0$ GB; thời gian reload $\le 120.0$ s. Nếu quá ngưỡng sẽ tự động hủy và rollback.
- Ghi trạng thái mỗi lần chạy vào `/opt/zonendpoint/abusech/last-run.json` (không chứa bí mật).

### C. Systemd Service & Timer Hardening
- `/etc/systemd/system/zonendpoint-abusech-update.service`:
  - `Type=oneshot`, `TimeoutStartSec=600`.
  - Bảo mật: `NoNewPrivileges=yes`, `PrivateTmp=yes`, `ProtectHome=read-only`, `ProtectSystem=full`.
- `/etc/systemd/system/zonendpoint-abusech-update.timer`:
  - `OnCalendar=hourly`, `RandomizedDelaySec=5min`, `Persistent=false`.
- **Trạng thái trên Lab**: Cả hai tệp đã được cài đặt lên hệ điều hành và nạp qua `systemctl daemon-reload`, nhưng **ĐỂ TẮT** (`disabled`, `inactive`), tuân thủ nghiêm ngặt yêu cầu không tự ý bật background scheduler khi chưa có lệnh.
- Cấu hình `clamd.conf` và `freshclam.conf` giữ nguyên 100%, clamd tiếp tục bind tại `127.0.0.1:3310`.

---

## 2. Live Verification on Lab Host (`100.96.250.245`)

### A. Lần chạy thật thứ nhất (Manual Run 1)
Kích hoạt qua systemd: `sudo systemctl start zonendpoint-abusech-update.service`
- **Tình trạng hạn mức**:
  - Trước khi chạy: 0/3 hourly (sliding 60m), 9/400 daily.
  - Sau khi chạy: 2/3 hourly, 11/400 daily (dùng đúng 2 request: 1 MB, 1 TF).
- **MalwareBazaar**:
  - Tải về 630.421 bytes CSV dump.
  - Phân tích 1.348 dòng, nạp 182 hash mới (tổng DB: 1.171.422).
- **ThreatFox**:
  - Tải về và phân tích 7.921 dòng, nạp 258 hash mới (tổng DB: 45.298).
  - Khử trùng lặp với MalwareBazaar: loại trừ 23.673 dòng trùng, số chữ ký net unique tăng +149 (từ 21.476 lên 21.625).
- **Triển khai clamd**:
  - Host RAM khả dụng: 11.68 GB ($\ge 3.0$ GB).
  - Pre-flight check DB tạm thời: 4.821.305 viruses, 0 errors, 0 False Positives.
  - Bản sao lưu `.bak` được tạo tại `/var/backups/clamav-malwarebazaar/` và `/var/backups/clamav-threatfox/`.
  - Chữ ký clamd trước reload: **8.614.224**.
  - Chữ ký clamd sau reload: **8.614.555** (tăng đúng $+331$ chữ ký $= 182 \text{ (MB)} + 149 \text{ (TF)}$).
  - Thời gian reload: **24.66s** (nhỏ hơn rất nhiều so với ngưỡng 120s).
  - clamd phản hồi `PING -> PONG` ngay sau reload.
- **Trạng thái ghi nhận (`/opt/zonendpoint/abusech/last-run.json`)**:
```json
{
  "timestamp": "2026-11-11T09:37:16.954850+00:00",
  "duration_seconds": 47.02,
  "requests": {
    "malwarebazaar": 1,
    "threatfox": 1,
    "total": 2
  },
  "new_hashes": {
    "malwarebazaar": 182,
    "threatfox": 258,
    "total": 440
  },
  "reloaded": true,
  "signatures_before": 8614224,
  "signatures_after": 8614555,
  "reload_duration_seconds": 24.66,
  "error": null
}
```

### B. Lần chạy thứ hai ngay sau đó (Immediate Run 2)
Kích hoạt ngay lần 2 để kiểm chứng giới hạn rate limit và chống reload thừa:
- MB gọi export (yêu cầu thứ 3 trong giờ): hoàn thành, nạp 0 hash mới (tất cả 1.348 dòng đã có).
- TF chuẩn bị gọi: bộ đếm RateLimiter phát hiện đã dùng 3/3 requests trong 60 phút $\rightarrow$ phát sinh `RateLimitError` và **bỏ lượt ngay lập tức**, không retry.
- Đánh giá chữ ký: MB mới $= 0$, TF mới $= 0 \rightarrow$ Tổng hash mới $= 0$.
- **Hệ thống bỏ qua reload clamd**: "No new signatures to deploy across feeds. Skipping clamd reload."
- Thời gian chạy: **3.19s**.
- File `last-run.json`: `reloaded: false`, `error: "ThreatFox: Shared abuse.ch rate limit exceeded: 3/3 requests in last 60 minutes; stopping without retry."`.

### C. Lần chạy thứ ba ngay sau đó (Immediate Run 3)
Kích hoạt lần 3 khi hạn mức 3/3 đã được ghi nhận:
- Hệ thống kiểm tra trước (pre-check) phát hiện `hourly_remaining <= 0` (3/3).
- Bỏ lượt ngay lập tức ở giây thứ 0.0s với cảnh báo: "Shared hourly rate limit reached (3/3 in sliding 60m). Skipping turn without retry."
- Tiêu thụ đúng 0 request, 0 reload.

### D. Rà soát rò rỉ bí mật (Secret Audit)
- `sudo sh -c 'journalctl -u zonendpoint-abusech-update.service --no-pager | grep -Ff /etc/zonendpoint/abusech.key || echo JOURNAL_CLEAN'` $\rightarrow$ **`JOURNAL_CLEAN`**.
- `sudo sh -c 'grep -rlFf /etc/zonendpoint/abusech.key /var/log /opt/zonendpoint/abusech/last-run.json || echo NO_KEY_LEAKED'` $\rightarrow$ **`NO_KEY_LEAKED`**.

---

## 3. Automated Test Suites Verification

### A. Python Test Suites trên Lab (`/tmp/suite_test`, Python 3.14.4)
Tất cả 4 test suites đều vượt qua 100%:
1. `test_abusech_rate_limit.py`: **4/4 PASS** (0.003s).
2. `test_zonendpoint_threatfox.py`: **14/14 PASS** (0.024s).
3. `test_zonendpoint_malwarebazaar.py`: **28/28 PASS** (26.685s).
4. `test_zonendpoint_abusech_update.py`: **8/8 PASS** (0.076s):
   - `test_01_flock_prevents_concurrent_runs`: PASS.
   - `test_02_rate_limit_hourly_skip_turn_no_retry`: PASS.
   - `test_03_order_mb_first_then_tf`: PASS.
   - `test_04_new_hashes_trigger_single_reload`: PASS.
   - `test_05_zero_new_hashes_skips_reload`: PASS.
   - `test_06_reload_timeout_triggers_rollback`: PASS.
   - `test_07_last_run_json_contains_no_auth_key`: PASS.
   - `test_08_upstream_503_skips_turn_without_retry`: PASS.
- **Tổng cộng**: **54/54 PASS**, 0 request thật tới abuse.ch.

### B. Rust Quality Gates (Local Workspace)
- `cargo fmt --check`: PASS (0 diffs).
- `cargo clippy --all-targets -- -D warnings`: PASS (0 warnings).
- `cargo test`: 68/68 PASS (49 lib tests + 1 bin test + 18 resilience integration tests).

---

## 4. Deliverables & Documentation

1. `scripts/feeds/abusech_rate_limit.py`: Bổ sung hàm `get_status()` kiểm tra trạng thái hạn mức không tiêu thụ request.
2. `scripts/feeds/zonendpoint-malwarebazaar.py`: Bổ sung `DEFAULT_RECENT_DUMP_URL`, hàm `fetch_recent_export()`, hỗ trợ cập nhật `last_seen` khi gặp dòng trùng, cờ CLI `--recent-url`, `--use-export`, `--use-api`.
3. `scripts/feeds/zonendpoint-threatfox.py`: Bổ sung `DEFAULT_RECENT_URL`, `fetch_recent()`, `get_hash_count()`, `get_ioc_count()`, `get_net_deployable_count()`, subcommand `fetch-recent`.
4. `scripts/feeds/zonendpoint-abusech-update.py`: Master update script điều phối định kỳ.
5. `scripts/feeds/zonendpoint-abusech-update.service`: Systemd oneshot unit có hardening.
6. `scripts/feeds/zonendpoint-abusech-update.timer`: Systemd timer hàng giờ (`OnCalendar=hourly`, `RandomizedDelaySec=5min`, `Persistent=false`).
7. `scripts/feeds/test_zonendpoint_abusech_update.py`: Bộ kiểm thử tự động 8 ca kiểm thử mock toàn diện.
8. `docs/ops/abusech-timer.md`: Tài liệu hướng dẫn vận hành, bật/tắt timer, xem log, và quy trình rollback.
9. `docs/handoffs/abusech-timer-vic-to-qeen.md`: Tài liệu bàn giao chi tiết.
10. `docs/project-status.md` & `docs/daily.md`: Cập nhật tiến độ dự án.
11. `docs/tasks/commit-notes.md`: Nhật ký commit theo dõi cho Loki.

---

## 5. Lab Server Current Status & Next Actions

- **clamd daemon**: `active` (PID 2245, **8.614.555 chữ ký**, bind `127.0.0.1:3310`).
- **File chữ ký active**:
  - `/var/lib/clamav/malwarebazaar.hsb`: 1.171.422 chữ ký.
  - `/var/lib/clamav/threatfox.hsb`: 21.625 chữ ký net unique.
- **Systemd Timer**:
  - File: `/etc/systemd/system/zonendpoint-abusech-update.timer`.
  - Trạng thái: **`disabled`**, **`inactive (dead)`**.
- **Blockers**: Không có blocker kỹ thuật nào. Sẵn sàng để Qeen review và nghiệm thu.
