# Thiết Kế Kỹ Thuật: Guard (Bảo Vệ Thời Gian Thực) — Zonendpoint

| Thuộc tính | Chi tiết |
| :--- | :--- |
| **Mã tài liệu** | `docs/design/guard.md` |
| **Nhiệm vụ** | Task V-45, V-46, V-47a, V-51 (Vic — Thiết kế Guard v1.1, Cập nhật theo rà soát Q-37B của Qeen & quyết định phê duyệt của Zon ngày 2026-11-13) |
| **Người thực hiện** | Vic (Backend Engineer) |
| **Người phê duyệt** | Zon (Project Lead) — Phê duyệt kiến trúc v1.1 & quyết định chốt ngày 2026-11-13 |
| **Ngày cập nhật** | 2026-11-13 |
| **Nhánh git** | `vic/guard-design` |
| **Phạm vi áp dụng** | Windows x64, pilot nội bộ doanh nghiệp (Internal Company Pilot) |
| **Trạng thái tính năng** | **Thiết kế kỹ thuật hoàn chỉnh thuộc phiên bản v1.1 (Approved Design Specification v4 - Post Q-37B)** — Mặc định TẮT (`OFF`). Code V-47..V-50 trên nhánh riêng, TUYỆT ĐỐI KHÔNG merge main trước khi v1.0 chốt. |

---

## Tóm Tắt Hiện Trạng & Quyết Định Phê Duyệt Của Zon (2026-11-13)

### 1. Hiện trạng mã nguồn (`src-tauri/src/watcher.rs`, `lib.rs`, `scanner.rs`)
Theo đánh giá từ Loki và phản biện Q-36, Q-37B từ Qeen:
1. **Phạm vi thô sơ**: Watcher chỉ lắng nghe 3 thư mục `Downloads`, `Desktop`, `Temp` ở chế độ `RecursiveMode::NonRecursive`. File trong thư mục con của `Downloads` bị bỏ lọt.
2. **Sự kiện thiếu sót**: Chỉ bắt `EventKind::Create(_)`. Bỏ qua `EventKind::Modify(_)` và cặp `RenameFrom`/`RenameTo`. Khi trình duyệt hoàn tất tải file và đổi tên từ `.crdownload` thành `.exe`, Watcher hoàn toàn không quét.
3. **Ổn định file bằng sleep cố định**: Đang dùng `tokio::time::sleep(300ms)`, gây lỗi `Sharing Violation` hoặc quét file dở dang khi file đang được ghi.
4. **Nguy cơ cạn kiệt tài nguyên**: Mỗi sự kiện spawn 1 OS thread + 1 runtime Tokio độc lập, không giới hạn.
5. **Nuốt kết quả phát hiện**: Kết quả quét bị vứt bỏ bằng `let _ = crate::scanner::scan_file(...)`.
6. **Thứ tự thực tế trong code `scanner.rs`**: Code hiện hành xin permit từ `ScanLimiter` TRƯỚC khi tính hash và tra cache SQLite. Thiết kế cần đồng bộ chính xác với luồng thực tế này.
7. **`Guard::stop()` hiện tại chưa đủ**: Code hiện hành trong `src-tauri/src/watcher.rs` chỉ đơn thuần gán `self.watcher = None;` để drop watcher. Thiếu cơ chế gửi cancel signal và join watcher thread + worker + arbiter với timeout 5 giây. Task V-47 bắt buộc phải sửa đổi hàm này để đảm bảo không rò rỉ task/tiến trình mồ côi.

### 2. Các quyết định kiến trúc của Zon (Cập nhật 2026-11-13 theo Q-37B)
1. **Phạm vi phiên bản v1.1**:
   - Guard chính thức thuộc phạm vi **v1.1** (sau khi P0 và bản cài v1.0 hoàn tất và chốt).
   - Quá trình triển khai mã nguồn (Tasks V-47 đến V-50) được cô lập trên các nhánh tính năng riêng (`vic/guard-pipeline`, v.v.), **TUYỆT ĐỐI KHÔNG merge vào `main` trước khi v1.0 được chốt**.
2. **File > 25 MB (`NOT_SCANNED_TOO_LARGE`)**:
   - Thiết lập trạng thái kết quả riêng biệt: **`Not scanned (> 25 MB)`** (`NOT_SCANNED_TOO_LARGE`).
   - Có bộ đếm riêng (`NotScanned: N`), **TUYỆT ĐỐI KHÔNG BAO GIỜ TÍNH LÀ `CLEAN`**.
   - Không nâng `StreamMaxLength` trên server lab (giữ nguyên trần 25 MB của ClamAV).
3. **Reparse Points, Hard Links & ADS (Anti-Evasion Policy)**:
   - **Mọi Reparse Point (Junction, Symlink)**: Bỏ qua hoàn toàn, không đi theo (nhất quán với hành vi của Quét tay).
   - **Hard Link**: Quét bình thường như file thông thường theo đường dẫn được phát hiện.
   - **Alternate Data Streams (ADS, ví dụ `file:stream`)**: Nằm ngoài phạm vi phiên bản v1; tài liệu ghi rõ giới hạn kỹ thuật và **không tuyên bố bảo vệ ADS**.
4. **Chính sách khi Mất Server / Tunnel Offline & Giới hạn Hàng đợi**:
   - Duy trì hàng đợi tối đa **10.000 file** khi server offline. **BỎ HOÀN TOÀN cơ chế cũ (giữ 100 sự kiện/xả sau 60s)**.
   - Khi tràn hàng đợi ($> 10.000$ files): Áp dụng **Drop Newest** + **Parent Directory Tracker** (lưu tối đa $\sim 100$ thư mục cha) + nút bấm **`"Scan Dropped Folders"`** trên UI.
   - Danh sách thư mục bị bỏ hiển thị **đường dẫn đầy đủ (full directory path)** trên UI để người dùng nắm rõ phạm vi cần quét bù.
   - **Tracker không bị xóa khi offline** để bảo toàn danh sách thư mục cần quét bù.
   - **Đóng ứng dụng**: Các file còn trong hàng đợi bộ nhớ khi đóng app được chấp nhận hủy (không persist hàng đợi trên đĩa).
5. **Khởi động lại ứng dụng (Persistence & Scope Reconciliation)**:
   - Guard ghi nhớ trạng thái bật/tắt của người dùng qua `config.json` (`guard_enabled: bool`), **mặc định là OFF** khi cài mới.
   - Khi khởi động lại app: **KHÔNG quét bù (no catch-up scan)** cho khoảng thời gian app đã đóng (ghi rõ giới hạn này trong phạm vi v1).
6. **Thông báo Toast, Quyền riêng tư & Quản lý Nhật ký An toàn (Privacy & Retention - Q-37B)**:
   - **Windows Toast**: CHỈ hiển thị **tên file (basename, ví dụ `setup.exe`)** + **tên mối đe dọa (ví dụ `Win.Trojan.Generic`)**. **KHÔNG hiển thị đường dẫn đầy đủ** để tránh rò rỉ username hoặc tên thư mục nhạy cảm trên màn hình desktop.
   - **Nhật ký (Log)**: Ghi vào `%LOCALAPPDATA%\ZonEndpoint\logs\guard.log`. Vì đây là máy trạm cá nhân và file log được bảo vệ chỉ tài khoản người dùng hiện tại có quyền đọc qua Standard User ACL, log **ĐƯỢC PHÉP ghi đường dẫn đầy đủ (canonical full path) + SHA-256 + tên mối đe dọa**.
   - **Lưu trữ & Quyền riêng tư**: Giữ tối đa **30 ngày** hoặc dung lượng tối đa **5 MB** (xoay vòng sang `guard.1.log` hoặc cắt tỉa bản ghi cũ). **Không cần cơ chế consent / export riêng biệt**.
