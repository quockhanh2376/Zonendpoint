# Handoff: Task V-59 — Giới hạn tài khoản SSH Tunnel trên Lab (X-2 / X-3 / S-9)

- **Người thực hiện**: Vic (Developer)
- **Người nhận bàn giao / nghiệm thu**: Qeen (QA / Security Verification) & Loki (Operations)
- **Ngày**: 2026-11-14 (UTC+7)
- **Nhánh**: `vic/tunnel-restrict` từ `origin/main 908535e`
- **Worktree**: `E:\Zonendpoint-worktrees\v59-tunnel`

---

## 1. Tóm tắt thay đổi

Theo phê duyệt của anh Zon ngày 2026-11-14 giải quyết phát hiện **X-2** và **X-3** trong `optimisse.md` (được xác thực tại `artifacts/q42/qeen-verify.md`):
1. **Lab Server**:
   - Tạo tài khoản hệ thống chuyên biệt `zontunnel` (`uid=995`, `gid=983`), shell `/bin/sh`, mật khẩu khóa (`passwd -l`), không thuộc các group `sudo`, `adm`, `clamav`.
   - Cấu hình SSH authorized_keys với quyền tối thiểu:
     - Khóa tunnel: `restrict,port-forwarding,permitopen="127.0.0.1:3310",command="/usr/bin/false"`
     - Khóa status: `restrict,command="cat /var/lib/zonendpoint/status/signatures.json"`
   - Cấu hình khối `Match User zontunnel` trong `/etc/ssh/sshd_config` (bắt buộc publickey, cấm mật khẩu, cấm TTY, cấm X11, cấm agent forwarding, chỉ cho phép port forwarding cục bộ tới `127.0.0.1:3310`).
   - Sao lưu sshd_config tại `/etc/ssh/sshd_config.bak-20261114-v59`, kiểm tra `sudo sshd -t` OK và reload an toàn bằng `sudo systemctl reload ssh`.
   - Giữ nguyên toàn vẹn phiên bản sshd toàn cục, `PasswordAuthentication yes` toàn cục cho các tài khoản khác, và khóa quản trị `admzon` không bị thay đổi.
2. **Client Windows**:
   - Tạo 2 khóa Ed25519 không mật khẩu tại `%USERPROFILE%\.ssh\zonendpoint_tunnel` và `zonendpoint_status`.
   - Khóa bảo mật ACL chỉ duy nhất `%USERNAME%` có quyền (icacls inheritance disabled, only user:F).
   - Tách cấu hình nhạy cảm (IP, user, keys) ra file `%LOCALAPPDATA%\ZonEndpoint\tunnel\tunnel-config.json` (X-3), cung cấp file mẫu `tunnel-config.example.json` với dữ liệu giả lập.
   - Script `zon-tunnel.ps1`: đọc cấu hình từ JSON; báo lỗi rõ ràng nếu thiếu file; dùng `tunnel_key` cho SSH tunnel và `status_key` cho sync status; gộp logic đồng bộ trạng thái thành một hàm duy nhất dùng chung cho cả `-SyncOnce` và background runspace (S-9).
   - Script `install-tunnel-task.ps1`: bổ sung `RestartCount 3` và `RestartInterval 1 phút` (S-9); tự copy `tunnel-config.example.json` nếu máy chưa có file cấu hình.

---

## 2. Thông tin Khóa & Bản sao lưu Lab

### Fingerprints 2 khóa mới trên Client Windows:
- **Tunnel Key**:
  `256 SHA256:j6K2wVKqR8E/zg/11CNr0LC2QMnRBv/EWuqFj4ORqdU zonendpoint_tunnel (ED25519)`
- **Status Key**:
  `256 SHA256:JCvVcLUSRYvdZYCaf/YpLsoVMpMGEz9W35cJt+Sq9jo zonendpoint_status (ED25519)`

### Đường dẫn bản sao lưu trên Lab:
- File sao lưu: `/etc/ssh/sshd_config.bak-20261114-v59`
- Trạng thái kiểm tra: `sudo sshd -t` trả về mã 0 trước khi reload ssh.

---

## 3. Bằng chứng kiểm thử nghiêm ngặt (Bước 4)

Tất cả các phép thử tunnel được thực hiện trên cổng thử nghiệm **13311**, tuyệt đối không can thiệp cổng production 13310 hay Scheduled Task của anh Zon.

### Phép thử 4.1: Tunnel Key chuyển tiếp cổng 13311 -> 3310
- **Lệnh**:
  `ssh -n -N -i "$HOME\.ssh\zonendpoint_tunnel" -o BatchMode=yes -o ExitOnForwardFailure=yes -L 127.0.0.1:13311:127.0.0.1:3310 zontunnel@100.96.250.245`
- **Gửi zPING qua cổng 13311**:
  - Kết quả: `PONG`
- **Gửi zVERSION qua cổng 13311**:
  - Kết quả: `ClamAV 1.5.4/28183/Sat Nov 14 13:24:17 2026`
- **Kết luận**: Chuyển tiếp cổng ClamAV hoạt động hoàn hảo.

### Phép thử 4.2: Tunnel Key bị cấm Shell, cấm lệnh, cấm forward cổng khác
- **Thử mở Shell tương tác/batch**:
  - Lệnh: `ssh -n -o BatchMode=yes -o ConnectTimeout=15 -i "$HOME\.ssh\zonendpoint_tunnel" zontunnel@100.96.250.245`
  - Kết quả: Thoát ngay lập tức với mã lỗi `1` (thực thi `/usr/bin/false`).
