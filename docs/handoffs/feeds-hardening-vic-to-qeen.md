# Handoff: Task V-58 — Server Feeds Hardening (optimisse.md §5) — Vic to Qeen

**Date**: 2026-11-14  
**From**: Victor (Vic)  
**To**: Qeen (Auditor & Lab Ops)  
**Task**: V-58 (Sửa script feed & service hardening theo `E:\Zonendpoint\optimisse.md` phần 5)  
**Zon Assignment**: 2026-11-14 (Branch `vic/feeds-hardening`, cô lập repo/test, KHÔNG triển khai lab trong task này)  
**Branch**: `vic/feeds-hardening`  
**Base Commit**: `908535eec72d4c89a31b8376c7c40c6e3d345c67` (`origin/main`)  
**Lab Environment Tested**: Host `udevlevel73` (`100.96.250.245`, Ubuntu 24.04, Python 3.14.4, ClamAV 1.5.4) trong thư mục tạm `/tmp/v58`  
**Test Status**: 76/76 unit tests PASS (0 network calls thật, mock/fixture đầy đủ), `systemd-analyze verify` 3 service PASS (exit 0)  
**Deployment Policy**: CHỈ SỬA CODE VÀ TEST TRONG REPO. Toàn bộ triển khai lên lab cần Zon duyệt trước.  

---

## 1. Chi tiết các hạng mục đã hoàn thành (Từng commit)

### Commit 1 (`e0b659b`): L-1 — Ngắt lượt chạy khi MalwareBazaar gặp 429/401/403/RateLimitError
- **File sửa**: `scripts/feeds/zonendpoint-abusech-update.py`, `scripts/feeds/test_zonendpoint_abusech_update.py`
- **Hành vi**:
  - Khi MalwareBazaar gặp lỗi xác thực hoặc cạn hạn ngạch (`RateLimitError`, HTTP 429 `Too Many Requests`, HTTP 401 `Unauthorized`, HTTP 403 `Forbidden`), updater lập tức dừng toàn bộ lượt chạy hiện tại.
  - Hủy ngay các slot đã bảo lưu trong `rate_limit.json` cho ThreatFox (`limiter.cancel_reservation(token)`), KHÔNG gửi bất kỳ request nào tới ThreatFox (0 request).
  - Ghi rõ nguyên nhân lỗi vào `last-run.json` (`error: "MalwareBazaar: HTTP Error 429..."`).
  - Các lỗi mạng khác hoặc HTTP 503 tạm thời vẫn giữ nguyên cơ chế xử lý cũ.
- **Kiểm thử**: Các test `test_11_malwarebazaar_429...`, `test_12_malwarebazaar_401...`, `test_13_malwarebazaar_403...` xác nhận số request gửi sang ThreatFox luôn bằng 0.

### Commit 2 (`3735dc5`): L-2 — Mặc định `max_retries=0` cho mọi đường gọi abuse.ch
- **File sửa**: `scripts/feeds/zonendpoint-malwarebazaar.py`, `scripts/feeds/test_zonendpoint_malwarebazaar.py`
- **Hành vi**:
  - Đặt `MAX_TRANSIENT_RETRIES = 0` làm hằng số mặc định.
  - Các hàm `_post()`, `check_connection()`, `fetch_recent()`, `fetch_full_dump()` đều nhận mặc định `max_retries=0`.
  - Tuân thủ quy tắc quota nghiêm ngặt: khi gặp lỗi, bỏ lượt ngay, không tự động retry ngầm. Số lượng HTTP request thực tế gửi ra ngoài luôn khớp đúng 1:1 với số lượt ghi nhận trong ledger `rate_limit.json`.
- **Kiểm thử**: `test_29_default_max_retries_zero_for_all_endpoints_counts_match` kiểm tra cả 4 endpoint.