7. **Công bằng & Luân phiên trên ScanLimiter (Fairness & Alternating Mode - Q-37B)**:
   - Guard sử dụng tối đa **1 permit** tại một thời điểm $\rightarrow$ Khi trần luồng $\ge 2$, Quét tay luôn còn ít nhất 1 luồng, đảm bảo Quét tay không bao giờ bị nghẽn hoàn toàn.
   - Khi trần luồng bị hạ về 1 (`concurrency limit == 1` do ClamAV backoff vì quá tải): Guard và Quét tay **LUÂN PHIÊN (Alternating Turns)**. Sau 1 lượt của Guard, permit kế tiếp BẮT BUỘC trao cho Quét tay nếu có yêu cầu Quét tay đang chờ. Quét tay có **giới hạn độ trễ chờ tối đa là 1 lượt của Guard** (tối đa chờ 1 file Guard kết thúc), triệt tiêu hoàn toàn nguy cơ Quét tay bị bỏ đói (starvation-free).
8. **Quy tắc đếm theo FILE duy nhất (Counter Semantics - Q-37B)**:
   - Tất cả các bộ đếm (`Observed`, `Excluded`, `Dropped`, `Scanned`, `Deferred`, `NotScanned`, `Error`) được tính theo **FILE (đường dẫn chuẩn hóa - canonical path)**.
   - Một file trong một cửa sổ settling chỉ được tính **tối đa 1 lần** cho toàn bộ các bộ đếm.
   - Sự kiện trùng lặp (ví dụ nhiều Modify liên tiếp) hoặc sự kiện đến đảo thứ tự từ hệ điều hành **không làm tăng số đếm**.
9. **Bảo đảm khi tắt Guard hoặc đóng app (Shutdown & Disable Invariant - Q-37B)**:
   - Khi tắt Guard hoặc đóng app: Hệ thống gửi tín hiệu cancel, bỏ watcher, join watcher thread + worker task + arbiter task với timeout tối đa **5 giây** (đồng bộ với chuẩn đóng app khi đang quét V-30 trong `src-tauri/src/lib.rs`).
   - Mọi công việc dở dang/hàng đợi chưa quét xong **TUYỆT ĐỐI KHÔNG BAO GIỜ TRỞ THÀNH `CLEAN`**.
   - Bảo đảm dọn dẹp sạch sẽ, **0 task/tiến trình mồ côi**.
   - `Guard::stop()` hiện tại trong `watcher.rs` chỉ bỏ watcher và Task V-47 bắt buộc phải sửa đổi để hiện thực đầy đủ invariant này.
10. **Lỗi không chiếm được watch thư mục (Quyền / ACL - Q-37B)**:
   - Nếu Guard không thể đăng ký theo dõi trên một thư mục giám sát (ví dụ lỗi phân quyền Win32 `ERROR_ACCESS_DENIED` 5 hoặc thư mục bị khóa ACL): Hệ thống **hiển thị trạng thái lỗi rõ ràng cho thư mục đó trên UI** (`Failed to watch folder: <path> (Access Denied)`), **TUYỆT ĐỐI KHÔNG âm thầm bỏ qua**.
11. **Chuẩn hóa chữ trên giao diện người dùng (Product Copy - Q-37B & v1.0.0-scope)**:
   - Toàn bộ chữ hiển thị trên giao diện người dùng (Product Copy) đổi sang tiếng Anh theo `v1.0.0-scope` (ví dụ: `"Scan Dropped Folders"`, `"Scan these folders"`, `"Real-time Protection"`, `"Exclude developer folders"`, `"Failed to watch folder: {0}"`). Phần giải thích tài liệu vẫn dùng tiếng Việt.

---

## Kiến Trúc Tổng Thể Guard v3

```mermaid
flowchart TD
    OS["Windows Kernel (ReadDirectoryChangesW)"] -->|Notify Crate Events| EP["1. Pipeline Sự Kiện (Event Pipeline)"]
    
    subgraph S1 ["Lọc, Chuẩn Hóa & Debounce"]
        EP -->|Create / Modify / RenameTo| EX{"Bộ lọc Loại Trừ (Exclusion Filter)"}
        EX -->|Dev Dirs: .git, node_modules, target| EXC["Tăng Excluded Count + Gợi ý Quét Tay"]
        EX -->|Reparse Point: Junction / Symlink| RP_DROP["Bỏ qua 100% (Không follow)"]
        EX -->|App Dir / Temp Browser / ADS| DROP1["Bỏ qua (Ignore)"]
        EX -->|Hợp lệ (Đích RenameTo chuẩn hóa)| DB["Sliding Debounce 500ms (Max 10.000 entries)"]
        DB --> ST["Kiểm tra Ổn định (Size + Mtime + Shared Read)"]
        ST -->|File bị xóa nhanh| DEL["Gán NOT_COMPLETED_DELETED (Lỗi, KHÔNG CLEAN)"]
        ST -->|Timeout 30s locked| DEF["Đánh dấu DEFERRED (Tăng Deferred Count, Max 1.000)"]
        DEF -->|Hẹn giờ 5 phút| RETRY{"Thử lại lần 2 sau 5 phút"}
        RETRY -->|Mở được| S2
        RETRY -->|Vẫn khóa độc quyền| ERR_LCK["Gán ERROR: Permanently Locked (KHÔNG CLEAN)"]
    end
    
    subgraph S2 ["Hàng Đợi 10.000 & Điều Tiết Luồng"]
        ST -->|Ổn định| Q{"Hàng đợi Bounded Queue: 10.000 files"}
        Q -->|Hàng đợi đầy / Bão sự kiện| STM["Drop Newest + Parent Directory Tracker (<= 100 thư mục)"]
        STM --> UIBTN["UI: Cảnh báo + Nút 'Scan Dropped Folders'"]
        Q -->|Lấy file ra| WKR["Guard Worker Task (Single Consumer)"]
        WKR --> SZ{"Kiểm tra kích thước file"}
        SZ -->|> 25 MB| NOT_SCN["Gán NOT_SCANNED_TOO_LARGE (Tăng NotScanned, KHÔNG CLEAN)"]
        SZ -->|<= 25 MB| LIM{"ScanLimiter Scheduler (2-Tier Priority / Alternating)"}
        LIM -->|Nhận 1 Permit| SCAN["Thực thi scanner::scan_file"]
        SCAN --> HASH["Hash nội dung + Tra cứu cache.db"]
        HASH -->|Cache HIT| V_HIT["Trả kết quả ngay, giải phóng permit sớm"]
        HASH -->|Cache MISS| STRM["Stream INSTREAM tới clamd (127.0.0.1:13310)"]
    end
    
    subgraph S3 ["Xử Lý Kết Quả, Toast & Mạng"]
        STRM --> RES{"Phân tích Verdict"}
        RES -->|CLEAN| C_WR["Lưu Cache CLEAN"] --> V_CLN["Tăng bộ đếm Scanned"]
        RES -->|INFECTED| C_INF["Lưu Cache INFECTED"] --> V_INF["Báo động INFECTED"]
        RES -->|Disconnect in-flight| V_ERR["Gán ERROR: Disconnect (KHÔNG CLEAN)"]
        
        V_INF --> TST_INF["Windows Toast: [Threat] in [Basename] (Không full path)"]
        V_INF --> UIBADGE["UI Badge Đỏ + Log (%LOCALAPPDATA%\\ZonEndpoint)"]
        
        LIM -.->|Offline > ~2 phút| TST_OFF["Windows Toast: Guard Paused (Server Offline)"]
        LIM -.->|Khôi phục mạng| TST_ON["Windows Toast: Guard Resumed (Protection Active)"]
    end
```

---

## 1. Phạm Vi Theo Dõi, Loại Trừ & Chính Sách Chống Né Tránh (Anti-Evasion)

