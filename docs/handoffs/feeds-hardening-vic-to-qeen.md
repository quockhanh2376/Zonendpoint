# Handoff: Task V-58, V-62 & V-68 — Server Feeds Hardening & Sanesecurity Integration — Vic to Qeen

**Date**: 2026-11-14  
**From**: Victor (Vic)  
**To**: Qeen (Auditor & Lab Ops)  
**Task**: V-58 (Script feed & service hardening theo `optimisse.md` §5) + V-62 (Sửa dứt điểm lỗi Q-43 & hoàn thiện optimisse.md §5) + V-68 (Bộ cập nhật Sanesecurity & tích hợp cơ chế bảo vệ clamd)  
**Zon Assignment**: 2026-11-14 (Branch `vic/feed-sanesecurity-impl`, cô lập repo/test, KHÔNG triển khai lab trong task này — triển khai gộp chung sau khi nghiệm thu)  
**Branch**: `vic/feed-sanesecurity-impl`  
**Base Commit**: `cb65ed7` (tip của `vic/feeds-hardening`), cherry-pick 2 commit V-67 (`19bc16b`, `f96fe58`)  
**Lab Environment Tested**: Host `udevlevel73` (`100.96.250.245`, Ubuntu 24.04, Python 3.14.4, ClamAV 1.5.4) trong thư mục tạm `/tmp/v68` (đã dọn dẹp sạch sẽ)  
**Test Status**: **122/122 unit tests PASS 100%** (14/14 tests Sanesecurity mới + 108 tests feed hiện hữu; 0 network calls thật, mock/fixture đầy đủ, 0 real clamd reload — đã chứng minh qua journalctl), `systemd-analyze verify` PASS (exit 0), `systemd-analyze security` đạt mức **7.1 MEDIUM**  
**Deployment Policy**: CHỈ SỬA CODE VÀ TEST TRONG REPO. Toàn bộ triển khai lên lab cần Zon duyệt trước và gộp chung trong một đợt bảo trì duy nhất.  

---

## 1. Chi tiết các hạng mục đã hoàn thành

### Đợt 1 (Task V-58)
- **Commit 1 (`e0b659b`) - L-1**: Ngắt lượt chạy khi MalwareBazaar gặp 429/401/403/RateLimitError, 0 request gửi sang ThreatFox, hủy slot bảo lưu trong `rate_limit.json`.
- **Commit 2 (`3735dc5`) - L-2**: Đặt mặc định `max_retries=0` cho mọi đường gọi abuse.ch.
- **Commit 3 (`19f0967`) - L-8**: Tham số `--min-reload-interval` hoãn reload clamd khi chưa đủ thời gian.
- **Commit 4 (`1711522`) - L-3**: File hold (`HOLD-abusech`, `HOLD-yara`) trên rollback, updater bỏ lượt chạy cleanly.
- **Commit 5 (`2412db6`) - L-4**: YARA updater ghi tag sau khi kiểm tra sức khỏe & xử lý log xoay vòng.
- **Commit 6 (`1fcbb04`) - L-6**: Khóa `flock` độc quyền dùng chung `/run/lock/zonendpoint-clamd-deploy.lock`, sửa fail-open.
- **Commit 7 (`c8ea858`) - L-11**: File tạm an toàn chống symlink attack (`safe_atomic_write` với mkstemp/fchmod/fsync/replace).
- **Commit 8 (`5979ae2`) - L-12**: Sandbox systemd cho 3 file `.service` & sigcount DynamicUser.
- **Commit 9 (`b17b94b`) - L-7**: Thêm timeout=120 cho tiến trình `clamscan` con & delta chữ ký tính theo thực tế sinh ra.

---

### Đợt 2 (Task V-62) — Sửa lỗi Q-43 & Hoàn thiện optimisse.md §5

