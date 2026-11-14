# Changelog

All notable changes to Zonendpoint are documented in this file.

## [0.3.1] - 2026-11-14

### Added
- **Đo lường thời gian từng chặng quét chi tiết (P-4)**: Bổ sung cấu trúc `StageTimings` đo lường 7 chặng độc lập per-file với `Instant` không làm chậm đường chạy chính; nâng cấp công cụ benchmark `folder_bench` hỗ trợ thống kê phân vị p50, p95, max và tổng thời gian từng chặng.
- **Khóa cache gắn liền với thế hệ chữ ký ClamAV server (X-5)**: Nâng cấp schema cache SQLite lên phiên bản v3 với `sig_generation` gồm tổng số chữ ký và thời điểm nạp (`signatures:loaded_at`). Tự động làm mới cache khi database máy chủ cập nhật chữ ký mới; vô hiệu hóa sử dụng cache khi trạng thái máy chủ không hợp lệ.
- **Hiển thị huy hiệu ghi đè môi trường "Env active" cố định (X-6)**: Di chuyển huy hiệu "Env active" ra cạnh icon trạng thái server ở thanh tiêu đề, luôn hiển thị khi có biến môi trường cấu hình server (`ZONENDPOINT_CLAMD`), không phụ thuộc vào hàng Server đang đóng hay mở; tương thích hoàn hảo trên cả 2 theme Sáng/Tối và kích thước compact 240px.

### Changed
- **Tái sử dụng kết nối SQLite và gom lô giao dịch (P-2)**: Sử dụng chung một kết nối SQLite duy nhất cho toàn bộ phiên quét, kết hợp `prepare_cached` và gom ghi transaction theo lô ($\le 200$ dòng hoặc $\le 1$ giây); tự động flush khi hoàn tất hoặc hủy quét, tăng tốc ghi cache gấp 25 lần và tra cứu cache gấp 5 lần mà không làm mất verdict.
- **Băm tệp với đệm 1 MiB trong spawn_blocking và tối ưu truyền INSTREAM (P-3)**: Băm SHA-256 trong `spawn_blocking` với bộ đệm `BufReader` 1 MiB; truyền INSTREAM sang clamd daemon theo khối đọc lớn hơn ($\le 1$ MiB) với frame clamd chuẩn $\le 64$ KiB, giảm 83.6% thời gian gửi dữ liệu.
- **Đọc phản hồi clamd theo khối có kiểm thực phân khung (P-6)**: `read_nul_frame` đọc theo khối đệm thay vì từng byte đơn lẻ, giảm 99% syscalls I/O; bảo toàn nghiêm ngặt giới hạn `MAX_RESPONSE_BYTES` (4096 bytes) và từ chối mọi dữ liệu thừa sau NUL terminator.
- **Chuẩn hóa trước danh sách loại trừ Quick Scan (P-7)**: Chuẩn hóa canonical danh sách loại trừ một lần duy nhất trước khi duyệt cây thư mục, tránh lặp lại cấp phát bộ nhớ và chuẩn hóa đường dẫn cho từng tệp.
- **Giới hạn lưu trữ chi tiết ScanSummary ưu tiên mối đe dọa (S-1)**: `ScanSummary` chỉ lưu chi tiết các mục không-CLEAN (tối đa 1.000 mục), toàn bộ mục `INFECTED` luôn được bảo toàn và ưu tiên (nếu danh sách đầy sẽ loại bớt các mục không phải INFECTED), giúp tiết kiệm bộ nhớ và tránh nghẽn IPC khi quét số lượng lớn file sạch.
- **Tối ưu hóa kiểm tra Cache Hit bằng siêu dữ liệu (P-1)**: Khi cache hit, thay thế lượt đọc và băm SHA-256 thứ hai bằng so khớp lại siêu dữ liệu tệp (kích thước `size` và thời gian sửa đổi `mtime`) với snapshot lúc băm lần 1; nếu tệp bị thay đổi trong lúc quét sẽ đánh dấu `ScanStatus::Unknown`.
- **Điều tiết sự kiện tiến trình quét UI (P-5)**: Phía backend điều tiết (throttle) các sự kiện `scan-progress` cho file sạch tối đa ~10 lần/giây (100ms) để giảm tải re-render liên tục cho giao diện WebView; các phát hiện không-CLEAN (`Infected`, `Error`, `Skipped`, `Unknown`, `Cancelled`) và sự kiện kết thúc luôn được phát ngay lập tức.
- **Thu gọn bề mặt tấn công ứng dụng (X-7, S-4)**: Gỡ bỏ plugin `tauri-plugin-opener` và các quyền không sử dụng trong capabilities; loại bỏ đăng ký hai lệnh chưa kích hoạt `set_guard` và `cache_stats` khỏi Tauri command handler.

