# Hướng dẫn kết nối Client ZonEndpoint & Tunnel Lab (Bảo mật V-59 / X-2)

Tài liệu vận hành cho máy trạm Windows của anh Zon: quản lý SSH tunnel tự động với tài khoản giới hạn, cấu hình linh hoạt không lộ IP/user trong code (X-3), và cấu hình địa chỉ server trong ứng dụng.

---

## 1. Kiến trúc bảo mật khóa SSH Tunnel (Task V-59 / X-2)

Trước đây (Q-42), tunnel sử dụng khóa quản trị `~/.ssh/zonendpoint_lab` của tài khoản `admzon` (có toàn quyền `sudo NOPASSWD: ALL` và shell đầy đủ).

Từ bản cập nhật V-59:
- Kết nối tunnel và đồng bộ trạng thái sử dụng tài khoản hệ thống chuyên biệt **`zontunnel`** trên lab server.
- Sử dụng **hai cặp khóa Ed25519 riêng biệt**, phân quyền tối thiểu theo nguyên tắc least privilege:
  1. **Khóa Tunnel (`zonendpoint_tunnel`)**: Chỉ được phép chuyển tiếp cổng tới `127.0.0.1:3310` (ClamAV TCP). Mọi yêu cầu mở shell tương tác hoặc chạy lệnh từ xa đều bị từ chối (`command="/usr/bin/false"`).
  2. **Khóa Trạng thái (`zonendpoint_status`)**: Chỉ được phép đọc file chữ ký (`command="cat /var/lib/zonendpoint/status/signatures.json"`). Bị cấm chuyển tiếp cổng và cấm chạy bất kỳ lệnh nào khác.

---

## 2. Tạo khóa & Phân quyền trên Windows Client

Nếu cài đặt trên máy trạm mới, tạo 2 khóa Ed25519 không passphrase trong PowerShell:
```powershell
ssh-keygen -t ed25519 -N '""' -f "$env:USERPROFILE\.ssh\zonendpoint_tunnel" -C "zonendpoint_tunnel"
ssh-keygen -t ed25519 -N '""' -f "$env:USERPROFILE\.ssh\zonendpoint_status" -C "zonendpoint_status"
```

Khóa bảo mật ACL chỉ cho phép duy nhất người dùng hiện tại truy cập:
```powershell
icacls "$env:USERPROFILE\.ssh\zonendpoint_tunnel" /inheritance:r /grant:r "${env:USERNAME}:F"
icacls "$env:USERPROFILE\.ssh\zonendpoint_status" /inheritance:r /grant:r "${env:USERNAME}:F"
```

Đưa public key lên file `~zontunnel/.ssh/authorized_keys` trên server (thực hiện bởi quản trị viên lab):
```text
restrict,port-forwarding,permitopen="127.0.0.1:3310",command="/usr/bin/false" <NỘI_DUNG_zonendpoint_tunnel.pub>
restrict,command="cat /var/lib/zonendpoint/status/signatures.json" <NỘI_DUNG_zonendpoint_status.pub>
```

---

## 3. Cấu hình Tunnel (`tunnel-config.json` — X-3)

Tất cả thông tin nhạy cảm (địa chỉ IP máy chủ, tên tài khoản, đường dẫn khóa) được đưa ra file cấu hình ngoài:
`%LOCALAPPDATA%\ZonEndpoint\tunnel\tunnel-config.json`

File mẫu có sẵn tại `scripts\windows\tunnel-config.example.json`:
```json
{
  "host": "100.96.250.245",
  "user": "zontunnel",
  "tunnel_key": "%USERPROFILE%\\.ssh\\zonendpoint_tunnel",
  "status_key": "%USERPROFILE%\\.ssh\\zonendpoint_status",
  "local_port": 13310,
  "remote_port": 3310
}
```