#### 0. Khắc phục lỗi nghiêm trọng từ Q-43: Zero Real Reload Invariant
- **Bối cảnh & Nguyên nhân gốc rễ**: Trong Q-43, khi chạy test suite trên lab, các bài test gọi script rollback và deploy với cấu hình mặc định (port 3310, binary `clamdscan`), dẫn đến việc gửi lệnh `zRELOAD` thật tới tiến trình `clamd` của server (2 lần reload lúc 20:44 và 20:45).
- **Giải pháp triệt để**:
  1. **Tiêm tham số 100%**: Tất cả socket clamd, port clamd, binary `clamdscan`, đường dẫn DB `/var/lib/clamav`, log file, lock file, reload state file, backup dir đều được tham số hóa qua CLI args và tham số hàm, trỏ hoàn toàn vào thư mục tạm/mock trong bài test.
  2. **Cơ chế Safety Guard bắt buộc**:
     - Trong Python (`zonendpoint-malwarebazaar.py`, `zonendpoint-threatfox.py`, `zonendpoint-yara-update.py`): Hàm `check_safety_guard(host, port, clamdscan_bin)` kiểm tra nếu biến môi trường `ZONENDPOINT_TEST_ENV=1` mà `port == 3310` hoặc `clamdscan_bin == "clamdscan"`, lập tức raise `AssertionError("SAFETY GUARD TRIGGERED: Test attempted to contact real clamd!")`.
     - Trong Shell (`*-rollback.sh`): Nếu `ZONENDPOINT_TEST_ENV=1` và `CLAMD_PORT=3310` hoặc `CLAMDSCAN_BIN=clamdscan`, script lập tức log `CRITICAL SAFETY VIOLATION` và thoát với exit code 99.
  3. **Chứng minh thực nghiệm**: Chạy toàn bộ 108 tests trên máy chủ lab (`udevlevel73`) trong `/tmp/v62`, sau đó truy vấn journal hệ thống:
     ```bash
     sudo journalctl -u clamav-daemon --since '2026-11-14 21:50:00' --no-pager
     # Kết quả: -- No entries --
     ```
     Hoàn toàn không có bất kỳ dòng reload nào được kích hoạt tới `clamd` thật trong suốt quá trình chạy test.

#### 1. L-3: Tạo HOLD file ngay đầu script rollback
- Cả 3 script rollback (`zonendpoint-malwarebazaar-rollback.sh`, `zonendpoint-threatfox-rollback.sh`, `zonendpoint-yara-rollback.sh`) đều tạo file HOLD ngay dòng đầu tiên (trước khi sao chép, di chuyển hay thay thế bất kỳ file nào).
- Mọi nhánh thoát lỗi sau đó (`trap on_err ERR`, khôi phục backup thất bại, v.v.) đều GIỮ NGUYÊN file HOLD để bảo vệ hệ thống không bị updater chạy đè.

#### 2. L-6: Khóa `flock` chung cho lệnh `deploy-hsb` thủ công
- Lệnh `deploy-hsb` thủ công trong cả `zonendpoint-malwarebazaar.py` và `zonendpoint-threatfox.py` đều bắt buộc acquire khóa chung `/run/lock/zonendpoint-clamd-deploy.lock` (sử dụng class `DeployLock`).
- Nếu khóa đang bị chiếm bởi một tiến trình deploy/rollback khác, lệnh lập tức in thông báo lỗi ra stderr và thoát với exit code 3 (chống xung đột nạp clamd).

#### 3. L-8: Khoảng cách reload tối thiểu dùng chung (`reload-state.json`)
- Mọi đường nạp lại clamd do Zonendpoint quản lý (updater hàng giờ abuse.ch, updater hàng ngày YARA, và lệnh `deploy-hsb` thủ công) đều kiểm tra và chia sẻ chung file trạng thái reload: `/opt/zonendpoint/reload-state.json`.
- Nếu chưa đủ `--min-reload-interval` (mặc định 6.0 giờ), việc nạp lại clamd bị hoãn lại (`reload_deferred=True`).
- Riêng lệnh rollback khẩn cấp được phép bỏ qua giới hạn thời gian (`--force-reload` / `forced=True`), nhưng vẫn ghi nhận lại trạng thái reload vào `reload-state.json`.
- **Lưu ý quan trọng**: `freshclam` là dịch vụ cập nhật chữ ký chính thức của ClamAV, chạy độc lập theo timer/cron của hệ điều hành và nằm ngoài tầm kiểm soát của `reload-state.json`.

#### 4. L-12: Thu hẹp `ReadWritePaths` cho từng service
- Thu hẹp cấu hình `ReadWritePaths` trong các service unit từ toàn bộ `/var/backups` xuống đúng thư mục con riêng biệt:
  - `zonendpoint-abusech-update.service`: `/var/backups/clamav-malwarebazaar` và `/var/backups/clamav-threatfox`
  - `zonendpoint-yara-update.service`: `/var/backups/clamav-yara`