### Commit 3 (`19f0967`): L-8 — Khoảng cách tối thiểu giữa 2 lần nạp clamd (`--min-reload-interval`)
- **File sửa**: `scripts/feeds/zonendpoint-abusech-update.py`, `scripts/feeds/test_zonendpoint_abusech_update.py`
- **Hành vi**:
  - Thêm tham số CLI `--min-reload-interval` (giờ, float, mặc định 6.0 giờ) và `--reload-state-file` (mặc định `/opt/zonendpoint/abusech/reload-state.json`).
  - Khi có chữ ký mới nhưng chưa đủ khoảng cách tối thiểu kể từ lần reload trước: script VẪN tải feed và import vào SQLite database như bình thường, nhưng HOÃN (`defer`) bước export file `.hsb` và hoãn gửi lệnh `zRELOAD` sang clamd.
  - Ghi nhận trạng thái `"reload_deferred": true` vào `last-run.json`.
  - Khi đủ khoảng cách và reload thành công, thời điểm reload được lưu nguyên tử vào `reload-state.json`.
- **Kiểm thử**: `test_14_reload_deferred_when_min_interval_not_elapsed`, `test_15_reload_proceeds_when_min_interval_elapsed`, `test_16_state_file_updated_atomically_on_reload`.

### Commit 4 (`1711522`): L-3 — File hold trên rollback và updater bỏ lượt
- **File sửa**: `scripts/feeds/zonendpoint-malwarebazaar-rollback.sh`, `scripts/feeds/zonendpoint-threatfox-rollback.sh`, `scripts/feeds/zonendpoint-yara-rollback.sh`, `scripts/feeds/zonendpoint-abusech-update.py`, `scripts/feeds/zonendpoint-yara-update.py`, `docs/ops/abusech-timer.md`, `docs/ops/yara-daily-update.md`
- **Hành vi**:
  - Khi chạy các script rollback (kể cả revert backup hay `--purge`), script tự động tạo file hold:
    - Abuse.ch: `/opt/zonendpoint/HOLD-abusech`
    - YARA: `/opt/zonendpoint/HOLD-yara`
  - Các updater (`zonendpoint-abusech-update.py` và `zonendpoint-yara-update.py`) khi thức dậy kiểm tra file hold đầu tiên. Nếu file hold tồn tại, log cảnh báo rõ ràng và thoát ngay với exit code 0 mà không thực hiện tải hay can thiệp clamd.
  - Cập nhật tài liệu vận hành hướng dẫn kiểm tra và gỡ hold (`rm -f /opt/zonendpoint/HOLD-...`).
- **Kiểm thử**: `test_zonendpoint_yara_hold.py` và `test_17_hold_file_skips_hourly_update`.

### Commit 5 (`2412db6`): L-4 — YARA updater ghi tag sau khi kiểm tra sức khỏe & xử lý log xoay vòng
- **File sửa**: `scripts/feeds/zonendpoint-yara-update.py`, `scripts/feeds/test_zonendpoint_yara_hold.py`
- **Hành vi**:
  - Trong `deploy_and_reload`, tag mới (`latest_tag`) CHỈ được ghi vào `current_tag.txt` sau khi đã nhìn thấy dòng `"Database correctly reloaded (<N> signatures)"` trong log clamd và lệnh `PING` -> `PONG` thành công.
  - Nếu reload thất bại hoặc sức khỏe không đạt, nhánh emergency rollback được kích hoạt, tag cũ trong `current_tag.txt` được giữ nguyên vẹn.
  - Thay thế `sleep(8)` cũ bằng vòng lặp kiểm tra log clamd có timeout (`--reload-timeout`, mặc định 120s). Xử lý an toàn trường hợp log file bị xoay vòng (truncation/logrotate khi `cur_size < log_pos_before`).
- **Kiểm thử**: `TestYaraDeploy` kiểm tra tag không bị đổi khi rollback, tag chỉ ghi sau reload, và xử lý log rotation.