### Quyết định (Decision)
1. **Thư mục giám sát**:
   - `USERPROFILE\Downloads`: Theo dõi đệ quy (**`RecursiveMode::Recursive`**).
   - `USERPROFILE\Desktop`: Theo dõi không đệ quy (**`RecursiveMode::NonRecursive`**).
   - `TEMP` (`std::env::temp_dir()`, thường là `%USERPROFILE%\AppData\Local\Temp`): Theo dõi không đệ quy (**`RecursiveMode::NonRecursive`**).
2. **Xử lý sự kiện Rename & Đổi tên thư mục (Khắc phục phản biện Q-36)**:
   - **Cặp sự kiện `RenameFrom` / `RenameTo`**:
     - Khi một file đổi tên, Guard áp dụng bộ lọc loại trừ trên **ĐƯỜNG DẪN ĐÍCH SAU CHUẨN HÓA (`RenameTo`)**.
     - **Quy tắc an toàn chống né tránh (Anti-Evasion Rule)**: Nếu một file mã độc được sinh ra trong thư mục bị loại trừ (ví dụ `Downloads\node_modules\malware.exe`), sau đó được đổi tên hoặc chuyển ra ngoài (ví dụ `Downloads\setup.exe`), sự kiện `RenameTo` trên `Downloads\setup.exe` **BẮT BUỘC PHẢI ĐƯỢC ĐƯA VÀO HÀNG ĐỢI QUÉT**.
     - Nếu file bị đổi tên từ ngoài vào thư mục bị loại trừ: Bỏ qua và hủy tiến trình theo dõi của file đó.
     - **Đổi tên cả thư mục**: Khi một thư mục được đổi tên (ví dụ `Downloads\Archive` $\rightarrow$ `Downloads\Extracted`), Guard thực hiện duyệt đệ quy (recursive walk) toàn bộ thư mục đích và đẩy tất cả các file hợp lệ vào hàng đợi.
   - **Thư mục con mới tạo**: Với cơ chế `RecursiveMode::Recursive` trên `Downloads` (sử dụng Win32 `ReadDirectoryChangesW` với cờ `bWatchSubtree = TRUE`), các file sinh ra trong thư mục con mới tạo sẽ tự động được hệ điều hành gửi sự kiện mà không bị mất dấu.
3. **Chính sách Reparse Points, Hard Links & Alternate Data Streams (Zon duyệt)**:
   - **Mọi Reparse Point (Junction, Symlink)**: **BỎ QUA HOÀN TOÀN, KHÔNG ĐI THEO** (nhất quán tuyệt đối với cơ chế Quét tay trong `scanner.rs` và `directory.rs`). Việc từ chối toàn bộ reparse point triệt tiêu hoàn toàn lỗ hổng TOCTOU (Time-of-Check to Time-of-Use) và ngăn chặn directory traversal.
   - **Hard Link**: Quét bình thường như một file thông thường theo đường dẫn được phát hiện.
   - **Alternate Data Streams (ADS, ví dụ `file.exe:hidden_payload`)**: **Nằm ngoài phạm vi phiên bản v1**. Ghi rõ giới hạn kỹ thuật: NTFS không gửi thông báo sự kiện stream độc lập và ClamAV quét theo path thông thường không tự động bóc tách stream; Zonendpoint **không tuyên bố bảo vệ ADS** trong v1.
4. **Loại trừ thư mục dự án (`.git`, `node_modules`, `target`, `.vscode`)**:
   - Mặc định loại trừ để chống bão sự kiện I/O máy trạm.
   - Hệ thống đếm chính xác số lượng file bị bỏ qua (`Excluded: N`).
   - Cấu hình trong `%LOCALAPPDATA%\ZonEndpoint\config.json`:
     ```json
     {
       "guard_exclude_dev_dirs": true
     }
     ```
   - Cảnh báo rủi ro supply-chain attack trong `node_modules` và khuyến nghị người dùng chạy **Folder Scan** thủ công sau khi `npm install`.
5. **Loại trừ nội bộ**: `%LOCALAPPDATA%\ZonEndpoint`, file tạm `.crdownload`, `.part`, `.tmp`, file hệ thống `desktop.ini`, `thumbs.db`, `$RECYCLE.BIN`.
6. **Xử lý khi không chiếm được watch một thư mục (Quyền / ACL - Q-37B)**:
   - Khi khởi động watcher hoặc bổ sung một thư mục giám sát (`Downloads`, `Desktop`, `Temp`), nếu cuộc gọi Win32 `ReadDirectoryChangesW` hoặc crate `notify` trả về lỗi không thể chiếm watch (ví dụ `io::ErrorKind::PermissionDenied` / Win32 `ERROR_ACCESS_DENIED` 5 do thư mục bị khóa ACL hoặc phân quyền tài khoản người dùng):
   - **Xử lý trên UI**: Hệ thống hiển thị trạng thái lỗi trực quan cho thư mục đó trên UI (`"Failed to watch folder: {path} (Access Denied)"`).
   - **Bảo đảm an toàn**: **TUYỆT ĐỐI KHÔNG âm thầm bỏ qua** (never silently ignore). Người dùng phải nhận thức rõ thư mục nào đang không được bảo vệ theo thời gian thực.

### Cách test (Testing Methodology)
- **Test 1.1 (Rename out of Excluded Dir)**: Tạo file `Downloads\node_modules\dummy.exe` (không quét). Đổi tên thành `Downloads\verified.exe`. Kiểm tra Guard bắt được sự kiện `RenameTo` và kích hoạt quét file `verified.exe`.
- **Test 1.2 (Reparse Point & Junction Skip)**: Tạo junction `mklink /J Downloads\sys_junc C:\Windows`. Thêm file vào `C:\Windows`. Kiểm tra Guard bỏ qua junction 100%, không duyệt quét.
- **Test 1.3 (Hard Link Scan)**: Tạo hard link `mklink /H Downloads\link.txt Downloads\orig.txt`. Kiểm tra Guard quét `link.txt` như file thông thường.
- **Test 1.4 (Exclusion Counter & Config Toggle)**: Thả 50 file vào `node_modules`. Xác nhận bộ đếm `Excluded` tăng đúng 50. Tắt config `guard_exclude_dev_dirs: false` $\rightarrow$ kiểm tra Guard quét bình thường.
- **Test 1.5 (Folder Watch Permission / ACL Failure)**: Cấu hình watcher với thư mục bị hạn chế quyền đọc/traverse (ACL denied). Kiểm tra Guard không crash, hiển thị lỗi `Failed to watch folder: ...` trên UI, không âm thầm bỏ qua.

---

## 2. Sự Kiện, Ổn Định File & Xử Lý DEFERRED / File Xóa Nhanh

### Quyết định (Decision)
1. **Sliding Debounce & Giới hạn bộ nhớ (Debounce Memory Cap)**:
   - Debounce sliding window: **500 ms** theo canonical path.
   - **Giới hạn bộ nhớ Debounce Map**: Cố định tối đa **10.000 entries**. Sử dụng cơ chế eviction (LRU): nếu map đầy 10.000 entries, entry cũ nhất được chuyển ngay sang bước kiểm tra ổn định, đảm bảo không rò rỉ bộ nhớ khi gặp bão sự kiện.
2. **Kiểm tra ổn định file (Settling Check)**:
   - Kích thước (`metadata.len()`) và thời gian sửa đổi (`metadata.modified()`) không đổi qua 2 lần kiểm tra cách nhau 500 ms.
   - Mở được file với quyền chia sẻ đọc (`FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE`).
