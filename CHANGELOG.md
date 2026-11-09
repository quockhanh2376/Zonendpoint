# Changelog

All notable changes to Zonendpoint are documented in this file.

## [0.2.0] - 2026-11-09

### Added
- **Server status indicator**: Icon trạng thái server (tick xanh ✓ / X đỏ ✕) hiển thị trực quan, loại bỏ flicker khi kiểm tra ngầm định kỳ và preflight.
- **SSH tunnel supervisor**: Tiến trình tunnel nền tự động nối lại (auto-reconnect) đảm bảo kết nối daemon luôn ổn định.
- **In-app server settings**: Ô cấu hình địa chỉ server cục bộ trong ứng dụng, hỗ trợ kiểm tra tính hợp lệ và ghi nhận biến môi trường `ZONENDPOINT_CLAMD`.
- **YARA-Forge Core lab feed**: Tích hợp bộ quy tắc YARA-Forge Core trên ClamAV lab daemon để tăng cường khả năng phát hiện mối đe dọa thực tế.

### Changed
- **Giới hạn stream scan 25 MiB**: Chuẩn hóa giới hạn quét stream ở mức 25 MiB phù hợp với cấu hình clamd và cảnh báo rõ ràng khi file vượt ngưỡng.
- **UI nút bấm 36px**: Chuẩn hóa kích thước nút biểu tượng (theme, server) về 36px, tối ưu không gian hiển thị và bố cục gọn gàng.