### Commit 6 (`1fcbb04`): L-6 — Khóa `flock` chung chống xung đột nạp clamd & sửa fail-open
- **File sửa**: `scripts/feeds/zonendpoint-abusech-update.py`, `scripts/feeds/zonendpoint-yara-update.py`, `scripts/feeds/zonendpoint-malwarebazaar-rollback.sh`, `scripts/feeds/zonendpoint-threatfox-rollback.sh`, `scripts/feeds/zonendpoint-yara-rollback.sh`, `docs/ops/abusech-timer.md`
- **Hành vi**:
  - Thiết lập khóa `flock` độc quyền chung: `/run/lock/zonendpoint-clamd-deploy.lock` (ghi đè được qua biến môi trường `DEPLOY_LOCK_FILE` hoặc tham số `--lock-file`).
  - Dùng chung cho: `abusech-update`, `yara-update` và cả 3 script rollback (`malwarebazaar-rollback`, `threatfox-rollback`, `yara-rollback`).
  - Nếu không lấy được khóa (đang có tiến trình deploy/reload khác nắm giữ): lập tức bỏ lượt và thoát với mã lỗi khác 0 (`return 1` trong Python, `exit 3` trong Shell).
  - Sửa dứt điểm lỗi fail-open ở `zonendpoint-abusech-update.py`: trước đây nếu mở lock file lỗi (ví dụ không tạo được thư mục) thì script vẫn chạy tiếp; nay script log error và thoát mã lỗi khác 0 ngay lập tức.
- **Kiểm thử**: `test_zonendpoint_deploy_lock.py` (5 test) kiểm tra cả 5 script thoát mã lỗi khác 0 khi lock bị giữ hoặc đường dẫn lock không hợp lệ.

### Commit 7 (`c8ea858`): L-11 — File tạm an toàn chống symlink attack trong `/var/lib/clamav` & rollback
- **File sửa**: `scripts/feeds/zonendpoint-malwarebazaar.py`, `scripts/feeds/zonendpoint-abusech-update.py`, `scripts/feeds/zonendpoint-yara-update.py`, và 3 script rollback `.sh`
- **Hành vi**:
  - Loại bỏ hoàn toàn các tên file tạm đoán trước được (như `malwarebazaar.hsb.tmp`, `yara-forge-core.yar.tmp`).
  - Triển khai hàm `safe_atomic_write` trong Python: sử dụng `tempfile.mkstemp(dir=...)` tạo file ngẫu nhiên bảo mật với `O_CREAT | O_EXCL`, áp dụng `os.fchown(fd, uid, gid)` và `os.fchmod(fd, 0o644)` trực tiếp trên file descriptor mở, gọi `os.fsync(fd)` đảm bảo dữ liệu ghi xuống đĩa trước khi `os.replace` nguyên tử.
  - Trong cả 3 script shell rollback: dùng `mktemp -p "$TARGET_DIR" ".$(basename "$ACTIVE_FILE").tmp-XXXXXX"`, `chmod 0644`, `sync "$TMP_FILE"`, rồi mới `mv -f`.
- **Kiểm thử**: `test_zonendpoint_secure_temp.py` (4 test) kiểm tra tính nguyên tử, quyền `0644`, và không để sót bất kỳ file `.tmp` tĩnh nào.