3. **Xử lý File Tạo Rồi Xóa Nhanh (Create-then-Delete, phản biện Q-36)**:
   - Khi file được tạo rồi bị xóa ngay trước khi settling hoặc scan kịp mở:
   - Hệ điều hành trả về `io::ErrorKind::NotFound`.
   - **Xử lý an toàn**: **TUYỆT ĐỐI KHÔNG COI LÀ `CLEAN`**. Ghi nhận trạng thái: **`NOT_COMPLETED_DELETED`** (thuộc nhóm `Error / Incomplete`), tăng bộ đếm lỗi và ghi log: `[EVENT_INCOMPLETE] File was deleted before scan completed`.
4. **Quy trình xử lý file DEFERRED & Giới hạn Hàng đợi Thử lại (Zon duyệt)**:
   - File bị khóa độc quyền quá 30 giây:
     1. Gán trạng thái tạm thời **`DEFERRED`**.
     2. Tăng bộ đếm hiển thị: **`Deferred: N`** (định nghĩa chính xác: *số lượng file duy nhất hiện đang nằm trong hàng đợi chờ thử lại*).
     3. **Giới hạn bộ nhớ hàng đợi thử lại**: Tối đa **1.000 file**. Nếu hàng đợi này đầy, file bị khóa mới sẽ chuyển thẳng sang `ERROR` thay vì `DEFERRED`.
     4. Tự động thử lại **1 lần duy nhất sau khoảng 5 phút (~300 giây)**.
   - **Xử lý kết quả thử lại lần 2 (Sau 5 phút)**:
     - Nếu mở được file $\rightarrow$ Quét bình thường, giảm bộ đếm `Deferred` đi 1.
     - Nếu file vẫn tiếp tục bị khóa độc quyền $\rightarrow$ Chuyển thành trạng thái chính thức: **`ERROR`** (lý do: `"File permanently locked exclusively by another process after 5-minute retry"`), giảm bộ đếm `Deferred` đi 1. **TUYỆT ĐỐI KHÔNG GÁN `CLEAN`**.
     - Nếu file bị xóa hoặc đổi tên trước khi hết 5 phút $\rightarrow$ Giảm bộ đếm `Deferred` đi 1, ghi nhận `NOT_COMPLETED_DELETED`.
     - Nếu người dùng tắt Guard hoặc thoát ứng dụng $\rightarrow$ Hàng đợi deferred bị hủy an toàn, bộ đếm về 0, không lưu `CLEAN`.

### Cách test (Testing Methodology)
- **Test 2.1 (Fast Create-then-Delete)**: Script tạo file `Downloads\fast.tmp` rồi xóa ngay sau 50 ms. Kiểm tra Guard không crash, ghi nhận `NOT_COMPLETED_DELETED`, không bao giờ báo `CLEAN`.
- **Test 2.2 (Deferred Retry 5 Minutes & Counter Transition)**: Khóa file trong 40 giây rồi mở khóa. Sau 30 giây `Deferred` tăng 1. Sau 5 phút file được quét thành công, `Deferred` giảm về 0.
- **Test 2.3 (Permanent Locked Error)**: Khóa file liên tục trong 6 phút. Xác nhận sau lần 2 chuyển thành `ERROR`, `Deferred` về 0, không bao giờ thành `CLEAN`.

---

## 3. Quản Lý Tải, Hàng Đợi 10.000 & Điều Tiết Luồng (2-Tier ScanLimiter & Alternation)

### Quyết định (Decision)
1. **Hàng đợi dung lượng 10.000 & Bỏ cơ chế xả 60s (Zon duyệt)**:
   - Sức chứa hàng đợi: **10.000 file** (`tokio::sync::mpsc::channel(10000)`).
   - **Xóa bỏ hoàn toàn cơ chế cũ (giữ 100 sự kiện/xả sau 60s)**. Hàng đợi duy trì đầy đủ tới 10.000 file ngay cả khi server offline.
   - Khi tràn hàng đợi ($> 10.000$ files): Áp dụng **Drop Newest** + tăng biến đếm `Dropped`.
   - **Parent Directory Tracker**:
     - Lưu trữ tập hợp thư mục cha (`HashSet<PathBuf>`) có file bị drop.
     - Cố định giới hạn tối đa **100 thư mục cha**. Thư mục thứ 101 trở đi chỉ tăng số đếm `"and N more directories"`.
     - **Tracker KHÔNG bị xóa khi offline** để đảm bảo người dùng có thể quét bù bất cứ lúc nào.
     - Nút bấm **`"Scan Dropped Folders"`** hiển thị trên UI. Danh sách thư mục đi kèm hiển thị **đường dẫn đầy đủ (full directory path)**.
   - **Đóng ứng dụng**: Các file còn trong hàng đợi khi đóng app được chấp nhận hủy (in-memory queue, không persist).
2. **Quy định File > 25 MB (`NOT_SCANNED_TOO_LARGE` - Zon duyệt)**:
   - File có kích thước **> 25 MB**:
     - Gán trạng thái riêng: **`Not scanned (> 25 MB)`** (`NOT_SCANNED_TOO_LARGE`).
     - Tăng bộ đếm `NotScanned: N`.
     - **TUYỆT ĐỐI KHÔNG BAO GIỜ TÍNH LÀ `CLEAN`**.
     - Không nâng `StreamMaxLength` trên server lab.
3. **Thứ tự Permit vs Cache & Cơ chế Luân Phiên Công Bằng (Zon duyệt & Q-37B)**:
   - **Thứ tự thực thi (Khớp 100% với `scanner.rs`)**:
     - Guard worker xin permit từ `ScanLimiter` TRƯỚC khi thực hiện quét.
     - Khi giữ permit, Guard tính hash SHA-256 và tra cứu `cache.db`:
       - Nếu **Cache HIT**: Trả verdict ngay (`CLEAN` hoặc `INFECTED`) và **giải phóng permit lập tức** (chỉ giữ permit trong vài mili-giây).
       - Nếu **Cache MISS**: Giữ permit để stream dữ liệu INSTREAM tới clamd.
   - **Cơ chế 2-Tier Priority Lane & Luân Phiên (Alternating Mode)**:
     - **Trần luồng Guard**: Guard sử dụng tối đa **1 permit** tại mọi thời điểm.
     - Khi trần hệ thống $\ge 2$: Quét tay luôn còn $\ge 1$ permit $\rightarrow$ Quét tay không bao giờ bị nghẽn.
     - Khi trần hệ thống bị hạ về 1 (`concurrency == 1` do ClamAV backoff):
       - Áp dụng cơ chế **Luân phiên công bằng (Fair Alternating Turns)**:
         ```text
         State: Turn { Guard, Manual }
         ```
       - Sau khi Guard sử dụng xong 1 permit, nếu có Quét tay đang chờ, permit kế tiếp **BẮT BUỘC trao cho Quét tay**. Guard phải chờ cho tới khi Quét tay hoàn thành 1 file rồi mới được lấy lại permit.
     - **Giới hạn độ trễ cho Quét tay khi trần = 1 (Upper-bound Latency - Q-37B)**:
       - Quét tay có **độ trễ chờ tối đa là 1 lượt của Guard** (tối đa chờ đúng 1 file Guard đang in-flight hoàn thành). Ngay khi Guard giải phóng permit, permit lập tức được cấp cho Quét tay; Guard không bao giờ được phép chiếm liên tiếp permit khi có Manual waiter.
     - **Hiện thực kỹ thuật bằng Primitives an toàn**:
       - Sử dụng `struct FairPermitCoordinator` quản lý 2 hàng đợi bất đồng bộ (`tokio::sync::mpsc` cho Guard và Quét tay).
       - Arbiter task kiểm soát cấp permit, hỗ trợ cancellation an toàn (nếu một waiter bị cancel qua drop receiver, permit không bao giờ bị rò rỉ).
   - **Bảng chuyển trạng thái Arbiter khi nhiều Waiter / Hủy Waiter (Arbiter State Transitions - Q-37B)**:

