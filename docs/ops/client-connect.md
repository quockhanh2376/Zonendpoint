# Hướng dẫn kết nối Client ZonEndpoint & Tunnel Lab

Tài liệu vận hành ngắn gọn cho máy trạm Windows của anh Zon: quản lý SSH tunnel tự động và cấu hình địa chỉ server trong ứng dụng.

---

## 1. SSH Tunnel tự động chạy khi đăng nhập

Hệ thống cung cấp Scheduled Task chạy dưới quyền người dùng hiện tại (không cần quyền Administrator, không hiển thị cửa sổ console).

### Cài đặt Task
Chạy PowerShell với quyền người dùng thông thường:
```powershell
powershell -ExecutionPolicy Bypass -File "scripts\windows\install-tunnel-task.ps1"
```
- **Hành động**: Sao chép `zon-tunnel.ps1` vào `%LOCALAPPDATA%\ZonEndpoint\tunnel\zon-tunnel.ps1`, tự dọn file cũ ở thư mục gốc và đăng ký Scheduled Task `ZonEndpointTunnel`.
- **Kích hoạt**: Tự động chạy khi người dùng đăng nhập Windows (`At logon`), khởi chạy ngay lập tức sau khi đăng ký.
- **Giám sát tự động**: Tự động kết nối lại khi SSH rớt mạng với thuật toán backoff (5s → 10s → 20s → 40s → tối đa 60s). Nếu phiên kết nối ổn định ≥ 30 giây, backoff được đặt lại về 5s.
- **Lệnh SSH thực thi**:
  ```powershell
  ssh -i "$env:USERPROFILE\.ssh\zonendpoint_lab" -o BatchMode=yes -o ExitOnForwardFailure=yes -o ServerAliveInterval=15 -o ServerAliveCountMax=3 -N -L 127.0.0.1:13310:127.0.0.1:3310 admzon@100.96.250.245
  ```
- **Nhật ký (Log)**: Ghi ngắn gọn vào `%LOCALAPPDATA%\ZonEndpoint\tunnel\tunnel.log`. Tự động xoay vòng sang `tunnel.log.1` khi vượt quá 256 KB. Tuyệt đối không ghi thông tin nhạy cảm hoặc khóa bí mật.

### Gỡ bỏ Task
Khi cần tạm dừng hoặc gỡ bỏ tunnel tự động:
```powershell
powershell -ExecutionPolicy Bypass -File "scripts\windows\uninstall-tunnel-task.ps1"
```
Script sẽ hủy đăng ký Scheduled Task và dừng toàn bộ tiến trình giám sát PowerShell cũng như tiến trình SSH liên quan.

### Kiểm tra trạng thái Tunnel
Kiểm tra ping ClamAV qua tunnel:
```powershell
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

---

## 2. Cài đặt địa chỉ Server trong ứng dụng ZonEndpoint

Ứng dụng cho phép thiết lập và lưu trữ địa chỉ máy chủ ClamAV trực tiếp từ giao diện người dùng.

### Vị trí cấu hình & Thứ tự ưu tiên
Địa chỉ máy chủ kết nối được xác định theo thứ tự ưu tiên giảm dần:
1. **Biến môi trường `ZONENDPOINT_CLAMD`**: Nếu biến này được thiết lập và không rỗng, ứng dụng luôn ưu tiên sử dụng địa chỉ từ biến này (giao diện hiển thị huy hiệu `ENV ACTIVE`).
2. **Cài đặt đã lưu trong ứng dụng**: Lưu tại file JSON cấu hình `%LOCALAPPDATA%\ZonEndpoint\config.json`.
3. **Mặc định**: `127.0.0.1:13310` (cổng chuyển tiếp mặc định của tunnel phòng lab).

### Quy tắc an toàn IP riêng tư (Private IP Rule)
Để đảm bảo an toàn, ứng dụng chỉ cho phép kết nối tới:
- Địa chỉ loopback IPv4/IPv6 (`127.0.0.0/8`, `::1`).
- Mạng nội bộ riêng tư RFC 1918 (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`).
- Dải mạng VPN / CGNAT Tailscale (`100.64.0.0/10`, ví dụ `100.96.250.245`).
- Địa chỉ link-local và IPv6 unique local.

Ứng dụng **từ chối mọi địa chỉ IP công cộng (Public IP)** hoặc tên miền công cộng để ngăn ngừa việc rò rỉ dữ liệu hoặc phơi clamd ra Internet.

### Thao tác đổi địa chỉ trên giao diện
1. Mở ZonEndpoint.
2. Tại hàng **Server**, nhập địa chỉ IP và cổng (ví dụ: `127.0.0.1:13310` hoặc `100.96.250.245:3310`).
3. Bấm **Save**.
4. Ứng dụng kiểm tra tính hợp lệ của IP, ghi vào file cấu hình và **lập tức kiểm tra lại kết nối** với máy chủ.
   - Nếu kết nối thành công: Icon trạng thái hiển thị dấu tick xanh (✓) và thông báo `Connected to <địa chỉ>`.
   - Nếu kết nối thất bại: Icon trạng thái chuyển sang dấu chéo đỏ (✕) và thông báo lỗi tương ứng.