### Commit 8 (`5979ae2`): L-12 — Thêm Sandbox Systemd vào 3 file `.service` & sigcount DynamicUser
- **File sửa**: `scripts/feeds/zonendpoint-abusech-update.service`, `scripts/feeds/zonendpoint-yara-update.service`, `scripts/server/zonendpoint-sigcount.service`, `scripts/server/README.md`, `docs/ops/abusech-timer.md`, `docs/ops/yara-daily-update.md`
- **Hành vi**:
  - Bổ sung cấu hình sandbox nghiêm ngặt theo khuyến nghị bảo mật systemd:
    - `ProtectSystem=strict`
    - `ReadWritePaths=` chỉ trỏ đến đúng các thư mục runtime thực tế (`/opt/zonendpoint`, `/var/lib/clamav`, `/var/backups`, `/run/lock`)
    - `PrivateDevices=yes`
    - `ProtectKernelTunables=yes`
    - `ProtectKernelModules=yes`
    - `ProtectControlGroups=yes`
    - `RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX`
    - `NoNewPrivileges=yes`
    - `PrivateTmp=yes`
  - Riêng `zonendpoint-sigcount.service`: chuyển sang chạy hoàn toàn không cần root bằng `DynamicUser=yes`, `SupplementaryGroups=systemd-journal` (đảm bảo quyền đọc journal `clamav-daemon`), và `StateDirectory=zonendpoint/status` (quản lý thư mục `/var/lib/zonendpoint/status` cho dynamic user).
  - Đã kiểm tra cú pháp và tính tương thích trên lab server bằng `systemd-analyze verify /tmp/v58/services/*.service` (kết quả PASS, exit code 0).
  - Tài liệu hóa quy trình kiểm tra bằng `systemd-analyze security` trong `scripts/server/README.md` và `docs/ops/`.

### Commit 9 (`b17b94b`): L-7 — Thêm timeout cho `clamscan` con & delta chữ ký chính xác
- **File sửa**: `scripts/feeds/filter_yara_clamav.py`, `scripts/feeds/zonendpoint-abusech-update.py`, `scripts/feeds/zonendpoint-malwarebazaar.py`, `scripts/feeds/zonendpoint-yara-update.py`
- **Hành vi**:
  - Bổ sung `timeout=120` cho tất cả các lời gọi `subprocess.run(["clamscan", ...])` nhằm chống tình trạng tiến trình con bị treo vĩnh viễn gây kẹt service.
  - Trong `zonendpoint-abusech-update.py`: tính toán `new_signatures_delta` dựa trên số dòng chữ ký thực tế sinh ra trong staging content (`mb_real_count`, `tf_real_count`) thay vì `COUNT(*)` từ SQLite database, triệt tiêu nguy cơ lệch delta gây rollback oan uổng mỗi giờ nếu có dòng `file_size <= 0`.

---

## 2. Kết quả Kiểm thử Toàn diện trên Lab Server

Tất cả các bài kiểm thử được chạy trực tiếp trên môi trường máy chủ lab trong thư mục tạm `/tmp/v58` (đã dọn dẹp sạch sẽ sau khi test xong):

```text
Ran 76 tests in 42.941s
OK (failures=0, errors=0)
```

Chi tiết các suite:
1. `test_zonendpoint_malwarebazaar.py`: 29/29 tests PASS (max_retries=0, sliding window, error handling, atomic write).
2. `test_zonendpoint_threatfox.py`: 10/10 tests PASS (import, dump validation, rate limiting).
3. `test_zonendpoint_abusech_update.py`: 18/18 tests PASS (L-1 429/401/403, L-8 min-reload-interval, L-3 hold file, L-6 lock non-zero).
4. `test_zonendpoint_yara_hold.py`: 5/5 tests PASS (L-3 hold file, L-4 tag timing, rollback tag preservation, log rotation).
5. `test_zonendpoint_deploy_lock.py`: 5/5 tests PASS (L-6 shared flock across 5 scripts, uncreatable lock fail-safe).
6. `test_zonendpoint_secure_temp.py`: 4/4 tests PASS (L-11 mkstemp, fchmod 0644, fsync, atomic replace).

Kiểm tra service unit:
```text
systemd-analyze verify /tmp/v58/services/*.service -> EXIT 0
```

---

## 3. Danh sách các bước triển khai Lab đề xuất (Cần anh Zon duyệt)

Sau khi anh Zon duyệt mã nguồn nhánh `vic/feeds-hardening`, quy trình triển khai lên lab thực tế khuyến nghị như sau:

