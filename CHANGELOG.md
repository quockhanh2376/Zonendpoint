# Changelog

All notable changes to Zonendpoint are documented in this file.

## [0.3.0] - 2026-11-13

### Added
- **Hiển thị số chữ ký bảo vệ (Cover Badge)**: Hiển thị huy hiệu "Cover: <số chữ ký>" viền cam dưới slogan, đồng bộ định kỳ chữ ký ClamAV thực tế từ lab server qua SSH tunnel, hỗ trợ phát hiện dữ liệu cũ (>2 giờ) và tương thích cả 2 giao diện sáng/tối (V-52).
- **Quét thư mục song song thích ứng (Adaptive Concurrency)**: Tự động điều chỉnh số luồng quét theo dung lượng RAM và số CPU (8 GiB $\rightarrow$ 2, 16 GiB $\rightarrow$ 3, tối đa trần 4 luồng); tự giảm luồng về 1 khi clamd quá tải và hồi phục dần sau 50 file ổn định (J-01..J-05).
- **Tích hợp nguồn cấp mã độc abuse.ch (MalwareBazaar & ThreatFox)**: Đồng bộ danh sách hash độc hại từ MalwareBazaar và ThreatFox lên ClamAV lab, tuân thủ nghiêm ngặt hạn ngạch 3 request/giờ (V-33..V-41).
- **Bộ lập lịch cập nhật tự động (Hourly Timer Scheduler)**: Tự động tải bản cập nhật hash mỗi 61 phút sau khi lượt trước kết thúc (`OnUnitInactiveSec`), luân phiên slot giữa MalwareBazaar và ThreatFox chống starvation (V-42..V-44).
- **Cache nội dung SQLite**: Tra cứu kết quả quét dựa trên hash nội dung SHA-256, metadata và engine identity, giải phóng sớm tài nguyên khi cache hit.
- **Công cụ đo kiểm hiệu năng (Folder Benchmark)**: Bổ sung benchmark nội bộ (`folder_bench`) phục vụ đo đạc tốc độ quét thư mục cục bộ (J-04).

### Fixed
- **Sửa lỗi hiển thị ô Cover do UTF-8 BOM & siết chặt xác thực dữ liệu (V-53)**: Xử lý triệt để lỗi app hiện "Cover: —" khi file status có UTF-8 BOM; tunnel chuyển sang ghi UTF-8 không BOM; bổ sung trường `fetched_at_utc` để tính stale chính xác theo lần sync máy trạm (>2h); siết chặt kiểm tra số nguyên signatures JSON và định dạng thời gian ISO-8601 UTC chuẩn xác.
- **Đóng ứng dụng khi đang quét (Graceful Window Close)**: Hủy quét an toàn và ẩn cửa sổ ngay lập tức khi người dùng đóng app, đợi tối đa 5 giây cho worker hoàn tất, bảo toàn 100% bằng chứng phát hiện mối đe dọa (INFECTED) và không để lại tiến trình mồ côi (V-30, V-30M).
- **Loại trừ lỗi quyền DriverData**: Loại trừ `%SystemRoot%\System32\drivers\DriverData` trong Quick Scan trên Windows tiêu chuẩn, tránh lỗi `Access is denied` không mong muốn.

## [0.2.1] - 2026-11-10

### Added
- **Version label**: Hiện version dưới nút trạng thái.
- **Lab feed**: YARA Forge tự cập nhật hằng đêm, gỡ luật test ZonQA khỏi clamd.

### Fixed
- **Quét file lớn**: Quét file lớn không còn "read server response timed out" (chờ clamd tới 120s, Cancel vẫn dừng ngay).

## [0.2.0] - 2026-11-09

### Added
- **Server status indicator**: Icon trạng thái server (tick xanh ✓ / X đỏ ✕) hiển thị trực quan, loại bỏ flicker khi kiểm tra ngầm định kỳ và preflight.
- **SSH tunnel supervisor**: Tiến trình tunnel nền tự động nối lại (auto-reconnect) đảm bảo kết nối daemon luôn ổn định.
- **In-app server settings**: Ô cấu hình địa chỉ server cục bộ trong ứng dụng, hỗ trợ kiểm tra tính hợp lệ và ghi nhận biến môi trường `ZONENDPOINT_CLAMD`.
- **YARA-Forge Core lab feed**: Tích hợp bộ quy tắc YARA-Forge Core trên ClamAV lab daemon để tăng cường khả năng phát hiện mối đe dọa thực tế.

### Changed
- **Giới hạn stream scan 25 MiB**: Chuẩn hóa giới hạn quét stream ở mức 25 MiB phù hợp với cấu hình clamd và cảnh báo rõ ràng khi file vượt ngưỡng.
- **UI nút bấm 36px**: Chuẩn hóa kích thước nút biểu tượng (theme, server) về 36px, tối ưu không gian hiển thị và bố cục gọn gàng.