- **Thử chạy lệnh từ xa (`id`)**:
  - Lệnh: `ssh -n -o BatchMode=yes -o ConnectTimeout=15 -i "$HOME\.ssh\zonendpoint_tunnel" zontunnel@100.96.250.245 "id"`
  - Kết quả: Thoát ngay lập tức với mã lỗi `1` (thực thi `/usr/bin/false`).
- **Thử forward tới cổng khác (cổng SSH 22)**:
  - Lệnh: `ssh -n -N -o BatchMode=yes -o ConnectTimeout=15 -i "$HOME\.ssh\zonendpoint_tunnel" -L 127.0.0.1:13311:127.0.0.1:22 zontunnel@100.96.250.245`
  - Kết quả: Máy chủ từ chối mở channel: `channel 1: open failed: administratively prohibited: open failed`.
- **Kết luận**: Khóa tunnel không thể lấy shell, không thể chạy lệnh, và không thể mở bất kỳ cổng nào khác ngoài 3310.

### Phép thử 4.3: Status Key bị cấm forward, chỉ đọc được signatures.json
- **Lấy trạng thái chữ ký mặc định**:
  - Lệnh: `ssh -n -o BatchMode=yes -o ConnectTimeout=15 -i "$HOME\.ssh\zonendpoint_status" zontunnel@100.96.250.245`
  - Kết quả: Trả về JSON hợp lệ:
    `{ "signatures": 8616894, "loaded_at": "2026-11-14T18:55:43+07:00", "engine": "ClamAV 1.5.4/28183" }`
- **Thử chạy lệnh khác (`id`)**:
  - Lệnh: `ssh -n -o BatchMode=yes -o ConnectTimeout=15 -i "$HOME\.ssh\zonendpoint_status" zontunnel@100.96.250.245 "id"`
  - Kết quả: Máy chủ vẫn chỉ chạy lệnh cố định và trả về JSON `signatures.json`, bỏ qua lệnh `id`.
- **Thử chuyển tiếp cổng với status key**:
  - Lệnh: `ssh -n -N -o BatchMode=yes -o ConnectTimeout=15 -i "$HOME\.ssh\zonendpoint_status" -L 127.0.0.1:13311:127.0.0.1:3310 zontunnel@100.96.250.245`
  - Kết quả: Máy chủ từ chối kết nối channel: `channel 1: open failed: administratively prohibited: open failed`.
- **Kết luận**: Khóa status chỉ đọc được duy nhất file trạng thái, hoàn toàn không forward được cổng.

### Phép thử 4.4: Kiểm tra đặc quyền zontunnel
- **Kiểm tra sudoers**:
  - Lệnh: `sudo -l -U zontunnel`
  - Kết quả: `User zontunnel is not allowed to run sudo on udevlevel73.`
- **Kiểm tra đăng nhập bằng mật khẩu**:
  - Lệnh: `ssh -o PreferredAuthentications=password -o PubkeyAuthentication=no zontunnel@100.96.250.245`
  - Kết quả: `Permission denied (publickey)`.
- **Kiểm tra đăng nhập admzon bằng khóa cũ**:
  - Lệnh: `ssh -n -o BatchMode=yes -o ConnectTimeout=15 -i "$HOME\.ssh\zonendpoint_lab" admzon@100.96.250.245 "id"`
  - Kết quả: Thành công (`uid=1000(admzon) gid=1000(admzon) groups=1000(admzon),4(adm),27(sudo)...`).
- **Kết luận**: zontunnel hoàn toàn không có quyền root/sudo; không thể đăng nhập bằng mật khẩu; các tài khoản quản trị khác hoạt động bình thường.

---

## 4. Kiểm thử Script Client với thư mục tạm (Bước 5)
- Chạy `zon-tunnel.ps1 -SyncOnce` với `LOCALAPPDATA` tạm:
  - Exit code: `0`
  - File `%LOCALAPPDATA%\ZonEndpoint\server-status.json` được tạo và ghi nguyên tử với định dạng chuẩn UTC+7.
- Chạy tiến trình nền `zon-tunnel.ps1` với `LOCALAPPDATA` tạm trên cổng 13311:
  - Tunnel kết nối thành công, log ghi nhận kết nối.
  - Background runspace tự động kích hoạt đồng bộ trạng thái chữ ký thành công.
  - Gửi `zPING` tới `127.0.0.1:13311` nhận phản hồi `PONG`.
  - Tiến trình thử nghiệm đã được dừng hoàn toàn, cổng 13311 đã được đóng sạch sẽ.

---

## 5. Quy trình Rollback

### Rollback Lab Server:
```bash
# 1. Khôi phục sshd_config từ bản sao lưu
sudo cp -p /etc/ssh/sshd_config.bak-20261114-v59 /etc/ssh/sshd_config
sudo sshd -t
sudo systemctl reload ssh

# 2. Xóa tài khoản zontunnel
sudo userdel -r zontunnel
```

### Rollback Client Windows:
1. Gỡ bỏ task hiện tại:
   `powershell -ExecutionPolicy Bypass -File "scripts\windows\uninstall-tunnel-task.ps1"`
2. Cài lại script từ commit `908535e` sử dụng `zonendpoint_lab`.