#### 5. L-5: Xử lý log clamd xoay vòng & đánh dấu chưa xác minh (không rollback)
- Phát hiện log clamd bị xoay vòng (`cur_size < log_pos_before`) -> tự động reset offset về 0.
- Nếu không đọc được số lượng chữ ký sau reload từ log nhưng kiểm tra `PING` -> `PONG` vẫn thành công: đánh dấu trạng thái là **"chưa xác minh"** (`sig_after is None`), ghi warning log và KHÔNG rollback chỉ vì thiếu dòng log.
- Hàm `get_last_logged_signature_count` tìm kiếm dòng đếm chữ ký gần nhất SAU mốc khởi động clamd gần nhất (`+++ Started at `) để tránh đọc nhầm dòng log của phiên chạy trước.

#### 6. L-9: Preflight chỉ kiểm tra file `.hsb` mới
- Hàm `verify_hsb_on_temp_db` và `verify_hsbs_on_temp_db` chỉ chạy `clamscan -d` đối với file `.hsb` mới tạo trong thư mục tạm, không nạp hay symlink các cơ sở dữ liệu gốc (`main`, `daily`, `bytecode`) của ClamAV, giúp kiểm tra cú pháp nhanh và giảm tải RAM.

#### 7. L-10: Streaming `.hsb` trực tiếp từ SQLite & ThreatFox NOT EXISTS
- `export_hsb_file`: Dùng cursor SQLite đọc và ghi trực tiếp từng dòng vào đĩa thông qua buffer 64KB, không gom danh sách vào bộ nhớ RAM.
- `zonendpoint-threatfox.py`: Sử dụng `ATTACH DATABASE` và truy vấn SQL `WHERE NOT EXISTS (SELECT 1 FROM mbdb.malware_hashes mb WHERE mb.sha256 = hashes.sha256)` để loại trừ các hash trùng với MalwareBazaar hoàn toàn ở tầng SQLite, triệt tiêu việc nạp `set()` lớn vào RAM Python.

#### 8. L-13: Thắt chặt bảo mật tải & giải nén YARA-Forge
- Kiểm tra `tag_name` bằng regex nghiêm ngặt: `^[a-zA-Z0-9][a-zA-Z0-9_.-]{0,63}$`.
- URL tải về bắt buộc phải bắt đầu bằng tiền tố an toàn: `https://github.com/YARAHQ/`.
- Áp đặt giới hạn kích thước: Tải về tối đa 50 MB, giải nén tối đa 100 MB, kiểm tra đường dẫn giải nén chống tấn công Zip Slip traversal.
- Bắt buộc kiểm tra SHA-256 checksum nếu upstream release có phát hành file checksum.

#### 9. L-14: Kiểm soát phân quyền Auth-Key & Bộ lọc che khóa nhạy cảm
- Kiểm tra phân quyền file khóa: Nếu file có quyền mở quá rộng (`mode & 0o077 != 0`) hoặc không thuộc sở hữu của root (`st_uid != 0` ngoài môi trường test), script từ chối chạy và raise `AuthKeyError`.
- Bộ lọc `SensitiveKeyFilter`: Được đăng ký vào root logger và tất cả các handler của root logger, tự động che giấu Auth-Key thành `<AUTH-KEY>` trên mọi luồng xuất log.
- Kiểm tra URL: Hàm `validate_https_url` bắt buộc mọi URL gọi API / tải export phải sử dụng giao thức HTTPS.

---

### Đợt 3 (Task V-68) — Bộ Cập nhật Sanesecurity & Tích hợp Bảo vệ clamd

#### 1. Rà soát & Loại bỏ Placeholder (Task V-68 Mục 1)
- Kiểm tra nội dung thực tế trên lab của 3 file `winnow_*` (`winnow_malware.hdb`, `winnow_extended_malware.hdb`, `winnow_bad_cw.hdb`): Cả 3 file đều chỉ có dung lượng 65-66 bytes và nội dung chỉ chứa hash chuỗi test EICAR từ feed OITC cũ đã ngừng cập nhật.
- Kết luận: Loại bỏ hoàn toàn 3 file `winnow_*` khỏi danh mục allowlist. Bộ Sanesecurity chính thức của Zonendpoint gồm **9 tập tin**: `rogue.hdb`, `malwarehash.hsb`, `hackingteam.hsb`, `foxhole_generic.cdb`, `foxhole_filename.cdb`, `bofhland_malware_attach.hdb`, `porcupine.hsb`, `sigwhitelist.ign2`, `sanesecurity.ftm`.