| Trạng thái hiện tại | Sự kiện kích hoạt | Điều kiện / Kiểm tra | Trạng thái tiếp theo | Hành động của Arbiter |
| :--- | :--- | :--- | :--- | :--- |
| `Idle` | `GuardRequest` | Hàng đợi Manual rỗng | `GuardRunning` | Cấp permit ngay cho Guard |
| `Idle` | `ManualRequest` | - | `ManualRunning` | Cấp permit ngay cho Manual (Manual ưu tiên cao hơn) |
| `Idle` | Cả hai cùng đến | Cạnh tranh đồng thời | `ManualRunning` | Ưu tiên cấp permit cho Manual trước |
| `GuardRunning` | `ManualRequest` | - | `GuardRunning_ManualWaiting` | Đưa Manual vào đầu hàng đợi; đặt Turn kế tiếp = `Manual` |
| `GuardRunning` | `GuardRelease` | Hàng đợi Manual rỗng | `Idle` (hoặc `GuardRunning`) | Thu hồi permit hoặc cấp cho file Guard tiếp theo |
| `GuardRunning_ManualWaiting` | `GuardRelease` | Manual waiter còn kết nối | `ManualRunning` | **BẮT BUỘC trao permit cho Manual**, Turn = `Manual` |
| `GuardRunning_ManualWaiting` | `CancelWaiter(Manual)` | Manual drop receiver / hủy scan | `GuardRunning` | Xóa Manual khỏi hàng đợi chờ, không rò rỉ permit; lượt sau mở lại cho Guard |
| `GuardRunning_ManualWaiting` | `CancelWaiter(Guard)` | Guard bị drop / hủy | Không đổi (Manual vẫn đợi) | Guard giải phóng slot, chuyển sang cấp ngay cho Manual |
| `ManualRunning` | `GuardRequest` | - | `ManualRunning_GuardWaiting` | Đưa Guard vào hàng đợi; đặt Turn kế tiếp = `Guard` (nếu trần=1) |
| `ManualRunning` | `ManualRelease` | Hàng đợi Guard rỗng | `Idle` (hoặc `ManualRunning`) | Thu hồi permit hoặc cấp cho Manual file tiếp theo |
| `ManualRunning_GuardWaiting` | `ManualRelease` | Concurrency ceiling = 1, Guard còn kết nối | `GuardRunning` | **Trao permit cho Guard** theo nguyên tắc luân phiên |
| `ManualRunning_GuardWaiting` | `ManualRelease` | Concurrency ceiling $\ge 2$ | `ManualRunning` + `GuardRunning` | Cấp song song cả Manual và Guard (Guard tối đa 1 permit) |
| `ManualRunning_GuardWaiting` | `CancelWaiter(Guard)` | Guard drop receiver / tắt Guard | `ManualRunning` | Xóa Guard khỏi hàng đợi chờ, giải phóng coordinator state |
| Bất kỳ | `Drain / Shutdown` | Tắt Guard hoặc đóng app | `Draining` $\rightarrow$ `Closed` | Gửi cancel tới tất cả waiter, thu hồi permit, join với timeout 5 s |

### Cách test (Testing Methodology)
- **Test 3.1 (Event Storm 15.000 Files & Persistent Tracker)**: Tạo 15.000 file trong 120 thư mục con khi server offline. Kiểm tra:
  - Hàng đợi giữ nguyên 10.000 file. 5.000 file bị drop.
  - Tracker lưu đúng 100 thư mục cha + `"and 20 more directories"`.
  - Tracker không bị xóa khi offline. Nút "Scan Dropped Folders" hoạt động chính xác.
- **Test 3.2 (File > 25MB Not Scanned)**: Tạo file 30 MB. Kiểm tra kết quả là `NOT_SCANNED_TOO_LARGE`, bộ đếm `NotScanned` tăng 1, không bao giờ ghi nhận `CLEAN`.
- **Test 3.3 (Alternating Mode at Concurrency = 1)**: Giả lập trần concurrency = 1 (backoff). Cho Guard có 5 file chờ, đồng thời Quét tay có 5 file chờ. Kiểm tra thứ tự thực thi xen kẽ chính xác: Guard $\rightarrow$ Manual $\rightarrow$ Guard $\rightarrow$ Manual... Quét tay không bao giờ bị bỏ đói.

---

## 4. Xử Lý Kết Quả Quét, Tính Toàn Vẹn Dữ Liệu & Rớt Mạng Giữa Chừng

### Quyết định (Decision)
1. **Tính toàn vẹn dữ liệu chống Race Condition (Phản biện Q-36)**:
   - Không dựa duy nhất vào `size + mtime` để đảm bảo file không bị thay đổi.
   - Khi stream dữ liệu: Guard vừa đọc file vừa tính toán **Streaming SHA-256 Hash**.
   - Sau khi stream xong, kiểm tra lại metadata file trên đĩa:
     - Nếu metadata bị thay đổi giữa chừng hoặc hash luồng không khớp với cache $\rightarrow$ Gán **`UNKNOWN / CONTENT_MODIFIED_DURING_SCAN`**, hủy cache, không bao giờ báo `CLEAN`.
2. **Mất kết nối server giữa chừng (Server Disconnect In-Flight, phản biện Q-36)**:
   - Nếu kết nối tới clamd bị đứt khi đang stream:
   - Ghi nhận ngay lập tức verdict: **`ERROR`** (lý do: `"Server disconnected during in-flight scan"`).
   - **TUYỆT ĐỐI KHÔNG BAO GIỜ GÁN `CLEAN`**.
   - Permit được giải phóng ngay lập tức về `ScanLimiter`.
3. **Cảnh báo INFECTED & Ranh giới bảo vệ**:
   - Gửi Windows Toast (chỉ hiện basename + tên mã độc).
   - Hiển thị badge đỏ trong app.
   - **TUYỆT ĐỐI KHÔNG CÁCH LY / XÓA / DI CHUYỂN FILE Ở BẢN NÀY** (Observe & Alert Only).

### Cách test (Testing Methodology)
- **Test 4.1 (Mid-Stream Server Disconnect)**: Kill tunnel socket khi đang stream file 10 MB. Kiểm tra file kết thúc với trạng thái `ERROR`, permit được trả về ngay, không bao giờ thành `CLEAN`.
- **Test 4.2 (Race Condition Content Change)**: Ghi đè nội dung file đúng lúc Guard đang hash. Kiểm tra hệ thống bắt được lỗi, gán `UNKNOWN`, không phát sinh verdict sai lệch.

---

## 5. Xử Lý Khi Server Offline, Khởi Động Lại & Toast Trạng Thái Mạng

### Quyết định (Decision)
1. **Thông báo Windows Toast cho trạng thái mạng (Zon duyệt)**:
   - **Toast mất kết nối kéo dài**: Khi mất kết nối server liên tục **$> \sim 2$ phút (120 giây)**, gửi đúng 1 Toast:
     - Tiêu đề: `Zonendpoint Guard Paused`
     - Nội dung: `Connection to server lost. Real-time protection is suspended.`
   - **Toast khôi phục kết nối**: Khi server kết nối lại thành công, gửi đúng 1 Toast:
     - Tiêu đề: `Zonendpoint Guard Resumed`
     - Nội dung: `Connected to ZonEndpoint server. Real-time protection is active.`
   - **Chống lặp (Anti-Flapping & Cooldown)**: Không gửi toast nếu mạng chỉ rớt dưới 2 phút. Cooldown tối thiểu 60 giây giữa các lần gửi toast mạng.
2. **Khởi động lại ứng dụng & Giới hạn Scope v1 (Zon duyệt)**:
   - Guard lưu lựa chọn bật/tắt vào `config.json` (`guard_enabled`). Mặc định **OFF** khi cài mới.
   - Khi app khởi động lại: Nếu `guard_enabled == true`, Guard tự động bật lại.
   - **Giới hạn kỹ thuật v1**: **KHÔNG quét bù (No Catch-Up Scan)** cho khoảng thời gian app đã đóng để tránh làm đơ máy trạm.

