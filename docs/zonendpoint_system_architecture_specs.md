# ZONENDPOINT - TECHNICAL SPECIFICATIONS & ARCHITECTURE DESIGN

## 1. Tổng Quan Kiến Trúc (Executive Summary)

**Zonendpoint** là giải pháp phần mềm phòng chống mã độc và kiểm soát an ninh điểm cuối (Endpoint Security) tối ưu cho môi trường máy trạm **Windows 11** với tài nguyên hạn chế (**8GB RAM**).

Hệ thống hoạt động theo mô hình **Hybrid Tiering (Kiến Trúc Lai)** kết hợp **Thin Client + Local SQLite Acceleration + Remote Engine**:

* **Client (Laptop Windows 11 - Văn phòng hoặc Từ xa qua VPN)**: Sử dụng **Tauri (Rust + TypeScript/React/Svelte)** kết hợp cơ sở dữ liệu nhúng **SQLite (WAL Mode)**. Client chịu trách nhiệm:
  * Giám sát tệp tin hệ thống thời gian thực (**User-Space Real-Time Guard** qua `ReadDirectoryChangesW`).
  * Tra cứu tức thì kết quả quét qua bộ nhớ đệm 2 lớp ($< 1\text{ ms}$).
  * Tra cứu mã độc nhanh qua danh bạ băm nhị phân cục bộ (**Local Known-Bad/Good Hash DB** dạng `BLOB(32)`).
  * Điều phối stream mạng và hiển thị giao diện người dùng.
* **Backend Server (Ubuntu Linux Mini Server đặt tại Công ty/DC)**: Chạy **ClamAV Daemon (`clamd`)**, **Freshclam** và **Fangfrisch**. Đảm nhiệm:
  * Nạp signature database chính thức từ Cisco Talos kết hợp các feed mở rộng của bên thứ ba (SecuriteInfo, YARA Rules) vào RAM.
  * Phục vụ quét giải nén/so khớp chuỗi mã độc phức tạp (heuristics, regex, bytecode).
  * Mở endpoint cập nhật danh bạ băm (Known Hashes SQLite sync) cho các máy trạm từ xa qua mạng riêng ảo (VPN).

---

## 2. Kết Nối Mạng Từ Xa & Cơ Chế Cập Nhật Qua VPN

### 2.1. Bản chất mô hình: Client ở nhà, Server đặt tại công ty

Khi laptop Windows 11 ở ngoài mạng LAN văn phòng (ví dụ: làm việc tại nhà, quán cafe), máy trạm kết nối về máy chủ tại công ty thông qua **VPN (Mesh VPN như Tailscale/WireGuard hoặc Site-to-Site/OpenVPN)**.

Trong mô hình này:
1. **Client không cần tải database virus nặng 1.5GB – 2GB về máy**:
   * Toàn bộ database chữ ký mẫu (signature) khổng lồ của ClamAV, SecuriteInfo và YARA vẫn **thường trú trên RAM của máy chủ công ty**.
   * Khi laptop quét file, nó chỉ gửi stream dữ liệu file qua đường hầm mã hóa VPN (`100.x.y.z:3310`) đến máy chủ để quét.
2. **"Update Database cho máy ở nhà" thực chất là cập nhật cái gì?**:
   * Máy ở nhà chỉ cần đồng bộ **Danh bạ Hash nhẹ (Known Bad/Good Hashes)** vào file `cache.db` (SQLite cục bộ) với dung lượng chỉ vài MB đến vài chục MB.
   * Quá trình đồng bộ này diễn ra cực nhanh qua VPN (REST API hoặc tải tệp delta SQLite nén zstd), không làm nghẽn đường truyền mạng gia đình và không tốn RAM máy laptop.