### Bước 1: Sao chép các script cập nhật vào `/usr/local/bin`
```bash
# 1.1 Python feed updaters & helpers
sudo cp scripts/feeds/zonendpoint-abusech-update.py /usr/local/bin/
sudo cp scripts/feeds/zonendpoint-malwarebazaar.py /usr/local/bin/
sudo cp scripts/feeds/zonendpoint-threatfox.py /usr/local/bin/
sudo cp scripts/feeds/zonendpoint-yara-update.py /usr/local/bin/
sudo cp scripts/feeds/filter_yara_clamav.py /opt/zonendpoint/yara/
sudo cp scripts/server/zonendpoint-sigcount.py /usr/local/bin/

sudo chmod 0755 /usr/local/bin/zonendpoint-*.py /opt/zonendpoint/yara/filter_yara_clamav.py

# 1.2 Shell rollback scripts
sudo cp scripts/feeds/zonendpoint-malwarebazaar-rollback.sh /usr/local/bin/
sudo cp scripts/feeds/zonendpoint-threatfox-rollback.sh /usr/local/bin/
sudo cp scripts/feeds/zonendpoint-yara-rollback.sh /usr/local/bin/

sudo chmod 0755 /usr/local/bin/zonendpoint-*-rollback.sh
```

### Bước 2: Triển khai các file systemd service đã sandbox
```bash
sudo cp scripts/feeds/zonendpoint-abusech-update.service /etc/systemd/system/
sudo cp scripts/feeds/zonendpoint-yara-update.service /etc/systemd/system/
sudo cp scripts/server/zonendpoint-sigcount.service /etc/systemd/system/

# Phân quyền 0644 root:root cho service units
sudo chmod 0644 /etc/systemd/system/zonendpoint-*.service
sudo systemctl daemon-reload
```

### Bước 3: Nghiệm thu và kiểm tra an ninh (Verification)
```bash
# 3.1 Kiểm tra tính hợp lệ cú pháp systemd
systemd-analyze verify /etc/systemd/system/zonendpoint-abusech-update.service
systemd-analyze verify /etc/systemd/system/zonendpoint-yara-update.service
systemd-analyze verify /etc/systemd/system/zonendpoint-sigcount.service

# 3.2 Kiểm tra điểm đánh giá an ninh (security exposure)
systemd-analyze security zonendpoint-abusech-update.service
systemd-analyze security zonendpoint-yara-update.service
systemd-analyze security zonendpoint-sigcount.service

# 3.3 Chạy thử nghiệm sigcount không cần root
sudo systemctl restart zonendpoint-sigcount.service
cat /var/lib/zonendpoint/status/signatures.json
```

---

## 4. Kế hoạch Rollback trên Lab (Khi cần thiết)

Nếu trong quá trình triển khai xảy ra bất kỳ bất thường nào:

1. **Khôi phục service units về trạng thái cũ**:
   ```bash
   sudo git checkout main -- scripts/feeds/*.service scripts/server/*.service
   sudo cp scripts/feeds/zonendpoint-abusech-update.service /etc/systemd/system/
   sudo cp scripts/feeds/zonendpoint-yara-update.service /etc/systemd/system/
   sudo cp scripts/server/zonendpoint-sigcount.service /etc/systemd/system/
   sudo systemctl daemon-reload
   ```

2. **Khôi phục script feed về commit gốc `908535e`**:
   ```bash
   sudo git checkout 908535eec72d4c89a31b8376c7c40c6e3d345c67 -- scripts/feeds/ scripts/server/
   sudo cp scripts/feeds/*.py /usr/local/bin/
   sudo cp scripts/feeds/*.sh /usr/local/bin/
   sudo cp scripts/server/*.py /usr/local/bin/
   ```

3. **Gỡ bỏ file hold hoặc lock tồn dư (nếu có)**:
   ```bash
   sudo rm -f /run/lock/zonendpoint-clamd-deploy.lock
   sudo rm -f /opt/zonendpoint/HOLD-abusech /opt/zonendpoint/HOLD-yara
   ```