#### 2. Kiến trúc Script Cập nhật `zonendpoint-sanesecurity.py` (Mục 2)
- **Thư viện chuẩn Python 100%**: Thiết kế độc lập, không phụ thuộc package bên ngoài, cấu trúc tương đồng `zonendpoint-yara-update.py`.
- **Mirror HTTPS & Tải có điều kiện**: Mặc định `https://mirror.seichter.de/sanesecurity/`, chỉ chấp nhận giao thức HTTPS; lưu trữ `ETag` và `last_modified` trong `state.json` để nhận diện `304 Not Modified`; giới hạn kích thước đệm tối đa 20 MiB/file; kiểm tra khoảng cách tối thiểu giữa 2 lần kiểm tra mirror (2.0 giờ).
- **Xác minh chữ ký GPG độc lập**: Keyring riêng tại `/opt/zonendpoint/sanesecurity/gnupg`; khóa công khai bắt buộc khớp fingerprint chính thức: `890D413C9E6EFD809D0EE892D6F4C53697CD8EE6`. File không vượt qua kiểm tra chữ ký sẽ bị hủy bỏ ngay trong thư mục staging, giữ nguyên bản DB cũ trên hệ thống.
- **Staging & Preflight (L-9)**: Tải và giải nén trong staging directory tạm thời; chạy `clamscan -d <staged_files>` kiểm tra cú pháp trên file rỗng trước khi triển khai.
- **Triển khai nguyên tử & Sao lưu (L-11)**: Sử dụng `safe_atomic_write` (mkstemp, fchmod 0644 clamav:clamav, fsync, os.replace) vào `/var/lib/clamav`; tự động sao lưu bản cũ vào `/var/backups/clamav-sanesecurity/<name>.bak`.
- **Khóa chung (L-6) & Khoảng cách nạp lại (L-8)**: Chiếm khóa độc quyền `/run/lock/zonendpoint-clamd-deploy.lock`; tích hợp kiểm tra `/opt/zonendpoint/reload-state.json` với `--min-reload-interval` (6.0 giờ). Nếu chưa đủ thời gian, việc reload được hoãn lại an toàn (`reload_deferred=True`).
- **Kiểm tra sức khỏe sau reload & Emergency Rollback (L-4/L-5)**: Giám sát chuỗi `Database correctly reloaded` trong log clamd (hỗ trợ log rotation) và xác nhận `PING` -> `PONG`; nếu thất bại, tự động kích hoạt `emergency_rollback` (khôi phục từ backup, tạo HOLD file `/opt/zonendpoint/HOLD-sanesecurity`, và reload clamd khẩn cấp).
- **Ghi nhận trạng thái**: Xuất file `/opt/zonendpoint/sanesecurity/last-run.json` không chứa dữ liệu nhạy cảm.

#### 3. Service Sandbox, Timer & Script Rollback (Mục 3)
- `zonendpoint-sanesecurity.service`: Sandbox L-12 hoàn chỉnh (`ProtectSystem=strict`, `ProtectHome=true`, `PrivateTmp=true`, `NoNewPrivileges=true`, `ReadWritePaths=/var/lib/clamav /var/backups/clamav-sanesecurity /opt/zonendpoint/sanesecurity /run/lock`).
- `zonendpoint-sanesecurity.timer`: Chạy định kỳ 2 giờ một lần (`OnCalendar=*-*-* 00/2:15:00`), `RandomizedDelaySec=900` (15 phút), `Persistent=true`.
- `zonendpoint-sanesecurity-rollback.sh`: Tạo HOLD file `/opt/zonendpoint/HOLD-sanesecurity` ngay dòng đầu tiên, chiếm khóa `/run/lock/zonendpoint-clamd-deploy.lock`, khôi phục các file `.bak` hoặc hỗ trợ cờ `--purge` để gỡ bỏ toàn bộ feed.

---

## 2. Kết quả Kiểm thử Toàn diện trên Lab Server

Tất cả các bài kiểm thử được chạy trực tiếp trên máy chủ lab trong thư mục tạm `/tmp/v68` (đã dọn dẹp sạch sẽ):

```text
Ran 122 tests in 34.120s
OK (failures=0, errors=0)
```