```text
[ Laptop ở Nhà (Win 11) ] 
       │ 
       ▼ (Đường hầm VPN: WireGuard / Tailscale)
 [ Router / Firewall Cty ] 
       │ 
       ▼ (IP Nội bộ: 10.x.x.x:3310 & Port 8080 Sync)
[ Ubuntu Mini Server @ Office ] 
  ├── clamd (RAM 1.5GB - 2GB: Cisco Talos + SecuriteInfo + YARA)
  └── sync-service (Phục vụ đồng bộ delta Known-Hashes SQLite)
```

---

## 3. Module Bảo Vệ Thời Gian Thực (User-Space Real-Time Guard)

### 3.1. Phân biệt Real-Time Guard User-Space và Kernel Minifilter

ClamAV nguyên bản **không có tính năng bảo vệ thời gian thực trên Windows** vì thiếu Windows Minifilter Driver (`fltmgr.sys`). Zonendpoint giải quyết bài toán này bằng cách triển khai **Real-Time Guard ở tầng User-Space** thông qua Rust:

| Tiêu chí | Kernel Minifilter Driver (Commercial AV) | Rust User-Space Guard (Zonendpoint) |
| :--- | :--- | :--- |
| **Tầng can thiệp** | Kernel Ring 0 (`fltmgr.sys`) | User-Space Ring 3 (Windows API `ReadDirectoryChangesW`) |
| **Thời điểm chặn** | Trước khi I/O hoàn tất (Chặn click mở file) | Ngay sau khi file vừa xuất hiện / đóng luồng ghi |
| **Độ an toàn hệ thống** | Rủi ro gây lỗi màn hình xanh (BSOD) | Tuyệt đối an toàn, không thể làm crash Windows |
| **Yêu cầu triển khai** | Cần chứng chỉ EV Code Signing của Microsoft | Chạy dạng tiến trình nền chuẩn với quyền User/Admin |

### 3.2. Kiến trúc Real-Time Engine trong Rust

Engine giám sát thời gian thực hoạt động dựa trên crate `notify` kết hợp với hàng đợi bất đồng bộ (`tokio::sync::mpsc`):

```text
[ File System Event (Downloads/Desktop/Temp) ]
                    │
                    ▼ (ReadDirectoryChangesW)
        [ Notify Watcher Thread ]
                    │ (Debounce Filter 300ms)
                    ▼
     [ Tokio MPSC Channel Queue ]
                    │
                    ▼
      [ On-Create Scanning Pipeline ]
         ├── 1. Kiểm tra File Lock (Chờ ghi xong)
         ├── 2. Bỏ qua file tạm (.tmp, .crdownload, .part)
         ├── 3. Tra cứu Fast SQLite Cache
         ├── 4. Băm SHA-256 & kiểm tra Local Blacklist
         └── 5. Stream quét tới ClamAV Server qua Socket
                    │
           ┌────────┴────────┐
           ▼ (Clean)         ▼ (Threat Found)
      [ Ghi Cache ]   [ AUTO-QUARANTINE ]
                         ├── Đổi extension -> .infected
                         ├── Thu hồi quyền NTFS ACL
                         ├── Di chuyển vào ProgramData\Quarantine
                         └── Windows Toast Notification (Tauri UI)
```

### 3.3. Xử lý bài toán File Lock & Trình duyệt tải dở

Khi trình duyệt (Chrome, Edge) hoặc app tải file:
1. File ban đầu có tên tạm: `setup.exe.crdownload` hoặc `tmpA12B.tmp`. Real-time guard sẽ **bỏ qua** các extension tạm thời này.
2. Khi tải hoàn tất, hệ điều hành phát sinh sự kiện **`Rename`** thành tên file chính thức (`setup.exe`).
3. Guard tự động áp dụng vòng lặp kiểm tra trạng thái mở khóa độc quyền (Exclusive Access) với cơ chế Backoff ngắn (50ms – 100ms) để đảm bảo file đã được đóng hoàn toàn trước khi bắt đầu stream quét.

---

## 4. Xử Lý Bài Toán Quét Hàng Loạt File Nhỏ (Small-File Bottleneck)

