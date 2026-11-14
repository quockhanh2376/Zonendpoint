# Sanesecurity Feed Ops Guide — Task V-67 & V-68

**Date:** 2026-11-14 · **Status:** Tested & Ready for Deployment (Task V-68). Verified on Lab Ubuntu 24.04 (`100.96.250.245`) with 122/122 unit tests PASS (14/14 Sanesecurity tests), `systemd-analyze verify` PASS (exit 0), `systemd-analyze security` rating 7.1 MEDIUM, zero real clamd reloads.

---

## 1. Tổng quan & Giấy phép Sử dụng

### 1.1 Giới thiệu Feed
Sanesecurity cung cấp các cơ sở dữ liệu chữ ký bổ sung (Unofficial Signatures) cho ClamAV, chuyên sâu về phát hiện malware, macro độc hại, mã độc nhúng trong tài liệu và rogue software.

Sau quá trình đánh giá thực nghiệm A/B trên lab (Task V-67) và kiểm tra chi tiết các file (Task V-68), Zonendpoint tích hợp bộ 9 cơ sở dữ liệu chọn lọc, tối ưu cho môi trường quét file nhị phân và tài liệu văn phòng, loại bỏ hoàn toàn các cơ sở dữ liệu thuần email/spam và các file placeholder không còn cập nhật.