Chi tiết các suite:
1. `test_zonendpoint_sanesecurity.py`: **14/14 tests PASS** (safety guard, hold file, lock code 3, reject non-https, 304 not modified, GPG fingerprint mismatch abort, GPG failure preserve old, max size 20MB, preflight fail abort, reload interval deferred, emergency rollback + hold, rollback script hold & restore, rollback --purge, min check interval 2h).
2. `test_zonendpoint_malwarebazaar.py`: **37/37 tests PASS** (L-2, L-5, L-8, L-9, L-10, L-14, safety guard, streaming, log banner, key mask).
3. `test_zonendpoint_threatfox.py`: **16/16 tests PASS** (import, SQLite NOT EXISTS dedup, dump validation, rate limiting, deploy-hsb lock).
4. `test_zonendpoint_abusech_update.py`: **18/18 tests PASS** (L-1 429/401/403, L-8 min-reload-interval, L-3 hold file, L-6 lock non-zero).
5. `test_zonendpoint_yara_hold.py`: **12/12 tests PASS** (L-3 hold file early creation, L-4 tag timing, L-8 defer reload, L-13 URL/size/ZipSlip/tag regex, safety guard).
6. `test_zonendpoint_deploy_lock.py`: **7/7 tests PASS** (L-6 shared flock across Python & Shell rollback scripts, manual deploy-hsb lock, fail-safe).
7. `test_zonendpoint_secure_temp.py`: **4/4 tests PASS** (L-11 mkstemp, fchmod 0644, fsync, atomic replace).
8. `test_abusech_rate_limit.py`: **8/8 tests PASS** (shared ledger, sliding window, reservation token cancellation).
9. `test_abusech_timer_simulation.py`: **6/6 tests PASS** (OnUnitInactiveSec timer simulation, fairness preservation).

### Đánh giá An ninh & Cú pháp Systemd
- `systemd-analyze verify /tmp/v62/scripts/feeds/*.service /tmp/v62/scripts/feeds/*.timer`: **PASS (exit code 0)**.
- `systemd-analyze security --offline=true /tmp/v62/scripts/feeds/*.service`:
  - `zonendpoint-abusech-update.service`: **7.1 MEDIUM** :-|
  - `zonendpoint-yara-update.service`: **7.1 MEDIUM** :-|

### Bằng chứng Không Reload clamd Thật
Truy vấn nhật ký daemon clamd trên lab trong suốt thời gian chạy test:
```bash
sudo journalctl -u clamav-daemon --since '2026-11-14 21:50:00' --no-pager
# Output:
-- No entries --
```

---

## 3. Quy trình Triển khai Chuẩn trên Lab (Chính sách Hợp nhất)

Triển khai lab dùng duy nhất `E:\Zonendpoint-tools\coordination\deploy\deploy-feeds.sh` (Loki, sau khi Qeen nghiệm thu); không bao giờ xóa lock; HOLD chỉ gỡ thủ công sau khi kiểm tra có biên bản.

Tuyệt đối không thực hiện các quy trình thao tác thủ công (dừng/copy tay, chỉnh sửa file trực tiếp trên host). Script `deploy-feeds.sh` đảm nhiệm toàn bộ quy trình:
1. Chiếm khóa chung `/run/lock/zonendpoint-clamd-deploy.lock` trước khi thực hiện bất kỳ thay đổi nào.
2. Ghi nhận trạng thái timer hiện hữu, tạo backup có gắn ngày kèm manifest và mã băm SHA-256.
3. Cài đặt các file script và service/timer sandbox đồng bộ theo MAP.
4. Nạp lại cấu hình daemon systemd và kiểm tra sức khỏe clamd.
5. Khôi phục lại trạng thái active của các timer ban đầu.

---

## 4. Chính sách Rollback & Xử lý Khóa/HOLD

- **Rollback:** Nếu xảy ra sự cố trong quá trình triển khai hoặc nghiệm thu, sử dụng cơ chế rollback tự động hoặc lệnh rollback tích hợp của `deploy-feeds.sh`.
- **Khóa triển khai (`/run/lock/zonendpoint-clamd-deploy.lock`):** Không bao giờ xóa lock thủ công bằng `rm -f`. Khóa sử dụng cơ chế `flock` của Linux kernel, sẽ tự động giải phóng khi tiến trình nắm giữ kết thúc.
- **File HOLD (`HOLD-abusech`, `HOLD-yara`, `HOLD-sanesecurity`):** HOLD chỉ gỡ thủ công sau khi kiểm tra có biên bản điều tra nguyên nhân sự cố. Tuyệt đối không xóa tùy tiện trong quá trình vận hành.