> **Lưu ý bảo mật (V-64 / Q-43)**: Script `zon-tunnel.ps1` thực hiện kiểm tra nghiêm ngặt (strict validation) file cấu hình lúc khởi động:
> - `host`: chỉ chấp nhận IPv4, IPv6 hợp lệ hoặc hostname chuẩn RFC theo regex `^[A-Za-z0-9.-]{1,253}$`, không chứa khoảng trắng, không bắt đầu bằng dấu gạch ngang `-`.
> - `user`: tên tài khoản chuẩn POSIX chữ thường theo regex `^[a-z_][a-z0-9_-]{0,31}$`.
> - `local_port`, `remote_port`: bắt buộc là số nguyên 1–65535 (từ chối chuỗi số, số thập phân, hoặc chuỗi chèn cờ).
> - `tunnel_key`, `status_key`: đường dẫn file thực sự tồn tại trên đĩa, tuyệt đối không chứa dấu ngoặc kép `"`.
> - Tham số dòng lệnh SSH được xử lý qua hàm dựng quote đúng chuẩn Windows (`CommandLineToArgvW`) và thêm cờ kết thúc tùy chọn `--` trước host để chống tiêm cờ SSH (option injection).
> - Nếu sai: script ghi log lỗi chi tiết vào `tunnel.log`, in ra màn hình và thoát ngay lập tức (exit code 1), không fallback.
> - Có thể kiểm tra nhanh tính hợp lệ của cấu hình offline (không cần mạng) bằng switch `-ValidateOnly`:
>   ```powershell
>   powershell -ExecutionPolicy Bypass -File "scripts\windows\zon-tunnel.ps1" -ValidateOnly -AppDataRoot "C:\path\to\appdata"
>   ```

---

## 4. Cài đặt Scheduled Task tự động

### Cài đặt Task
Chạy PowerShell với quyền người dùng thông thường:
```powershell
powershell -ExecutionPolicy Bypass -File "scripts\windows\install-tunnel-task.ps1"
```
- **Hành động**:
  - Sao chép `zon-tunnel.ps1` vào `%LOCALAPPDATA%\ZonEndpoint\tunnel\zon-tunnel.ps1`.
  - Sao chép `tunnel-config.example.json` sang thư mục tunnel. Nếu chưa có `tunnel-config.json`, script sẽ tạo bản sao ban đầu để người dùng chỉnh sửa.
  - Tự dọn các file cũ ở thư mục gốc `%LOCALAPPDATA%\ZonEndpoint\`.
  - Đăng ký Scheduled Task `ZonEndpointTunnel` chạy ẩn khi đăng nhập (`At logon`).
  - **Tự động phục hồi (S-9)**: Cấu hình `RestartCount = 3` và `RestartInterval = 1 phút` trong settings của task.
- **Giám sát tự động**:
  - Tự động kết nối lại khi SSH rớt mạng với thuật toán exponential backoff (5s → 10s → 20s → 40s → tối đa 60s; reset về 5s nếu kết nối ổn định ≥ 30 giây).
  - Định kỳ ~15 phút đồng bộ trạng thái chữ ký từ server vào `%LOCALAPPDATA%\ZonEndpoint\server-status.json` chạy nền không làm gián đoạn tunnel chính.
- **Nhật ký (Log)**: Ghi vào `%LOCALAPPDATA%\ZonEndpoint\tunnel\tunnel.log`. Tự động xoay vòng sang `tunnel.log.1` khi vượt quá 256 KB. Không ghi khóa bí mật.

### Kiểm tra thử nghiệm an toàn cho QA / Developer (Bài học Q-43)
> **QUAN TRỌNG (Q-43)**: Khi chạy thử nghiệm, **bắt buộc** truyền tham số `-AppDataRoot <ThưMụcTạm>` tường minh để script ghi log và trạng thái vào thư mục tạm, tuyệt đối không được để mặc định khiến script ghi đè hoặc thêm log lỗi vào `%LOCALAPPDATA%\ZonEndpoint\tunnel\tunnel.log` của môi trường production. Ngoài ra, cấu hình thử nghiệm phải dùng cổng thử nghiệm (ví dụ `13314`), không dùng cổng production `13310`.

```powershell
# 1. Chạy bộ tự kiểm tra validation offline (không cần mạng):
powershell -ExecutionPolicy Bypass -File "scripts\windows\test-tunnel-validation.ps1"