### 4.1. Bản chất vấn đề
Khi quét thư mục chứa hàng chục nghìn file nhỏ (như `node_modules`, `AppData`, `.git`):
* Băng thông mạng không bị nghẽn, nhưng **Round-Trip Time (RTT)** và chi phí thiết lập luồng stream mạng sẽ khiến tốc độ sụt giảm nghiêm trọng nếu xử lý tuần tự.
* Đọc ngẫu nhiên hàng loạt file nhỏ làm tăng vọt chỉ số IOPS của SSD.

### 4.2. Chiến lược 3 lớp tăng tốc độ quét file nhỏ
1. **Lớp 1: Bỏ qua theo Whitelist Extension (Pre-filter)**:
   * Loại bỏ ngay các định dạng dữ liệu tĩnh an toàn: `.txt`, `.log`, `.md`, `.jpg`, `.png`, `.mp4`, `.svg`, `.json`...
   * Chỉ đưa vào pipeline quét các file nhị phân, thư viện liên kết và script thực thi: `.exe`, `.dll`, `.sys`, `.bat`, `.cmd`, `.ps1`, `.vbs`, `.js`, `.zip`, `.rar`, `.7z`, `.iso`.
2. **Lớp 2: Quét hàng loạt siêu tốc qua SQLite Cache**:
   * Kiểm tra `(path, size, mtime)` theo từng batch trong transaction SQLite: 10.000 file có thể được kiểm tra cache chỉ trong **~150ms – 200ms**. Hầu hết các file mã nguồn cũ không thay đổi sẽ bị loại ngay từ vòng này.
3. **Lớp 3: Connection Pool & Song song hóa (Worker Pool)**:
   * Giữ kết nối TCP Socket liên tục (Persistent TCP Stream) thay vì mở/đóng kết nối cho từng file nhỏ.
   * Sử dụng `tokio::sync::Semaphore` chia luồng từ 4 đến 8 worker quét đồng thời.

---

## 5. Hệ Thống Chữ Ký Mở Rộng & Nguồn Dữ Liệu Đe Dọa (Threat Intel)

Nhằm nâng tỷ lệ bao phủ của ClamAV từ mức cơ bản (~60% – 75%) lên trên **85% – 90%**, hệ thống tích hợp các nguồn mở rộng tập trung tại máy chủ Ubuntu:

| Nguồn chữ ký / Dữ liệu | Loại hình | Chi phí bản quyền | Cách tích hợp & Giới hạn thiết bị |
| :--- | :--- | :--- | :--- |
| **Cisco Talos Core** | Chữ ký chuẩn (`main.cvd`, `daily.cvd`) | Miễn phí | Tích hợp sẵn trong tiến trình `freshclam`. |
| **SecuriteInfo** | Heuristic, 0-day, Macro, Malicious Hashes | Miễn phí (Gói Cá nhân) | Đăng ký 1 tài khoản Free lấy Account ID. Cấu hình duy nhất trên Server qua Fangfrisch. **Quét không giới hạn máy trạm** vì request tải feed chỉ xuất phát từ 1 IP Server. |
| **YARA Rules Project** | Quy tắc nhận diện logic mã độc, script độc hại | Miễn phí (Open Source) | Đặt trực tiếp file `.yar` vào `/var/lib/clamav/`. Engine YARA nhúng sẵn trong `clamd` tự động biên dịch vào RAM. |
| **Abuse.ch MalwareBazaar** | Danh bạ Hash SHA256 mã độc mới phát hiện | Miễn phí (Open API) | Script chạy trên server định kỳ kéo hash mới về nạp vào bảng `known_bad_hashes` trên SQLite. |
| **VirusTotal Public API** | Tra cứu Threat Intelligence đa engine | Miễn phí (Giới hạn 4 req/min, 500 req/day) | Dùng làm tầng tra cứu dự phòng cuối cùng khi gặp file khả nghi chưa có chữ ký. |