### Cách test (Testing Methodology)
- **Test 5.1 (Toast Offline > 2 mins & Tolerance)**: Ngắt tunnel trong 125 giây $\rightarrow$ Đúng 1 Toast xuất hiện sau $\sim 2$ phút ($120\text{s} \pm 10\text{s}$). Bật lại tunnel $\rightarrow$ 1 Toast Resumed xuất hiện.
- **Test 5.2 (App Restart Persistence & No Catch-Up)**: Bật Guard, tắt app, tạo 5 file mới trong `Downloads`, mở lại app. Kiểm tra: Guard vẫn ở trạng thái ON, nhưng không tự ý quét 5 file cũ đã tạo lúc đóng app.

---

## 6. Thiết Kế Giao Diện Người Dùng & Định Nghĩa Chuẩn Xác Hệ Thống Bộ Đếm

### Quyết định (Decision)
1. **Định nghĩa chuẩn xác 8 bộ đếm theo FILE duy nhất (Metrics & Counters - Q-37B)**:
   Mọi bộ đếm được tính theo **FILE (đường dẫn chuẩn hóa - canonical path)**. Một file trong một cửa sổ settling chỉ tính tối đa 1 lần cho toàn bộ các bộ đếm; sự kiện trùng lặp (ví dụ nhiều Modify liên tiếp) hoặc sự kiện đến đảo thứ tự từ hệ điều hành không làm tăng số đếm:
   - **`Observed`**: Tổng số file hợp lệ tiếp nhận (theo đường dẫn chuẩn hóa, đếm theo file trong cửa sổ settling; sự kiện trùng/đảo thứ tự không làm tăng số đếm).
   - **`Queued`**: Số file hiện đang xếp hàng chờ quét trong hàng đợi (tức thời, 0 đến 10.000).
   - **`Scanned`**: Tổng số file đã quét hoàn tất (`CLEAN` hoặc `INFECTED`).
   - **`Excluded`**: Số file bị bỏ qua bởi bộ lọc loại trừ (`.git`, `node_modules`...).
   - **`Dropped`**: Tổng số file bị bỏ qua do tràn hàng đợi ($> 10.000$).
   - **`Deferred`**: Số file hiện đang chờ thử lại sau 5 phút do bị khóa (tức thời, 0 đến 1.000).
   - **`NotScanned`**: Số file bị từ chối quét do kích thước $> 25\text{ MB}$ (`NOT_SCANNED_TOO_LARGE`).
   - **`Error`**: Số file kết thúc trong lỗi (Lỗi đọc/ACL, khóa vĩnh viễn sau 5 phút, rớt mạng, file xóa dở).
2. **Giao diện người dùng (`src/App.tsx`) & Chuẩn hóa Product Copy tiếng Anh (v1.0.0-scope & Q-37B)**:
   - Toàn bộ chữ hiển thị trên giao diện người dùng (Product Copy) **bắt buộc dùng tiếng Anh 100%**:
     - Tiêu đề card: `"Real-time Protection"`
     - Trạng thái hoạt động: `"Active"`, `"Paused"`, `"Disabled"`, `"Warning"`
     - Nút bật/tắt chuẩn WCAG AA ($44\times 44\text{ px}$)
     - Settings toggle: `"Exclude developer folders (.git, node_modules, target, .vscode)"` (mặc định bật)
     - Nút tiện ích khi có file bị drop: **`"Scan Dropped Folders"`** (hoặc `"Scan these folders"`) xuất hiện khi `Dropped > 0`. Danh sách thư mục đi kèm hiển thị **đường dẫn đầy đủ (full directory path)**.
     - Cảnh báo lỗi quyền watcher: `"Failed to watch folder: {path} (Access Denied)"`
   - Card điều khiển Guard tinh gọn, tuân thủ layout chuẩn **800×240 px** (không scrollbar dọc ngoài ý muốn).

---

## 7. Chung Sống Hòa Bình Với Antivirus Cài Sẵn (Defender / Sophos Coexistence)

### Quyết định (Decision)
1. **Không xâm lấn**: Không thêm exclusion vào Defender/Sophos, không can thiệp AV chính.
2. **Xử lý mã lỗi AV khóa file**: Bắt mã lỗi Win32 `225` (`ERROR_VIRUS_INFECTED`) hoặc mã lỗi `5` (`ERROR_ACCESS_DENIED`) $\rightarrow$ Ghi nhận `SKIPPED / BLOCKED_BY_AV` hoặc `ERROR` với lý do rõ ràng: `"File access blocked by host antivirus (Defender/Sophos)"`. Không thử lại, không crash.

---

## 8. Riêng Tư, An Toàn Dữ Liệu & Quản Lý Nhật Ký (Privacy & Log Retention)

### Quyết định (Decision - Q-37B)
1. **Bảo vệ quyền riêng tư trên Windows Toast**:
   - Toast CHỈ hiển thị **tên file (basename)** (ví dụ: `malware.exe`) và **tên mã độc** (ví dụ: `Win.Trojan.Generic`).
   - **TUYỆT ĐỐI KHÔNG HIỂN THỊ ĐƯỜNG DẪN ĐẦY ĐỦ** (`C:\Users\username\...`) trên thông báo desktop để tránh lộ username hoặc tên thư mục nhạy cảm.
2. **Chính sách ghi Log an toàn & Xoay vòng (Log Retention & Rotation)**:
   - Đường dẫn: `%LOCALAPPDATA%\ZonEndpoint\logs\guard.log`.
   - **Phạm vi thông tin trong log**: Do là máy trạm cá nhân và file log được bảo vệ nghiêm ngặt chỉ tài khoản người dùng hiện tại có quyền truy cập (Standard User ACL), `guard.log` **ĐƯỢC PHÉP ghi đường dẫn đầy đủ (canonical full path) + mã hash SHA-256 + tên mối đe dọa**.
   - **Chính sách Consent / Export**: Không yêu cầu hộp thoại xác nhận consent hay tính năng export riêng biệt (dành cho máy cá nhân của chính người dùng).
   - **Giới hạn lưu trữ**: Tối đa **30 ngày** hoặc dung lượng tối đa **5 MB**.
   - Khi vượt quá 5 MB: Xoay vòng sang `guard.1.log` (ghi đè file cũ nhất). Các bản ghi quá 30 ngày được tự động cắt tỉa khi ứng dụng khởi động.
   - Log chỉ chứa siêu dữ liệu (timestamp, canonical path, size, sha256, verdict, threat name). **Không ghi nội dung nhị phân**.
   - Quyền truy cập: Standard User ACL.
3. **Hiển thị danh sách thư mục bị bỏ (Scan Dropped Folders)**:
   - Danh sách các thư mục cha bị bỏ do tràn hàng đợi hiển thị **đường dẫn đầy đủ (full directory path)** trên UI để người dùng nắm rõ phạm vi cần quét bù.

---

## 8b. Cơ Chế Tắt Guard & Đóng Ứng Dụng (Shutdown & Disable Invariant - Q-37B)