# 2. Thử nghiệm đồng bộ trạng thái một lần (-SyncOnce) với thư mục tạm:
powershell -ExecutionPolicy Bypass -File "scripts\windows\zon-tunnel.ps1" -SyncOnce -AppDataRoot "E:\Zonendpoint-tools\artifacts\test\appdata"

# 3. Thử nghiệm giám sát tunnel với thư mục tạm (cổng thử nghiệm 13314):
powershell -ExecutionPolicy Bypass -File "scripts\windows\zon-tunnel.ps1" -AppDataRoot "E:\Zonendpoint-tools\artifacts\test\appdata"
```

### Kiểm tra ping ClamAV qua tunnel production (hoặc tunnel thử nghiệm)
```powershell
# Kết nối cổng tunnel (13310 cho production hoặc 13314 cho thử nghiệm)
$client = New-Object System.Net.Sockets.TcpClient('127.0.0.1', 13310)
$stream = $client.GetStream()
$writer = New-Object System.IO.StreamWriter($stream)
$reader = New-Object System.IO.StreamReader($stream)
$writer.Write("zPING`0")
$writer.Flush()
$reader.ReadLine()
$client.Close()
```
Kết quả mong muốn: `PONG`.

### Gỡ bỏ Task
```powershell
powershell -ExecutionPolicy Bypass -File "scripts\windows\uninstall-tunnel-task.ps1"
```

---

## 5. Cài đặt địa chỉ Server trong ứng dụng ZonEndpoint

Ứng dụng cho phép thiết lập và lưu trữ địa chỉ máy chủ ClamAV trực tiếp từ giao diện người dùng.

### Thứ tự ưu tiên cấu hình
1. **Biến môi trường `ZONENDPOINT_CLAMD`**: Nếu biến này được thiết lập và không rỗng, ứng dụng luôn ưu tiên sử dụng địa chỉ từ biến này (giao diện hiển thị huy hiệu `ENV ACTIVE`).
2. **Cài đặt trong ứng dụng**: Lưu tại `%LOCALAPPDATA%\ZonEndpoint\config.json`.
3. **Mặc định**: `127.0.0.1:13310` (cổng tunnel cục bộ).

### Quy tắc an toàn IP riêng tư (Private IP Rule)
Ứng dụng chỉ cho phép kết nối tới:
- Địa chỉ loopback IPv4/IPv6 (`127.0.0.0/8`, `::1`).
- Mạng nội bộ riêng tư RFC 1918 (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`).
- Dải mạng VPN Tailscale (`100.64.0.0/10`).
- Từ chối mọi địa chỉ Public IP trên Internet.

---

## 6. Quy trình Rollback

Nếu gặp sự cố và cần quay lại cấu hình cũ:

### Rollback phía Client Windows:
1. Dừng Scheduled Task:
   ```powershell
   powershell -ExecutionPolicy Bypass -File "scripts\windows\uninstall-tunnel-task.ps1"
   ```
2. Khôi phục lại bản `zon-tunnel.ps1` và `install-tunnel-task.ps1` từ commit trước (`908535e`).
3. Chạy lại `install-tunnel-task.ps1` cũ để tiếp tục dùng khóa `zonendpoint_lab` với user `admzon`.

### Rollback phía Lab Server:
1. Khôi phục `/etc/ssh/sshd_config` từ bản sao lưu:
   ```bash
   sudo cp -p /etc/ssh/sshd_config.bak-20261114-v59 /etc/ssh/sshd_config
   sudo sshd -t
   sudo systemctl reload ssh
   ```
2. (Tùy chọn) Xóa tài khoản `zontunnel`:
   ```bash
   sudo userdel -r zontunnel
   ```