### Fixed
- **Phát hiện ClamAV engine version thay đổi giữa phiên quét (S-10)**: Kiểm tra lại `zVERSION` định kỳ mỗi 100 tệp hoặc khi gặp lỗi; nếu daemon cập nhật phiên bản hoặc nạp database mới, scanner tự động cập nhật phiên bản hoạt động và ngăn chặn ghi đè verdict vào cache dưới phiên bản cũ.
- **Tự động phục hồi cache SQLite bị hỏng và dọn dẹp TTL (S-7, S-6)**: Xử lý lỗi đọc cache như một lần cache miss và tiếp tục quét bình thường; khi file cache database bị hỏng hoặc sai schema, tự động đổi tên thành `cache.db.corrupt-YYYYMMDD-HHMMSS` (chuẩn UTC+7) rồi khởi tạo lại DB mới tinh; dọn dẹp tự động các bản ghi quá hạn TTL (7 ngày) mỗi khi khởi động ứng dụng.
- **Ghi cấu hình Server an toàn nguyên tử (S-5)**: Ghi cấu hình `config.json` nguyên tử thông qua tệp tạm cùng thư mục + `sync_all` + `rename` chống lỗi mất dữ liệu khi mất điện/crash; khi tệp cấu hình bị hỏng, trả lỗi rõ ràng lên giao diện thay vì âm thầm rơi về giá trị mặc định.

## [0.3.0] - 2026-11-13

### Added
- **Hiển thị số chữ ký bảo vệ (Cover Badge)**: Hiển thị huy hiệu "Cover: <số chữ ký>" viền cam dưới slogan, đồng bộ định kỳ chữ ký ClamAV thực tế từ lab server qua SSH tunnel, hỗ trợ phát hiện dữ liệu cũ (>2 giờ) và tương thích cả 2 giao diện sáng/tối (V-52).
- **Quét thư mục song song thích ứng (Adaptive Concurrency)**: Tự động điều chỉnh số luồng quét theo dung lượng RAM và số CPU (8 GiB $\rightarrow$ 2, 16 GiB $\rightarrow$ 3, tối đa trần 4 luồng); tự giảm luồng về 1 khi clamd quá tải và hồi phục dần sau 50 file ổn định (J-01..J-05).
- **Tích hợp nguồn cấp mã độc abuse.ch (MalwareBazaar & ThreatFox)**: Đồng bộ danh sách hash độc hại từ MalwareBazaar và ThreatFox lên ClamAV lab, tuân thủ nghiêm ngặt hạn ngạch 3 request/giờ (V-33..V-41).
- **Bộ lập lịch cập nhật tự động (Hourly Timer Scheduler)**: Tự động tải bản cập nhật hash mỗi 61 phút sau khi lượt trước kết thúc (`OnUnitInactiveSec`), luân phiên slot giữa MalwareBazaar và ThreatFox chống starvation (V-42..V-44).
- **Cache nội dung SQLite**: Tra cứu kết quả quét dựa trên hash nội dung SHA-256, metadata và engine identity, giải phóng sớm tài nguyên khi cache hit.
- **Công cụ đo kiểm hiệu năng (Folder Benchmark)**: Bổ sung benchmark nội bộ (`folder_bench`) phục vụ đo đạc tốc độ quét thư mục cục bộ (J-04).

### Changed
- **Co gọn cửa sổ kết quả quét về chiều cao nội dung (V-55)**: Khi quét xong trả về kết quả "All Good" (hoàn tất, 0 phát hiện mã độc/lỗi/không xác định/hủy bỏ), cửa sổ ứng dụng tự động co gọn về chiều cao chuẩn compact (~240px, loại bỏ hoàn toàn khoảng trống thừa dưới card All Good); khi có mối đe dọa hoặc kết quả lỗi cần xem xét, cửa sổ vẫn tự động nới cao theo nội dung thực tế (được giới hạn an toàn theo vùng làm việc `workArea` của màn hình); hỗ trợ co lại chuẩn xác khi quét lại All Good sau một lần kết quả dài.
- **Ẩn ô cấu hình Server khi kết nối OK (V-54)**: Ẩn tự động hàng cấu hình Server (ô nhập địa chỉ và nút Save) khi kết nối tới ClamAV daemon thành công hoặc trong trạng thái đang kiểm tra lần đầu; tự động hiển thị khi mất kết nối/lỗi để người dùng cấu hình; hỗ trợ bấm vào biểu tượng trạng thái kết nối để bật/tắt thủ công; giữ hiển thị khi người dùng đang thao tác trong ô nhập.

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