### Quyết định (Decision)
1. **Shutdown Invariant nghiêm ngặt**:
   - Khi người dùng tắt Guard trên giao diện (`guard_enabled -> false`) hoặc ứng dụng nhận sự kiện đóng cửa sổ / thoát (`WindowEvent::CloseRequested`, `RunEvent::ExitRequested`):
     1. Gửi tín hiệu hủy (`cancel`) tức thì qua watch channel / atomic token tới worker task và arbiter task.
     2. Hủy bỏ watcher (`watcher = None`), đóng các file watch handle của Windows.
     3. Thực hiện `join` đồng bộ: watcher thread + worker task + arbiter task với **timeout tối đa 5 giây** (đồng bộ hoàn toàn với cơ chế hủy quét khi đóng app của V-30 trong `src-tauri/src/lib.rs`).
   - **Bảo toàn kết quả**: Mọi công việc dở dang hoặc file đang nằm trong hàng đợi/in-flight **TUYỆT ĐỐI KHÔNG BAO GIỜ TRỞ THÀNH `CLEAN`**. Các file đang stream dở dang kết thúc với `ERROR / Incomplete` hoặc bị hủy bỏ an toàn.
   - **Không có tiến trình/task mồ côi**: Bảo đảm giải phóng 100% tài nguyên, không để lại bất kỳ async task nền hay OS thread nào chạy ngầm sau khi tắt Guard hoặc thoát ứng dụng.
2. **Yêu cầu đối với Task V-47**:
   - Hàm `Guard::stop()` hiện tại trong `src-tauri/src/watcher.rs` chỉ đơn thuần gán `self.watcher = None;` (chỉ drop watcher handle).
   - **Nhiệm vụ bắt buộc của Task V-47**: Sửa đổi toàn diện `Guard::stop()` và kiến trúc vòng đời của Guard để lưu trữ join handle của worker/arbiter và kênh cancel, hiện thực hóa đầy đủ quy trình cancel + join với timeout 5 giây theo đúng invariant nêu trên.

---

## 9. Kế Hoạch Triển Khai Chia Nhỏ (V-47 Đến V-50) & Kế Hoạch QA

### 1. Kế hoạch triển khai chia nhỏ (Phased Tasks V-47 đến V-50 — Phiên bản v1.1)

> [!IMPORTANT]
> **Quy định phạm vi v1.1**: Toàn bộ các task triển khai mã nguồn Guard (V-47 đến V-50) thuộc phạm vi **phiên bản v1.1** (sau khi P0 và v1.0 được chốt). Mã nguồn sẽ được phát triển trên các nhánh riêng biệt (`vic/guard-pipeline`, `vic/guard-arbiter`, v.v.), **TUYỆT ĐỐI KHÔNG merge vào `main` trước khi v1.0 được phê duyệt và đóng gói hoàn tất**.

| Task | Tên Nhiệm Vụ | Nội Dung Kỹ Thuật Chi Tiết | Tiêu Chuẩn Nghiệm Thu & Test |
| :--- | :--- | :--- | :--- |
| **V-47a** | **Hoàn Thiện Thiết Kế Guard v1.1** (Task Hiện Tại) | - Sửa đổi toàn diện `docs/design/guard.md` theo rà soát Q-37B và quyết định Zon ngày 2026-11-13.<br>- Xác định rõ phạm vi v1.1, log 30 ngày/5MB ghi canonical path + SHA256 + threat (không cần consent/export), 8 bộ đếm theo file duy nhất, shutdown invariant cancel+join 5s, bảng chuyển trạng thái Arbiter, lỗi quyền thư mục hiện UI, product copy tiếng Anh. | - Tài liệu được Zon nghiệm thu.<br>- Nhánh `vic/guard-design` được cập nhật và push origin. |
| **V-47** | **Pipeline Sự Kiện, Ổn Định File, Xử Lý Lỗi Quyền & Sửa `Guard::stop()`** (`watcher.rs`) | - Cấu hình `notify` đệ quy `Downloads`, non-recursive `Desktop`/`Temp`.<br>- Bắt `Create`, `Modify`, cặp `RenameFrom`/`RenameTo` (áp dụng exclusion trên destination path).<br>- Sliding Debounce 500ms (max 10.000 entries).<br>- Xử lý lỗi không chiếm được watch thư mục (hiện lỗi UI, không âm thầm bỏ qua).<br>- Settling check 30s + xử lý file xóa nhanh (`NOT_COMPLETED_DELETED`).<br>- Hàng đợi `DEFERRED` (max 1.000) tự động retry 1 lần sau 5 phút (thất bại $\rightarrow$ `ERROR`).<br>- **Sửa đổi toàn diện `Guard::stop()`**: gửi cancel signal, drop watcher, join watcher thread + worker + arbiter với timeout 5s, việc chưa xong không bao giờ thành `CLEAN`, 0 task mồ côi.<br>- Loại trừ dev dirs kèm bộ đếm `Excluded`. | - 100% unit tests `watcher.rs` pass.<br>- Test rename từ `node_modules` ra ngoài được quét thành công.<br>- Test file xóa nhanh và settling retry 5 phút pass.<br>- Test tắt Guard / đóng app sạch sẽ không task mồ côi pass. |
| **V-48** | **Hàng Đợi 10.000, 2-Tier ScanLimiter & Parent Directory Tracker** | - Triển khai `tokio::sync::mpsc::channel(10000)`.<br>- Tích hợp `ScanLimiter` 2 lớp: Guard tối đa 1 permit; khi limit=1 áp dụng luân phiên công bằng (Guard $\leftrightarrow$ Manual) với độ trễ chờ của Quét tay tối đa 1 lượt Guard.<br>- Triển khai bảng chuyển trạng thái Arbiter xử lý đa waiter và hủy waiter an toàn.<br>- Thuật toán Parent Directory Tracker (lưu tối đa 100 thư mục cha khi tràn queue, không xóa khi offline).<br>- Trạng thái `NOT_SCANNED_TOO_LARGE` cho file > 25MB.<br>- Thứ tự permit trước cache khớp `scanner.rs`. | - Test bão sự kiện 15.000 file không tràn RAM.<br>- Test luân phiên khi limit=1: Quét tay không bị bỏ đói.<br>- Tracker lưu đúng 100 thư mục cha khi offline.<br>- Test cancel waiter không rò rỉ permit. |
| **V-49** | **Xử Lý Kết Quả, Toast Privacy & Log Rotation** | - Bắn Toast INFECTED (chỉ hiện basename + threat name).<br>- Bắn Toast khi mất mạng $> \sim 2$ phút và Toast khi khôi phục (kèm cooldown chống spam).<br>- Xử lý rớt mạng giữa stream $\rightarrow$ `ERROR` và trả permit ngay.<br>- Xoay vòng log 30 ngày / tối đa 5 MB trong `%LOCALAPPDATA%\ZonEndpoint\logs\guard.log` (cho phép ghi canonical path + SHA-256 + threat name, không cần consent/export). | - Toast không lộ đường dẫn đầy đủ.<br>- Log tự động xoay vòng khi đạt 5 MB.<br>- Ngắt mạng giữa chừng không bao giờ gán `CLEAN`. |
| **V-50** | **Tích Hợp Giao Diện Người Dùng & Product Copy Chuẩn Hóa** (`App.tsx`) | - Thêm card điều khiển Guard: Nút bật/tắt (lưu config, mặc định OFF, không catch-up scan khi mở lại).<br>- Hiển thị đầy đủ 8 bộ đếm chuẩn hóa tính theo FILE (`Observed`, `Queued`, `Scanned`, `Excluded`, `Dropped`, `Deferred`, `NotScanned`, `Error`).<br>- Nút bấm `"Scan Dropped Folders"` kích hoạt Folder Scan kèm danh sách thư mục hiển thị đường dẫn đầy đủ.<br>- Hiển thị trạng thái lỗi khi watcher không chiếm được thư mục.<br>- Toàn bộ product copy dùng tiếng Anh 100% theo `v1.0.0-scope`. | - Thao tác bật/tắt mượt mà, lưu config chính xác.<br>- 8 bộ đếm hiển thị nhất quán theo file, không phóng đại.<br>- Nút Scan Dropped Folders hoạt động mượt mà.<br>- Product copy chuẩn tiếng Anh. |

---

### 2. Kế hoạch QA chi tiết cho Qeen (Q-GUARD Matrix v4)