### 1.2 Giấy phép & Điều kiện Sử dụng
- **Nguồn chính thức:** [Sanesecurity Usage & Licenses](https://sanesecurity.com/usage/)
- **Điều kiện bản quyền:** Các cơ sở dữ liệu do Sanesecurity tự biên soạn và các đối tác đóng góp phát hành theo giấy phép GPL v2/v3 hoặc Artistic License.
- **Phạm vi áp dụng tại Zonendpoint:** Miễn phí cho mục đích sử dụng cá nhân và phi thương mại trên máy chủ nội bộ.
- **Chính sách tải mirror:** Truy vấn qua HTTPS từ các mirror chính thức được phê chuẩn (mặc định: `https://mirror.seichter.de/sanesecurity/`). Tuân thủ tải có điều kiện (`If-Modified-Since` / `ETag`) và khoảng cách tối thiểu giữa các lần kiểm tra (tối thiểu 2 giờ).

---

## 2. Danh mục 9 Cơ sở Dữ liệu Được Chọn

Qua Task V-67 (đánh giá FP trên 445 file đa dạng) và V-68 (kiểm tra nội dung 3 file `winnow_*`), danh sách database chính thức của Zonendpoint gồm:

| Tên tập tin | Định dạng | Nhà cung cấp | Mục tiêu phát hiện & Lý do chọn |
| :--- | :--- | :--- | :--- |
| `rogue.hdb` | HDB (MD5) | Sanesecurity | Mã độc giả mạo (rogue software, fake AV, ransomware) |
| `malwarehash.hsb` | HSB (SHA256) | Sanesecurity | Hash SHA-256 mã độc nhị phân mới |
| `hackingteam.hsb` | HSB (SHA256) | Sanesecurity | Hash các công cụ gián điệp, khai thác nhị phân |
| `foxhole_generic.cdb` | CDB (Container) | Foxhole | File nhúng nguy hiểm trong tài liệu nén/văn phòng |
| `foxhole_filename.cdb` | CDB (Container) | Foxhole | Đuôi file nguy hiểm giả mạo tài liệu |
| `bofhland_malware_attach.hdb` | HDB (MD5) | Bofhland | Tệp đính kèm mã độc văn phòng, macro độc hại |
| `porcupine.hsb` | HSB (SHA256) | Porcupine | Hash mã độc lây nhiễm văn bản và script |
| `sigwhitelist.ign2` | IGN2 (Whitelist) | Sanesecurity | Danh sách loại trừ cảnh báo nhầm chính thức của Sanesecurity |
| `sanesecurity.ftm` | FTM (Filetype) | Sanesecurity | Định danh cấu trúc tập tin để kiểm soát container |

> [!NOTE]
> **Loại bỏ 3 file `winnow_*` (Task V-68):**
> Ba file `winnow_malware.hdb`, `winnow_extended_malware.hdb`, `winnow_bad_cw.hdb` chỉ có kích thước 65–66 byte và nội dung chỉ chứa chuỗi hash EICAR test (`44d88612fea8a8f36de82e1278abb02f:68:winnow.malware.test.eicar.com`). Đây là các placeholder cũ của dự án OITC đã ngưng hoạt động, không có giá trị bảo vệ nên đã được loại bỏ hoàn toàn khỏi danh sách allowlist.

---

## 3. Kiến trúc Bảo mật & Thiết kế Kỹ thuật (Task V-68)

Bộ cập nhật Sanesecurity (`zonendpoint-sanesecurity.py`) tuân thủ nghiêm ngặt các nguyên tắc hardening đã chuẩn hóa trong Tasks V-58/V-62:

1. **Giao thức an toàn (HTTPS only):**
   - Chỉ chấp nhận mirror bắt đầu bằng `https://`. Từ chối kết nối HTTP không mã hóa.
2. **Xác minh chữ ký GPG độc lập:**
   - Mỗi file database tải về bắt buộc phải đi kèm file chữ ký rời `.sig`.
   - Xác minh qua GnuPG với keyring riêng biệt đặt tại `/opt/zonendpoint/sanesecurity/gnupg`.
   - Khóa công khai chính thức của Sanesecurity được kiểm tra vân tay (fingerprint) trước khi nhập:
     `890D 413C 9E6E FD80 9D0E  E892 D6F4 C536 97CD 8EE6`
   - Bất kỳ file nào sai lệch chữ ký GPG sẽ bị hủy bỏ ngay tại thư mục tạm, bảo toàn file đang hoạt động trên hệ thống.
3. **Tải có điều kiện & Giới hạn tài nguyên:**
   - Sử dụng header `If-Modified-Since` và `ETag` (lưu tại `/opt/zonendpoint/sanesecurity/state.json`). Nếu mirror trả về `304 Not Modified`, script kết thúc tức thì mà không nạp lại clamd.
   - Giới hạn kích thước tối đa 20 MiB/file nhằm ngăn chặn tấn công từ chối dịch vụ (DoS) hoặc mirror bị lỗi nén.
   - Khoảng cách kiểm tra tối thiểu: 2.0 giờ giữa 2 lần kết nối mirror (dù timer có thể được kích hoạt nhiều lần).
4. **Pre-flight Validation (Task L-9):**
   - Tất cả các file tải về được tập hợp trong thư mục staging tạm thời.
   - Chạy lệnh kiểm tra cú pháp `clamscan -d <staged_files>` trên file rỗng. Chỉ khi ClamAV nạp thành công 100% không phát sinh lỗi cú pháp, quy trình cài đặt mới được tiếp tục.
5. **Cài đặt Nguyên tử & Sao lưu (Task L-11):**
   - File được sao chép sang `/var/backups/clamav-sanesecurity/<name>.bak` trước khi ghi đè.
   - Ghi nguyên tử bằng `safe_atomic_write` (`mkstemp` cùng filesystem `/var/lib/clamav`, `fchmod 0644 clamav:clamav`, `fsync`, `os.replace`).
6. **Khóa Chung & Khoảng cách Nạp lại clamd (Tasks L-6 & L-8):**
   - Khóa độc quyền `/run/lock/zonendpoint-clamd-deploy.lock` (sử dụng `flock`). Nếu có tiến trình cập nhật feed khác đang chạy, script thoát với exit code 3.
   - Kiểm tra `/opt/zonendpoint/reload-state.json`. Nếu khoảng cách từ lần reload clamd gần nhất chưa đủ `--min-reload-interval` (mặc định 6.0 giờ), việc nạp lại clamd sẽ được hoãn lại an toàn (`reload_deferred=True`). Các file DB mới vẫn nằm sẵn trong `/var/lib/clamav` và sẽ có hiệu lực trong lần reload kế tiếp.
7. **Kiểm tra Sức khỏe sau Reload & Tự động Rollback (Tasks L-4 & L-5):**
   - Sau khi gửi lệnh `clamdscan --reload` (hoặc `zRELOAD`), script kiểm tra chuỗi `Database correctly reloaded` trong `/var/log/clamav/clamav.log` (hỗ trợ phát hiện log rotation) và xác nhận kết nối `PING` -> `PONG`.
   - Nếu kiểm tra thất bại, script tự động kích hoạt `emergency_rollback`: khôi phục toàn bộ backup, tạo file HOLD `/opt/zonendpoint/HOLD-sanesecurity`, và reload lại clamd.
8. **Sandbox systemd (Task L-12):**
   - Unit `zonendpoint-sanesecurity.service` áp dụng đầy đủ các chỉ thị bảo vệ nghiêm ngặt (`ProtectSystem=strict`, `ProtectHome=true`, `PrivateTmp=true`, `NoNewPrivileges=true`).
   - `ReadWritePaths` được giới hạn tối thiểu:
     * `/var/lib/clamav`
     * `/var/backups/clamav-sanesecurity`
     * `/opt/zonendpoint/sanesecurity`
     * `/run/lock`

---

## 4. Hướng dẫn Vận hành Thủ công

### 4.1 Kiểm tra Trạng thái & Chạy Cập nhật Thủ công
Chạy thử nghiệm hoặc cập nhật ngay lập tức:

```bash
# Kiểm tra mirror (không tải hoặc cài đặt)
sudo /usr/local/bin/zonendpoint-sanesecurity.py --check-only

# Chạy cập nhật đầy đủ (tôn trọng khoảng cách reload 6h)
sudo /usr/local/bin/zonendpoint-sanesecurity.py

# Bắt buộc kiểm tra và bắt buộc nạp lại clamd ngay (bỏ qua khoảng cách)
sudo /usr/local/bin/zonendpoint-sanesecurity.py --force-check --force-reload
```

### 4.2 Kiểm tra Trạng thái Hoạt động
Xem file log và kết quả lần chạy gần nhất:
```bash
# Đọc trạng thái chạy gần nhất
sudo cat /opt/zonendpoint/sanesecurity/last-run.json

# Xem nhật ký thực thi của systemd
sudo journalctl -u zonendpoint-sanesecurity.service -n 50 --no-pager

# Kiểm tra log nạp cơ sở dữ liệu của clamd
sudo grep -i "sanesecurity\|correctly reloaded" /var/log/clamav/clamav.log | tail -n 20
```

---

## 5. Quy trình Rollback & Xử lý Sự cố

### 5.1 Khôi phục Khẩn cấp (Rollback về bản Backup)
Nếu một bản cập nhật gây cảnh báo nhầm hoặc làm clamd hoạt động không bình thường:

```bash
sudo /usr/local/bin/zonendpoint-sanesecurity-rollback.sh
```

**Hành vi của script:**
1. Tạo ngay lập tức file HOLD: `/opt/zonendpoint/HOLD-sanesecurity`. Khi file này tồn tại, timer và service cập nhật sẽ lập tức bỏ qua mọi lượt chạy tiếp theo.
2. Chiếm khóa chung `/run/lock/zonendpoint-clamd-deploy.lock`.
3. Khôi phục các file `.bak` từ `/var/backups/clamav-sanesecurity/` về `/var/lib/clamav/`.
4. Kích hoạt reload `clamd` và kiểm tra sức khỏe `PING` -> `PONG`.
5. Cập nhật trạng thái `forced=True` trong `/opt/zonendpoint/reload-state.json`.

### 5.2 Gỡ bỏ Hoàn toàn Feed Sanesecurity (`--purge`)
Nếu muốn loại bỏ vĩnh viễn toàn bộ cơ sở dữ liệu Sanesecurity khỏi hệ thống ClamAV:

```bash
sudo /usr/local/bin/zonendpoint-sanesecurity-rollback.sh --purge
```

Script sẽ tạo HOLD file, xóa toàn bộ 9 file Sanesecurity trong `/var/lib/clamav/`, và reload lại clamd.

### 5.3 Chính sách Quản lý Khóa & Gỡ bỏ HOLD
- **Khóa triển khai (`/run/lock/zonendpoint-clamd-deploy.lock`):** Tuyệt đối không bao giờ xóa lock bằng `rm -f`. Khóa kernel `flock` sẽ tự động giải phóng khi tiến trình nắm giữ kết thúc.
- **File HOLD (`/opt/zonendpoint/HOLD-sanesecurity`):** HOLD chỉ gỡ thủ công sau khi kiểm tra có biên bản điều tra nguyên nhân sự cố. Tuyệt đối không xóa tùy tiện trong quá trình vận hành.

---

## 6. Chính sách Triển khai Lên Server Lab (Chính sách Hợp nhất)

Triển khai lab dùng duy nhất `E:\Zonendpoint-tools\coordination\deploy\deploy-feeds.sh` (Loki, sau khi Qeen nghiệm thu); không bao giờ xóa lock; HOLD chỉ gỡ thủ công sau khi kiểm tra có biên bản.

Tuyệt đối không áp dụng quy trình cài đặt/sao chép thủ công. Mọi thao tác sao lưu, cài đặt nguyên tử và quản lý timer được điều phối tập trung qua `deploy-feeds.sh`.