### 5.1. Khai thác Engine YARA bên trong ClamAV
* Tạo thư mục quy tắc: `/var/lib/clamav/rules.d/`.
* Mọi tệp có phần mở rộng `.yar` hoặc `.yara` đặt trong thư mục database của ClamAV sẽ được tự động biên dịch khi khởi động service.
* Kết quả quét khi khớp rule YARA sẽ có định dạng: `stream: YARA.<Rule_Name> FOUND`.

---

## 6. Phân Tích Kỹ Thuật: SQLite Local & Nén Database Chữ Ký Virus

### 6.1. Đánh giá tính khả thi khi nén và đưa Signature DB vào SQLite trên máy trạm
1. **Bản chất của thuật toán Pattern Matching**:
   * ClamAV phát hiện virus bằng chuỗi hex xen kẽ wildcard, regular expressions và logic heuristic. Thuật toán cốt lõi sử dụng cây tìm kiếm chuỗi song song (**Aho-Corasick Automata**) và bảng dịch chuyển Boyer-Moore.
   * Để đạt tốc độ quét hàng trăm MB/s, các cây tìm kiếm này **bắt buộc phải nằm trên RAM ở dạng đã giải nén hoàn toàn**.
2. **Nếu nén DB trong RAM**:
   * CPU sẽ phải giải nén liên tục từng block chữ ký khi duyệt qua các byte của file. Điều này khiến **CPU laptop bị quá tải (100% CPU)** và tốc độ quét sụt giảm nghiêm trọng.
3. **Nếu chuyển toàn bộ Rules/Signatures vào bảng SQLite**:
   * SQLite không thể tối ưu cho việc tìm kiếm chuỗi con mờ (fuzzy sub-string matching) ở mức thanh ghi vi xử lý (SIMD/AVX2). Quét một file sẽ sinh ra hàng nghìn lệnh `SELECT`, gây nghẽn tại Disk I/O.

### 6.2. Mô hình phân tầng dữ liệu (Hybrid Storage Model)

| Loại dữ liệu | Vị trí lưu trữ | Cấu trúc & Tối ưu | Mức chiếm RAM trên Win 11 | Tốc độ phản hồi |
| :--- | :--- | :--- | :--- | :--- |
| **Local Hash Cache** | **SQLite (Windows 11)** | Index B-Tree `(Path, Size, Mtime)` | **~10 MB – 15 MB** | $< 0.1\text{ ms}$ |
| **Known Hashes (Black/White)** | **SQLite (Windows 11)** | `BLOB(32)` nhị phân nguyên bản | **~15 MB – 25 MB** | $< 1\text{ ms}$ |
| **Full Signature Engine** | **RAM (Ubuntu Server)** | Aho-Corasick Trie trên RAM server | **0 MB** (trên Laptop) | Streaming qua Socket |

---

## 7. Thiết Kế Cơ Sở Dữ Liệu SQLite Cục Bộ Tối Ưu

Tệp database lưu trữ tại `%LOCALAPPDATA%\Zonendpoint\cache.db`. Sử dụng kiểu `BLOB(32)` để tiết kiệm 50% dung lượng đĩa và tối ưu hóa so sánh nhị phân:

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA temp_store = MEMORY;
PRAGMA cache_size = -8000; -- Giới hạn RAM SQLite Cache ở mức ~8MB