#### Nguyên tắc an toàn môi trường QA:
- **MÔI TRƯỜNG KIỂM THỬ**: Phải thực hiện trên **Máy ảo Windows Disposable VM / Tài khoản Standard User**. **TUYỆT ĐỐI KHÔNG DÙNG THƯ MỤC THẬT CỦA ZON HAY NHÂN VIÊN**.
- **MẪU KIỂM THỬ**: **CHỈ SỬ DÙNG CHỮ KÝ TEST VÔ HẠI ĐÃ CÓ SẴN TRÊN LAB CLAMD** (`local_qa_test.ndb`, chuỗi `Zonendpoint-QA-Test-Safe-2026`). **TUYỆT ĐỐI KHÔNG DÙNG EICAR HOẶC MALWARE THẬT**. Nếu lab chưa có chữ ký này thì kiểm thử detection là `NOT RUN / BLOCKED`.

#### Ma trận Kịch bản Kiểm thử QA:

| Mã Test | Tên Kịch Bản | Môi Trường & Thao Tác | Tiêu Chuẩn Đánh Giá (Pass Criteria) & Dung Sai |
| :--- | :--- | :--- | :--- |
| **Q-GUARD-01** | Bật/Tắt & Lưu Config | Disposable VM Standard User. Bật Guard, kiểm tra config, restart app. | Guard nhớ trạng thái ON; mở lại app **không quét bù** file cũ đã tạo lúc app đóng. |
| **Q-GUARD-02** | Toast Quyền Riêng Tư | Thả file chứa chuỗi test vô hại vào `Downloads`. | Toast xuất hiện trong $\le 2\text{s}$, **chỉ hiện basename + threat name**, KHÔNG hiện full path. File gốc nguyên vẹn. |
| **Q-GUARD-03** | Đổi Tên Từ Thư Mục Loại Trừ | Tạo file test trong `Downloads\node_modules\pkg\test.exe` $\rightarrow$ đổi tên sang `Downloads\test.exe`. | Bắt sự kiện `RenameTo`, nhận diện đúng mã độc tại đường dẫn đích. |
| **Q-GUARD-04** | File Tạo Rồi Xóa Nhanh | Script tạo file và xóa ngay sau 50 ms. | Ghi nhận trạng thái `NOT_COMPLETED_DELETED` (nhóm Error), **tuyệt đối không gán `CLEAN`**. |
| **Q-GUARD-05** | File > 25 MB | Tạo file 28 MB trong `Downloads`. | Trả về `NOT_SCANNED_TOO_LARGE`, tăng `NotScanned`, **không gán `CLEAN`**, không gửi socket. |
| **Q-GUARD-06** | Bão Sự Kiện 15.000 File | Giải nén 15.000 file vào 120 thư mục con khi offline. | Hàng đợi đệm 10.000; Drop 5.000; Tracker lưu đúng 100 thư mục cha; **Tracker không bị xóa khi offline**; nút "Scan Dropped Folders" hoạt động; hiển thị full path thư mục bị bỏ. RAM peak $< 25\text{MB}$. |
| **Q-GUARD-07** | Toast Mất Mạng & Cooldown | Ngắt tunnel 125 giây rồi bật lại. | Đúng 1 Toast Paused xuất hiện sau $> \sim 2$ phút ($120\text{s} \pm 10\text{s}$); 1 Toast Resumed khi khôi phục; mạng chập chờn không spam toast. |
| **Q-GUARD-08** | Luân Phiên Công Bằng (Limit = 1) | Giả lập concurrency = 1 (backoff). Cho Guard và Quét tay cùng có việc. | Guard và Quét tay luân phiên từng lượt (Guard $\leftrightarrow$ Manual). Quét tay chờ tối đa 1 lượt Guard, không bị bỏ đói vô hạn. |
| **Q-GUARD-09** | DEFERRED & Retry 5 Phút | Khóa file độc quyền 35 giây (thành DEFERRED), sau đó mở khóa và chờ 5 phút. | Thử lại sau 5 phút ($300\text{s} \pm 15\text{s}$) quét thành công. Nếu khóa tiếp $\rightarrow$ gán `ERROR`, không bao giờ `CLEAN`. |
| **Q-GUARD-10** | Reparse Point & Hard Link | Tạo Junction tới thư mục ngoài và Hard Link trong `Downloads`. | Junction bị bỏ qua 100%; Hard Link được quét bình thường. |
| **Q-GUARD-11** | Xoay Vòng Log 30 Ngày / 5MB | Ghi file log giả lập vượt quá 5 MB trong `%LOCALAPPDATA%\ZonEndpoint`. | Log tự động xoay vòng sang `guard.1.log`, file chính tiếp tục ghi, không crash. Log chứa full canonical path + SHA256 + threat name, không hỏi consent. |
| **Q-GUARD-12** | Tắt Guard & Đóng App (Shutdown Invariant) | Kích hoạt Guard, nạp sự kiện liên tục, tắt Guard trên UI hoặc đóng cửa sổ app. | Gửi cancel, drop watcher, join thread/worker/arbiter trong $\le 5\text{s}$; việc dở dang không thành `CLEAN`; 0 task/tiến trình mồ côi. |
| **Q-GUARD-13** | Lỗi Quyền Watcher Thư Mục | Tạo thư mục watch bị từ chối quyền (Win32 `ERROR_ACCESS_DENIED`). | Guard không crash, hiển thị lỗi rõ ràng trên UI (`Failed to watch folder: <path> (Access Denied)`), không âm thầm bỏ qua. |

---

### 3. Tình Trạng Các Điểm Rà Soát Q-37B & Quyết Định Zon 2026-11-13

- **Phạm vi phiên bản**: Đã ghi nhận rõ Guard thuộc **v1.1** (sau khi v1.0 chốt). Toàn bộ code V-47..V-50 được phát triển trên nhánh riêng, không merge `main` trước khi v1.0 hoàn tất.
- **Quyền riêng tư & Log**: `guard.log` được phép ghi full canonical path + SHA-256 + threat name (máy cá nhân, Standard User ACL), lưu 30 ngày / 5 MB, không cần consent/export. Scan Dropped Folders hiển thị full directory path. Toast chỉ hiển thị basename + threat name.
- **Ngữ nghĩa bộ đếm**: Toàn bộ 8 bộ đếm tính theo **FILE (đường dẫn chuẩn hóa)** trong cửa sổ settling; sự kiện trùng/đảo thứ tự không tăng số đếm; câu mô tả `Observed` đã được sửa nhất quán theo file.
- **Shutdown & Disable Invariant**: Xác lập invariant cancel + join watcher/worker/arbiter với timeout tối đa 5 giây; việc dở dang không bao giờ thành `CLEAN`; không task mồ côi. Xác định rõ `Guard::stop()` hiện tại chỉ bỏ watcher và V-47 bắt buộc phải sửa.
- **Arbiter & Latency**: Đã bổ sung bảng chuyển trạng thái Arbiter đầy đủ cho trường hợp đa waiter và hủy waiter; xác lập trần trễ cho Quét tay khi concurrency = 1 (tối đa chờ 1 lượt Guard).
- **Lỗi quyền watch thư mục**: Xác lập cơ chế bắt lỗi Win32 `ERROR_ACCESS_DENIED` và hiển thị trạng thái lỗi rõ ràng trên UI, tuyệt đối không âm thầm bỏ qua.
- **Product Copy**: Đã chuẩn hóa 100% chuỗi UI sang tiếng Anh theo `v1.0.0-scope`.
- **Kết luận**: Toàn bộ 7 điểm kỹ thuật từ rà soát Q-37B và quyết định của Zon đã được tích hợp đầy đủ, nhất quán và chặt chẽ vào thiết kế v1.1. Tài liệu sẵn sàng bàn giao.