-- 1. Bảng lưu trữ Cache kết quả quét các file trên máy trạm
CREATE TABLE IF NOT EXISTS scan_cache (
    file_path TEXT PRIMARY KEY,
    file_size INTEGER NOT NULL,
    modified_time INTEGER NOT NULL,
    sha256_raw BLOB NOT NULL,       -- 32 bytes binary
    status TEXT NOT NULL,           -- 'CLEAN', 'INFECTED', 'WHITELISTED'
    virus_name TEXT,
    scanned_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_fast_check ON scan_cache(file_path, file_size, modified_time);
CREATE INDEX IF NOT EXISTS idx_sha256_blob ON scan_cache(sha256_raw);

-- 2. Bảng chữ ký hash mã độc đã biết (Known Bad Hashes - Tải định kỳ từ Server qua VPN)
CREATE TABLE IF NOT EXISTS known_bad_hashes (
    hash_sha256 BLOB PRIMARY KEY,   -- 32 bytes SHA-256
    threat_name TEXT NOT NULL,
    severity INTEGER DEFAULT 1
);

-- 3. Bảng Whitelist hash an toàn (Known Good / System Files)
CREATE TABLE IF NOT EXISTS known_good_hashes (
    hash_sha256 BLOB PRIMARY KEY,   -- 32 bytes SHA-256
    description TEXT
);
```

---

## 8. Bảng Phân Bổ Tài Nguyên Chi Tiết (8GB RAM Windows 11)

| Thành phần | Công nghệ / Process | Mức RAM trung bình | Mức CPU trung bình | Ghi chú |
| :--- | :--- | :--- | :--- | :--- |
| **Windows 11 Base** | OS Kernel, Background Services | 3.2 GB – 4.0 GB | 1% – 3% | Hệ điều hành lúc nghỉ |
| **Zonendpoint GUI** | Tauri / Webview2 (TypeScript) | 35 MB – 55 MB | < 0.5% | Dashboard, Thông báo trạng thái |
| **Zonendpoint Core** | Rust Core Service + Real-time Watcher | 18 MB – 30 MB | 1% – 3% | ReadDirectoryChangesW, Watchdog |
| **SQLite Cache Engine** | Embedded SQLite (WAL, Page Cache 8MB) | 8 MB – 16 MB | < 1% | Lưu trữ metadata & SHA-256 |
| **Quá trình quét tích cực** | Stream Chunk 64KB qua LAN/VPN | + 10 MB – 20 MB | 5% – 12% | Đọc SSD tuần tự, không tràn RAM |
| **Tổng Zonendpoint Client** | **Toàn bộ ứng dụng** | **~75 MB – 125 MB** | **< 15%** | **Cực nhẹ, an toàn tuyệt đối cho 8GB RAM** |
| *So sánh: Chạy ClamAV Local* | *`clamd.exe` chạy trực tiếp* | *1.4 GB – 1.8 GB* | *20% – 45%* | *Gây áp lực lớn, dễ tràn Pagefile* |

---

## 9. Tối Ưu Hóa Daemon Phục Vụ Nhiều Máy Quét Đồng Thời (Concurrency Tuning)

### 9.1. Công thức cân bằng tài nguyên trên Server
* Tỷ lệ hàng đợi an toàn: $\text{MaxQueue} \ge 2 \times \text{MaxThreads}$ (khuyến nghị $3 \times \text{MaxThreads}$).
* Ràng buộc an toàn File Descriptors:
  $$\text{MaxThreads} \times \text{MaxRecursion} + (\text{MaxQueue} - \text{MaxThreads}) + 6 < \text{RLIMIT\_NOFILE}$$

### 9.2. Cấu hình Systemd Override (`LimitNOFILE`)
```bash
# sudo systemctl edit clamav-daemon
[Service]
LimitNOFILE=65536
```

### 9.3. Cấu hình chi tiết `/etc/clamav/clamd.conf`
```ini
# --- Socket Lắng Nghe ---
TCPSocket 3310
TCPAddr 0.0.0.0

# --- Tối Ưu Đa Luồng & Hàng Đợi ---
MaxThreads 16
MaxQueue 48
MaxConnectionQueueLength 200

# --- Timeout & Kết Nối Mạng LAN/VPN ---
ReadTimeout 60
CommandReadTimeout 10
SendBufTimeout 200

# --- Giới Hạn Quét Stream ---
StreamMaxLength 100M
MaxScanSize 150M
MaxFileSize 50M
MaxRecursion 10

# --- Bộ Nhớ RAM ---
ConcurrentDatabaseReload no
```

---

## 10. Cấu Trúc Dự Án (Tech Stack & Project Structure)

```text
zonendpoint/
├── Cargo.toml                      # Cấu hình Rust, rusqlite, sha2, tokio, notify
├── tauri.conf.json                 # Cấu hình Desktop Window, Permissions, Tray
├── src-tauri/
│   ├── src/
│   │   ├── main.rs                 # Điểm khởi chạy app & Tauri Builder
│   │   ├── db/
│   │   │   ├── mod.rs
│   │   │   ├── schema.rs           # Khởi tạo bảng WAL & BLOB Index
│   │   │   └── cache.rs            # Quản lý SQLite, truy vấn 2-tier & hash matching
│   │   ├── client/
│   │   │   ├── mod.rs
│   │   │   ├── socket.rs           # Quản lý TCP Connection Pool (hỗ trợ LAN/VPN IP)
│   │   │   └── protocol.rs         # Đóng gói lệnh zINSTREAM & parse kết quả
│   │   ├── monitor/
│   │   │   ├── mod.rs
│   │   │   └── watcher.rs          # Giám sát thư mục thời gian thực (notify crate)
│   │   ├── engine/
│   │   │   ├── scanner.rs          # Pipeline: Metadata Cache -> Hash Lookup -> Remote Stream
│   │   │   ├── filter.rs           # Lọc bỏ extension an toàn & xử lý batch file nhỏ
│   │   │   └── quarantine.rs       # Tước quyền NTFS, đổi đuôi & di chuyển file nhiễm
│   │   └── commands/
│   │       └── mod.rs              # Tauri IPC commands cho UI
├── src/                            # Frontend (TypeScript + Tailwind CSS)
│   ├── App.tsx                     # Dashboard chính
│   ├── components/
│   │   ├── StatusWidget.tsx        # Trạng thái kết nối Server (LAN/VPN) & Realtime Guard
│   │   ├── ScanProgress.tsx        # Tiến trình quét file/folder
│   │   └── QuarantineTable.tsx     # Bảng quản lý file cách ly
│   └── services/
│       └── tauri-bridge.ts         # Wrapper gọi invoke / listen event
└── scripts/
    ├── install-server.sh           # Cấu hình tự động Ubuntu Server ClamAV + Fangfrisch
    └── register-context-menu.ps1   # PowerShell tích hợp chuột phải Windows 11
```

---

## 11. Đánh Giá Rủi Ro & Chiến Lược Khắc Phục

| Kịch bản rủi ro | Hiện tượng thực tế | Giải pháp kiến trúc của Zonendpoint |
| :--- | :--- | :--- |
| **Quét lại file ISO 10GB cũ** | Nghẽn mạng LAN/VPN (~100 giây) | Tầng 1 Cache khớp metadata trả về kết quả `CLEAN` tức thì ($< 0.1\text{ ms}$). |
| **Quét thư mục nhiều file nhỏ** | RTT và socket latency cao | Áp dụng Pre-filter extension + Batching SQLite Cache + Đa luồng Connection Pool. |
| **File vừa tải đang bị khóa (Lock)** | Rust đọc file sinh lỗi `PermissionDenied` | Bỏ qua file đuôi tạm (`.crdownload`, `.tmp`); đợi sự kiện Rename hoàn tất và retry sau 100ms. |
| **Mất kết nối tới Server ClamAV** | Đứt kết nối VPN hoặc mất Internet | UI hiển thị cờ *"Engine Offline"*, vẫn duy trì bảo vệ bằng bảng Local Known-Bad Hashes. |
| **Quá tải Socket Server khi quét đồng thời** | Lỗi kết nối `Connection Refused` | Áp dụng Semaphore trên Client (tối đa 2–4 stream song song/máy) kèm Exponential Backoff. |
## 12. Current implementation and Notebook reconciliation — 2026-11-07

This addendum overrides conflicting aspirational proposals above for the current product. Original sections1–11 are retained as design history, not proof of implemented or approved protection. Read [project status](project-status.md), [scope](v1.0.0-scope.md), [roadmap](roadmap.md) and [technical plan](implementation-plan.md) first.

### 12.1. Approved architecture today

Zon confirmed internal company use first. Windows11 x64 React/Tauri client is a supplementary, manually initiated file scanner; application bitness does not filter file contents. Default Quick scan resolves native System32 via GetSystemDirectoryW and enumerates13 fixed core files plus drivers. File/Folder picker tokens select manual Scan. Both share a single UI job lifecycle, scan IDs/cancellation and typed outcomes; there is no memory/boot integrity or real-time prevention claim.

Native Rust sends64KiB INSTREAM chunks to the configured private/loopback ClamAV endpoint. Current limits:100,000,000 bytes/file,5s connect and60s total scan timeout,1024-byte framed response cap, scanner semaphore2, directory queue32, retained result details1000. UI allows one job at a time; retained count bounds do not mean only1000 files are scanned. Error/UNKNOWN/SKIPPED/cancelled work is visible and never clean.

SQLite stores scan_cache keyed by32-byte content hash+engine identity+policy identity; TTL12h. Current content and stable pre/post file identity are verified, and engine identity must be available before reuse. Schema is versioned/WAL and the DB path is shared by scan/cache stats. No public known-good/bad IOC tables or automatic external lookup is implemented. Plain SQLite cache is not SQLCipher.

Clamd is not an authenticated/encrypted transport by itself. Current lab uses loopback3310 on Ubuntu and an SSH tunnel exposed on Windows loopback13310; private addressing alone is not equivalent to encryption/server authentication. Future production topology requires an approved transport/deployment boundary. Existing CSP/capabilities and test-only hook guards must not be weakened for new features.

### 12.2. Superseded assumptions and future candidates

- Metadata-only instant CLEAN, “safe extension” bypass, whitelist-first CLEAN and declaring offline cache protection are not current guarantees. Preserve observable engine detections and incomplete/error states. A future allow-policy action must stay separate from a scanner verdict.
- The old0.0.0.0 clamd example is not an approved deployment. No Internet-exposed clamd, public feed/submission, quarantine or Guard expansion belongs to currentv1.0.0.
- Notebook SourceDatabase-v2 lists9 source groups, hash dedup/deltas and a5-tier lookup. These are reviewed candidates; license/authentication, exact ClamAV/YARA compatibility, signed update/revocation and conflict policy must be resolved before adopting any pipeline. Do not simply load every community rule, treat domains as file-hash signatures or enable free public APIs for business use.
- Battle Mode is a future bounded scheduler tied to an opt-in Guard/IOC design. IP blocking/socket termination needs typed attribution, privilege separation, IPv4/IPv6/UDP scope, management/VPN safeguards, expiring audited overrides and rollback. No network enforcement is implemented or authorized here.
- Process ACL/DLL search/ASLR/DEP/CFG, SQLCipher with DPAPI-managed keys and optional BYOK AI are separate proposals with threat model, deployment/recovery and evidence gates in the technical plan. User-space hardening is not protected-process immunity. AI analysis is advisory and cannot authorize clean verdicts, execution or enforcement.
- RAM, CPU, lookup/delta latency and detection-rate numbers in source proposals are unvalidated targets. Measure the actual process tree, deployed engine/signatures, corpus and network; do not market them as observed product results.

Existing [Ed25519/gateway/licensing source spec](zonendpoint_ed25519_secure_gateway_licensing_spec.md) is preserved unchanged; its unresolved encryption/server identity, key lifecycle, revocation/token and VERSION/cache-identity decisions remain in the implementation plan. No new gateway/licensing dependency or service was added.
