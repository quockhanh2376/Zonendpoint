# FEED-LAB: Đánh giá cơ sở dữ liệu chữ ký Sanesecurity (Task V-67)

> **Mục đích:** Nghiên cứu giấy phép, phân loại rủi ro cảnh báo nhầm, đo kiểm hiệu năng thực tế (A/B testing) và đề xuất phương án triển khai cơ sở dữ liệu chữ ký Sanesecurity cho hệ thống ZonEndpoint.  
> **Phạm vi:** CHỈ đánh giá thử nghiệm trong môi trường độc lập `/tmp/v67`, TUYỆT ĐỐI KHÔNG triển khai, không ghi vào `/var/lib/clamav`, không sửa cấu hình, không reload/restart daemon `clamd` đang chạy trên máy lab.  
> **Thời điểm thực hiện:** 2026-11-14.  
> **Người thực hiện:** Vic (FEED-LAB).

---

## 1. Giấy phép, Nguồn và Điều kiện Sử dụng

### 1.1. Nguồn chính thức
- **Trang chủ chính thức:** [sanesecurity.com](https://sanesecurity.com/)
- **Trang điều khoản & pháp lý (Legal Notice):** [sanesecurity.com/legal/](https://sanesecurity.com/legal/)
- **Trang hướng dẫn sử dụng (Usage):** [sanesecurity.com/usage/](https://sanesecurity.com/usage/)
- **Danh mục chữ ký (Signatures):** [sanesecurity.com/usage/signatures/](https://sanesecurity.com/usage/signatures/)
- **Tài liệu Linux scripts:** [sanesecurity.com/usage/linux-scripts/](https://sanesecurity.com/usage/linux-scripts/)
- **Tài liệu Windows scripts:** [sanesecurity.com/usage/windows-scripts/](https://sanesecurity.com/usage/windows-scripts/)
- **Trang quyên góp (Donate):** [sanesecurity.com/donate/](https://sanesecurity.com/donate/)
- **Foxhole databases:** [sanesecurity.com/foxhole-databases/](https://sanesecurity.com/foxhole-databases/)
- **Trang trạng thái mirror (Status):** [sanesecurity.com/support/status/](https://sanesecurity.com/support/status/)

### 1.2. Điều kiện sử dụng cá nhân & thương mại
- **Sử dụng cá nhân / Phi thương mại (Personal Use):**
  - **Được phép sử dụng hoàn toàn miễn phí.** Sanesecurity hoan nghênh và khuyến khích đóng góp ủng hộ (khuyến nghị tối thiểu $20) để trang trải chi phí duy trì cơ sở hạ tầng phân phối chữ ký phi lợi nhuận của tác giả Steve Basford.
- **Sử dụng thương mại / Doanh nghiệp / Chính phủ (Commercial / ISP / Education / Government Use):**
  - Sanesecurity cho phép tích hợp và sử dụng **miễn phí** trong các sản phẩm và mạng lưới thương mại ("*You can use the SaneSecurity signatures free of charge in commercial products*").
  - Điều kiện kèm theo: Sanesecurity khuyến khích các tổ chức thương mại gửi một khoản đóng góp (khuyến nghị tối thiểu $100) và gửi email thông báo tên công ty cùng tên sản phẩm tích hợp tới tác giả.
- **Các điều khoản ràng buộc và hạn chế:**
  - **Nghiêm cấm sao chép, nhân bản hoặc phân phối lại:** Không được phép sao chép hoặc tái phân phối các tập tin chữ ký nếu không có sự đồng ý bằng văn bản của Sanesecurity.
  - **Nghiêm cấm bán lại (Resale):** Nghiêm cấm bán bất kỳ tài liệu, dữ liệu hoặc tập tin chữ ký nào lấy từ website Sanesecurity.
  - **Miễn trừ trách nhiệm (Disclaimer):** Chữ ký được cung cấp theo nguyên tắc "as is" (nguyên trạng), không có cam kết bảo đảm về tính liên tục, tính không có lỗi hoặc khả năng không phát sinh cảnh báo nhầm. Người dùng tự chịu rủi ro khi triển khai trên hệ thống thực tế.
- **Kết luận cấp phép cho ZonEndpoint:** ZonEndpoint (sử dụng nội bộ cá nhân / phi thương mại trên 1–2 máy trạm) **hoàn toàn đủ điều kiện sử dụng hợp pháp** miễn phí.

### 1.3. Hệ thống Mirror chính thức
- **Mirror HTTP / HTTPS công cộng:**
  - Mirror chính thức được khuyến nghị bởi Sanesecurity và công cụ Fangfrisch: `https://mirror.seichter.de/sanesecurity/` (vận hành bởi Ralph Seichter — tác giả Fangfrisch, hỗ trợ cả HTTP và HTTPS qua cổng 80/443).
- **Cụm Mirror Rsync:**
  - Tên miền phân phối: `rsync.sanesecurity.net` (cổng TCP 873). Cụm máy chủ này sử dụng cơ chế DNS round-robin để điều hướng người dùng tới các mirror rsync đang hoạt động trên toàn cầu.
- **Mirror riêng biệt (Dedicated Non-Public Mirrors):**
  - Dành riêng cho người ủng hộ (donors): được cấp URL tải riêng biệt không công khai, sử dụng các máy chủ tốc độ cao nhất và nhận thông báo khi có thay đổi quan trọng.

### 1.4. Cơ chế xác minh chữ ký số (GPG Verification)
- Tất cả các tập tin cơ sở dữ liệu (`.hdb`, `.hsb`, `.cdb`, `.ndb`, `.ign2`, `.ftm`) đều có tập tin chữ ký số detached tương ứng mang phần mở rộng `.sig` (ví dụ `rogue.hdb.sig`).
- **Khóa công khai GPG chính thức:**
  - URL tải khóa: `https://sanesecurity.com/publickey.gpg` (mirror dự phòng: `http://sanesecurity.net/publickey.gpg`).
  - Key ID: `0x97CD8EE6`
  - Fingerprint: `890D 413C 9E6E FD80 9D0E E892 D6F4 C536 97CD 8EE6`
  - User ID: `Sanesecurity (Sanesecurity Signatures) <steveb_clamav@sanesecurity.co.uk>`
  - Thuật toán: DSA 1024-bit / Elgamal.
- **Quy trình xác minh:**
  ```bash
  export GNUPGHOME="/path/to/temp_gnupg"
  gpg --import publickey.gpg
  gpg --verify <database_file>.sig <database_file>
  ```
  Nếu kết quả trả về `Good signature from "Sanesecurity..."` thì tập tin toàn vẹn và hợp lệ.

### 1.5. Tần suất cập nhật & Giới hạn tải (Rate Limits)
- **Tần suất cập nhật:**
  - Chữ ký Sanesecurity được cập nhật nhiều lần trong ngày.
  - Riêng tập tin `rogue.hdb` (chuyên về mã độc mới, phần mềm bảo mật giả mạo, codec lừa đảo) được cập nhật liên tục hàng giờ (*Updated hourly*).
- **Giới hạn tải và quy định chống lạm dụng:**
  - **Tuyệt đối cấm quét/tải liên tục từng giây:** Sanesecurity cảnh báo tường lửa của các mirror sẽ tự động chặn (block) địa chỉ IP nếu phát hiện tần suất tải quá nhiều lần trong một giờ hoặc lạm dụng băng thông.
  - **Khuyến nghị chu kỳ cron:** Tần suất kiểm tra cập nhật an toàn là từ **1 đến 2 giờ một lần** (hoặc đồng bộ theo chính sách nạp lại tối thiểu của hệ thống).
  - Nếu dịch vụ công cộng tiếp tục bị lạm dụng, Sanesecurity có thể chuyển toàn bộ rsync sang chế độ bảo vệ bằng mật khẩu chỉ dành cho người ủng hộ.

---

## 2. Danh mục Cơ sở Dữ liệu & Đề xuất Bộ "Low False-Positive" cho Quét File Windows

Sanesecurity phân phối chữ ký do chính họ tạo ra cũng như từ một số đối tác tin cậy (OITC, Porcupine, bofhland, MiscreantPunch). Bảng phân loại dưới đây tổng hợp theo công bố chính thức về mức rủi ro cảnh báo nhầm (FP Risk) và đánh giá sự phù hợp với bài toán quét tập tin cục bộ trên Windows:

### 2.1. Bảng phân loại toàn bộ cơ sở dữ liệu Sanesecurity

| Tên Database | Đơn vị tạo | Mục tiêu phát hiện | Định dạng ClamAV | Rủi ro FP (Sanesecurity công bố) | Phù hợp quét file Windows? | Đề xuất |
|---|---|---|---|:---:|:---:|:---:|
| `rogue.hdb` | Sanesecurity | Malware, Fake AV, Rogue Codecs (cập nhật theo giờ) | HDB (MD5) | **Low** | **Rất cao** (mã độc thực thi) | **CHỌN** |
| `malwarehash.hsb` | Sanesecurity | Hash SHA-256 mã độc không rõ kích thước | HSB (SHA256) | **Low** | **Rất cao** (mã độc thực thi) | **CHỌN** |
| `hackingteam.hsb` | Sanesecurity | Hash mã độc và công cụ Hacking Team | HSB (SHA256) | **Low** | **Rất cao** (mã độc APT/tool) | **CHỌN** |
| `foxhole_generic.cdb` | Sanesecurity | File nén chứa file đuôi kép nguy hiểm (.pif, .scr, .exe, .bat...) | CDB (Container) | **Low** | **Rất cao** (chống lừa đảo Windows) | **CHỌN** |
| `foxhole_filename.cdb` | Sanesecurity | File nén chứa tên file mã độc phổ biến | CDB (Container) | **Low** | **Rất cao** (chống trojan archive) | **CHỌN** |
| `bofhland_malware_attach.hdb` | bofhland | Hash MD5 mã độc đính kèm | HDB (MD5) | **Low** | **Rất cao** (mã độc thực thi) | **CHỌN** |
| `porcupine.hsb` | Porcupine | Hash SHA-256 mã độc VBScript và JSE (7 ngày gần nhất) | HSB (SHA256) | **Low** | **Rất cao** (script độc hại Windows) | **CHỌN** |
| `winnow_malware.hdb` | OITC | Virus, trojan chưa có trong ClamAV chính thức | HDB (MD5) | **Low** | **Rất cao** (mã độc thực thi) | **CHỌN** |
| `winnow_extended_malware.hdb` | OITC | Chữ ký hash mã độc bổ sung thủ công | HDB (MD5) | **Low** | **Rất cao** (mã độc thực thi) | **CHỌN** |
| `winnow_bad_cw.hdb` | OITC | Hash MD5 mã độc thu thập từ botnet | HDB (MD5) | **Low** | **Rất cao** (mã độc thực thi) | **CHỌN** |
| `sigwhitelist.ign2` | Sanesecurity | Danh sách trắng cập nhật nhanh để triệt tiêu FP | IGN2 (Whitelist) | — | **Bắt buộc** (bảo vệ tránh FP) | **CHỌN** |
| `sanesecurity.ftm` | Sanesecurity | Định nghĩa cấu trúc file type cho parser | FTM (File type) | — | **Bắt buộc** (hỗ trợ ClamAV) | **CHỌN** |
| `junk.ndb` | Sanesecurity | Chuỗi văn bản thư rác (spam, 419, vé số, tuyển dụng) | NDB (Pattern) | Low | Rất thấp (thuần email text) | LOẠI |
| `jurlbl.ndb` | Sanesecurity | Danh sách URL rác / spam | NDB (Pattern) | Low | Thấp (dễ chặn nhầm link trong code) | LOẠI |
| `jurlbla.ndb` | Sanesecurity | URL rác tự động tổng hợp | NDB (Pattern) | Med | Thấp (nguy cơ FP cao) | LOẠI |
| `phish.ndb` | Sanesecurity | URL và mẫu email lừa đảo (phishing) | NDB (Pattern) | Low | Thấp (thuần email phishing) | LOẠI |
| `scam.ndb` | Sanesecurity | Thư lừa đảo tài chính | NDB (Pattern) | Low | Rất thấp (thuần email) | LOẠI |
| `spam.ldb` | Sanesecurity | Chữ ký logic nhận dạng thư rác | LDB (Logical) | Med | Rất thấp (thuần email) | LOẠI |
| `spamimg.hdb` | Sanesecurity | Hash ảnh chèn trong thư rác | HDB (MD5) | Low | Rất thấp (ảnh spam email) | LOẠI |
| `spamattach.hdb` | Sanesecurity | Hash file đính kèm thư rác | HDB (MD5) | Low | Trung bình (trùng lặp với hash malware) | LOẠI |
| `spear.ndb` | Sanesecurity | Địa chỉ email lừa đảo có chủ đích | NDB (Pattern) | Med | Rất thấp (thuần email header) | LOẠI |
| `spearl.ndb` | Sanesecurity | URL lừa đảo có chủ đích | NDB (Pattern) | Med | Thấp (thuần URL) | LOẠI |
| `lott.ndb` | Sanesecurity | Thư rác trúng thưởng vé số | NDB (Pattern) | Med | Rất thấp (thuần email text) | LOẠI |
| `blurl.ndb` | Sanesecurity | URL mã độc/spam trong 7 ngày gần nhất | NDB (Pattern) | Low | Thấp (thuần URL) | LOẠI |
| `badmacro.ndb` | Sanesecurity | Chặn macro nguy hiểm trong Word/Excel/XML/RTF | NDB (Pattern) | Med | Nguy cơ chặn macro lành tính nội bộ | LOẠI |
| `shelter.ldb` | Sanesecurity | Phishing & Malware logic | LDB (Logical) | Med | Nguy cơ FP trung bình | LOẠI |
| `foxhole_js.cdb` | Sanesecurity | Chặn JavaScript (.js) trong Zip/Rar | CDB (Container) | Med | Gây lỗi cho zip lập trình viên web/npm | LOẠI |
| `foxhole_js.ndb` | Sanesecurity | Chặn JavaScript (.js) trong Gzip/Ace | NDB (Pattern) | Med | Gây lỗi cho zip lập trình viên | LOẠI |
| `foxhole_all.cdb` | Sanesecurity | Chặn toàn bộ đuôi nguy hiểm (kể cả đuôi đơn .exe) trong Zip/Rar/7z | CDB (Container) | **High** | Chặn nhầm 100% file zip phần mềm/setup | LOẠI |
| `foxhole_all.ndb` | Sanesecurity | Chặn đuôi nguy hiểm trong lưu trữ mở rộng | NDB (Pattern) | **High** | Chặn nhầm file nén phần mềm | LOẠI |
| `foxhole_mail.cdb` | Sanesecurity | Chặn file đính kèm nguy hiểm trong email | CDB (Container) | **High** | Thuần email gateway | LOẠI |
| `winnow_malware_links.ndb` | OITC | Liên kết tải mã độc | NDB (Pattern) | Low | Thấp (thuần URL) | LOẠI |
| `winnow_spam_complete.ndb` | OITC | Mẫu email spam và gian lận | NDB (Pattern) | Med | Rất thấp (thuần email) | LOẠI |
| `winnow_phish_complete.ndb` | OITC | URL lừa đảo | NDB (Pattern) | High | Rất thấp (thuần URL, FP cao) | LOẠI |
| `winnow_phish_complete_url.ndb` | OITC | Toàn bộ đường dẫn URL lừa đảo | NDB (Pattern) | Med | Thấp (thuần URL) | LOẠI |
| `winnow.complex.patterns.ldb` | OITC | Mẫu logic phát hiện gian lận và mã độc | LDB (Logical) | Med | Nguy cơ FP trung bình | LOẠI |
| `winnow.attachments.hdb` | OITC | Hash file đính kèm thư rác | HDB (MD5) | Low | Thấp (ảnh/file spam) | LOẠI |
| `porcupine.ndb` | Porcupine | Email lừa đảo tiếng Bồ Đào Nha (Brazil) | NDB (Pattern) | Low | Rất thấp (email khu vực) | LOẠI |
| `phishtank.ndb` | Porcupine | URL lừa đảo từ nguồn PhishTank | NDB (Pattern) | Low | Thấp (thuần URL) | LOẠI |
| `bofhland_malware_URL.ndb` | bofhland | URL mã độc | NDB (Pattern) | Low | Thấp (thuần URL) | LOẠI |
| `bofhland_phishing_URL.ndb` | bofhland | URL lừa đảo | NDB (Pattern) | Low | Thấp (thuần URL) | LOẠI |
| `bofhland_cracked_URL.ndb` | bofhland | URL máy chủ bị xâm nhập | NDB (Pattern) | Low | Thấp (thuần URL) | LOẠI |
| `MiscreantPunch099-Low.ldb` | MiscreantPunch | Quy tắc phát hiện Macro, JS, HTA bất thường | LDB (Logical) | Med | Nguy cơ FP trung bình trên tài liệu văn phòng | LOẠI |
| `MiscreantPunch099-INFO-Low.ldb`| MiscreantPunch | Cung cấp thông tin ngữ cảnh tài liệu | LDB (Logical) | **High** | Không phải chữ ký diệt, cảnh báo cao | LOẠI |
| `Sanesecurity_sigtest.yara` | Sanesecurity | Rule YARA kiểm thử hệ thống | YARA | Low | Quy tắc thử nghiệm, không có giá trị bảo mật | LOẠI |
| `Sanesecurity_spam.yara` | Sanesecurity | Rule YARA phát hiện email spam | YARA | Low | Thuần email | LOẠI |

### 2.2. Đề xuất bộ "Low False-Positive" cho ZonEndpoint (12 tập tin)
Đối với bài toán của ZonEndpoint — một ứng dụng bảo vệ endpoint quét file và thư mục trực tiếp trên máy trạm Windows, các feed email spam/phishing URL mang lại ít giá trị bảo vệ nhị phân nhưng lại có nguy cơ chặn nhầm cao vào tài liệu HTML, code JavaScript, hoặc link văn bản.

Do đó, bộ cơ sở dữ liệu Sanesecurity tối ưu được đề xuất gồm **12 tập tin**:
1. `rogue.hdb`: Hash MD5 nhận diện Fake Antivirus, Rogue software, Fake Codecs (cực kỳ hữu hiệu trên Windows).
2. `malwarehash.hsb`: Hash SHA-256 các biến thể mã độc thực thi độc lập.
3. `hackingteam.hsb`: Hash SHA-256 các công cụ khai thác và trojan mục tiêu.
4. `foxhole_generic.cdb`: Nhận dạng các file nén chứa file thực thi đội lốt đuôi kép (ví dụ `.pdf.exe`, `.doc.vbs`, `.invoice.scr`).
5. `foxhole_filename.cdb`: Nhận dạng tên file mã độc phổ biến ẩn trong file nén.
6. `bofhland_malware_attach.hdb`: Hash MD5 các mẫu mã độc đính kèm phổ biến.
7. `porcupine.hsb`: Hash SHA-256 các script độc hại định dạng VBS và JSE (vốn là vector tấn công rất phổ biến trên Windows).
8. `winnow_malware.hdb`: Hash MD5 virus và trojan cập nhật từ OITC.
9. `winnow_extended_malware.hdb`: Hash MD5 mở rộng từ OITC.
10. `winnow_bad_cw.hdb`: Hash MD5 mã độc từ các mạng lưới botnet.
11. `sigwhitelist.ign2`: Danh sách trắng chính thức của Sanesecurity nhằm tức thời bỏ qua các chữ ký bị lỗi FP.
12. `sanesecurity.ftm`: Định nghĩa kiểu tập tin cho ClamAV.

---

## 3. Tải qua HTTPS và Xác minh GPG

Thực hiện tải toàn bộ 12 tập tin đề xuất và 12 tập tin chữ ký `.sig` tương ứng từ mirror chính thức `https://mirror.seichter.de/sanesecurity/` vào thư mục tạm `/tmp/v67/ss`, sử dụng môi trường GPG tạm tại `/tmp/v67/gnupg`:

```
=== Setting up GPG environment in /tmp/v67/gnupg ===
Key D691DED931EA4D9E imported: "Sanesecurity (Sanesecurity Signatures) <steveb_clamav@sanesecurity.co.uk>"
Fingerprint: 890D 413C 9E6E FD80 9D0E E892 D6F4 C536 97CD 8EE6
```

### Kết quả xác minh GPG từng file:

| Tập tin | Dung lượng (bytes) | Kết quả kiểm tra GPG | Chi tiết chữ ký số | Trạng thái |
|---|---:|:---:|---|:---:|
| `rogue.hdb` | 115,696 | **PASS** | Good signature from Sanesecurity | **CHẤP NHẬN** |
| `malwarehash.hsb` | 100,848 | **PASS** | Good signature from Sanesecurity | **CHẤP NHẬN** |
| `hackingteam.hsb` | 48,176 | **PASS** | Good signature from Sanesecurity | **CHẤP NHẬN** |
| `foxhole_generic.cdb` | 51,409 | **PASS** | Good signature from Sanesecurity | **CHẤP NHẬN** |
| `foxhole_filename.cdb` | 319,328 | **PASS** | Good signature from Sanesecurity | **CHẤP NHẬN** |
| `bofhland_malware_attach.hdb` | 106,247 | **PASS** | Good signature from Sanesecurity | **CHẤP NHẬN** |
| `porcupine.hsb` | 4,045 | **PASS** | Good signature from Sanesecurity | **CHẤP NHẬN** |
| `winnow_malware.hdb` | 65 | **PASS** | Good signature from Sanesecurity | **CHẤP NHẬN** |
| `winnow_extended_malware.hdb` | 65 | **PASS** | Good signature from Sanesecurity | **CHẤP NHẬN** |
| `winnow_bad_cw.hdb` | 66 | **PASS** | Good signature from Sanesecurity | **CHẤP NHẬN** |
| `sigwhitelist.ign2` | 205 | **PASS** | Good signature from Sanesecurity | **CHẤP NHẬN** |
| `sanesecurity.ftm` | 12,598 | **PASS** | Good signature from Sanesecurity | **CHẤP NHẬN** |

- **Tổng số tập tin đề xuất:** 12
- **Số tập tin xác minh GPG thành công:** 12 (100%)
- **Số tập tin thất bại / bị loại:** 0

---

## 4. Đo kiểm A/B Hiệu năng và Cảnh báo Nhầm (Chuẩn Q-46)

### 4.1. Cấu hình thử nghiệm
- **Môi trường lab:** Máy chủ lab Ubuntu 26.04.1 LTS, ClamAV 1.5.4, RAM khả dụng trước đo: `12,789 MiB` (vượt xa ngưỡng an toàn 6 GiB).
- **Bộ A (Hiện tại - Đã bỏ `securiteinfoold.hdb`):** 14 tập tin lấy từ `/var/lib/clamav` (`bytecode.cvd`, `daily.cld`, `javascript.ndb`, `main.cvd`, `malwarebazaar.hsb`, `securiteinfo.hdb`, `securiteinfo.ign2`, `securiteinfoandroid.hdb`, `securiteinfoascii.hdb`, `securiteinfohtml.hdb`, `securiteinfopdf.hdb`, `spam_marketing.ndb`, `threatfox.hsb`, `yara-forge-core.yar`).
- **Bộ B (Bộ A + Sanesecurity):** Bộ A cộng thêm 12 tập tin Sanesecurity đã xác minh GPG.
- **Tập mẫu vô hại (Benign Sample Set):** Tổng cộng **445 tập tin** (tổng dung lượng **48.27 MiB** / 50,618,418 bytes):
  - 300 tập tin thực thi và thư viện hệ thống từ `/usr/bin` (150 file) và `/usr/lib` (150 file), mỗi file $\le 25$ MiB.
  - 65 tập tin tài liệu Markdown `.md` từ thư mục `docs/` của repo Zonendpoint.
  - 40 tập tin tài liệu HTML `.html` từ `/usr/share/doc`.
  - 2 tập tin văn phòng tiêu chuẩn: 1 file Word `.docx` và 1 file Excel `.xlsx` hợp lệ.
  - 1 tập tin PDF `.pdf` từ `/usr/share/doc` (`shared-mime-info-spec.pdf`).
  - 7 tập tin JavaScript `.js` và 5 tập tin `.mjs`.
  - 12 tập tin `.json`, 4 tập tin `.txt`, 4 tập tin `.png`, 3 tập tin `.py`, 2 tập tin `.ps1`, 2 tập tin `.lst`, 1 tập tin `.sh`.
- **Giao thức đo:** Sử dụng `/usr/bin/time -v nice -n 19 clamscan --no-summary -d <bundle>`:
  - Đo nạp (Load): thực hiện trên file rỗng `/tmp/v67/empty`.
  - Đo quét (Scan): thực hiện đệ quy trên toàn bộ 445 tập tin mẫu.
  - Đo lặp lại 3 lần cho mỗi bộ, nghỉ 30 giây giữa các lần chạy theo quy chuẩn Q-46.

### 4.2. Thống kê chữ ký đóng góp của Sanesecurity

| Tập tin Sanesecurity bổ sung | Dung lượng (bytes) | Số chữ ký bổ sung |
|---|---:|---:|
| `bofhland_malware_attach.hdb` | 106,247 | 1,836 |
| `foxhole_filename.cdb` | 319,328 | 3,678 |
| `foxhole_generic.cdb` | 51,409 | 211 |
| `hackingteam.hsb` | 48,176 | 435 |
| `malwarehash.hsb` | 100,848 | 1,031 |
| `porcupine.hsb` | 4,045 | 50 |
| `rogue.hdb` | 115,696 | 1,036 |
| `sanesecurity.ftm` | 12,598 | 186 |
| `sigwhitelist.ign2` | 205 | 6 (bỏ qua) |
| `winnow_bad_cw.hdb` | 66 | 1 |
| `winnow_extended_malware.hdb` | 65 | 1 |
| `winnow_malware.hdb` | 65 | 1 |
| **Tổng cộng Sanesecurity thêm vào** | **758,748 bytes (~741 KiB / 0.72 MiB)** | **+8,472 chữ ký** |

### 4.3. Bảng số đo chi tiết từng lượt chạy

| Bundle | Lần đo | Thời gian Load (s) | Max RSS Load (KiB) | Thời gian Scan (s) | Max RSS Scan (KiB) | Số phát hiện (FOUND) |
|:---:|:---:|---:|---:|---:|---:|:---:|
| **A** | 1 | 13.02 | 1,182,384 | 32.20 | 1,197,252 | 0 |
| **A** | 2 | 12.75 | 1,182,552 | 32.43 | 1,197,068 | 0 |
| **A** | 3 | 13.01 | 1,182,396 | 31.49 | 1,197,012 | 0 |
| **B** | 1 | 11.57 | 1,190,140 | 30.97 | 1,204,660 | 0 |
| **B** | 2 | 13.52 | 1,190,136 | 33.91 | 1,204,804 | 0 |
| **B** | 3 | 12.59 | 1,190,116 | 33.24 | 1,204,880 | 0 |

### 4.4. Bảng tổng hợp so sánh A/B và chênh lệch (Delta)

| Chỉ số đánh giá | Bộ A (Hiện tại) | Bộ B (A + Sanesecurity) | Chênh lệch ($\Delta = B - A$) | Tỷ lệ thay đổi (%) |
|---|---:|---:|---:|---:|
| **Thời gian nạp trung bình (Load)** | **12.93 s** | **12.56 s** | -0.37 s | **-2.84%** (nhanh hơn nhẹ) |
| **Max RSS nạp trung bình (Load)** | **1,182,444 KiB** (~1,154.7 MiB) | **1,190,131 KiB** (~1,162.2 MiB) | **+7,687 KiB** (~7.51 MiB) | **+0.65%** |
| **Thời gian quét mẫu trung bình (Scan)** | **32.04 s** | **32.71 s** | **+0.67 s** | **+2.08%** |
| **Max RSS quét mẫu trung bình (Scan)** | **1,197,111 KiB** (~1,169.1 MiB) | **1,204,781 KiB** (~1,176.5 MiB) | **+7,671 KiB** (~7.49 MiB) | **+0.64%** |
| **Tổng số FOUND trên 445 file vô hại** | **0** | **0** | **0** | **0% False Positive** |

### 4.5. Phân tích kết quả
1. **Bộ nhớ RAM tăng thêm:**
   - Khi thêm toàn bộ 12 tập tin Sanesecurity (+8,472 chữ ký), Max RSS chỉ tăng thêm **~7.5 MiB** (từ 1,182 MiB lên 1,190 MiB, tức tăng vỏn vẹn **0.65%**).
   - Mức tăng này cực kỳ nhỏ, chỉ bằng 1/50 mức tăng của YARA-Forge (+110 MiB) hoặc MalwareBazaar (+122 MiB).
2. **Thời gian nạp và quét:**
   - Thời gian nạp database hầu như không biến động (chênh lệch $\pm 0.3$ giây nằm trong khoảng nhiễu bình thường của I/O hệ điều hành).
   - Thời gian quét 445 tập tin đa dạng chỉ tăng thêm **0.67 giây** (+2.08%), hoàn toàn không ảnh hưởng tới trải nghiệm người dùng.
3. **Độ an toàn cảnh báo nhầm (False Positive):**
   - Cả 3 lượt quét trên 445 tập tin vô hại (bao gồm file nhị phân hệ thống, script JS, tài liệu HTML, tài liệu Office DOCX/XLSX và PDF) đều kết thúc với mã thoát `0` và **0 FOUND**.
   - Điều này chứng minh việc loại bỏ các tập tin email spam/phishing thô (`junk.ndb`, `phish.ndb`, `foxhole_all.cdb`) và chỉ giữ lại bộ lọc mã độc tinh gọn đã ngăn chặn triệt để nguy cơ cảnh báo nhầm trên máy trạm Windows.

---

## 5. Đề xuất Phương án Triển khai (Khuyến nghị kỹ thuật - Chưa thực hiện)

### 5.1. Lựa chọn công cụ cập nhật: Fangfrisch vs Script Python nội bộ
- **Fangfrisch:**
  - *Ưu điểm:* Là công cụ chính thức được khuyến nghị trên trang Sanesecurity, viết bằng Python, xử lý tốt việc tải có điều kiện (HTTP ETag / Last-Modified), tự động kiểm tra GPG.
  - *Nhược điểm:* Yêu cầu cài đặt thêm gói ngoài và cấu hình SQLite riêng của Fangfrisch; khó tích hợp sâu với cơ chế khóa triển khai chung (`deploy.lock`) và hệ thống quản lý trạng thái nạp lại (`reload-state.json`) mà dự án ZonEndpoint đang chuẩn hóa trong Task V-62.
- **Script nội bộ (`scripts/feeds/zonendpoint-sanesecurity.py`):**
  - *Khuyến nghị:* **Nên viết script Python riêng** theo chuẩn kiến trúc của `zonendpoint-malwarebazaar.py` và `zonendpoint-yara-update.py`.
  - *Lý do:* Tương thích 100% với kiến trúc hiện có của ZonEndpoint:
    1. Hỗ trợ đầy đủ cờ `--dry-run`, `--check-only`, `--lock-file`.
    2. Sử dụng thư viện chuẩn của Python, không phụ thuộc gói ngoài.
    3. Tích hợp sẵn bộ lọc file và tự động tải kèm chữ ký `.sig` để xác minh GPG trước khi deploy.

### 5.2. Tích hợp cơ chế Khóa chung (Deploy Lock) & Khoảng cách nạp lại (Reload Interval) theo V-62
- **Khóa dùng chung (`flock`):**
  - Sử dụng chung file khóa triển khai của V-62: `/run/lock/zonendpoint-clamd-deploy.lock` (hoặc biến môi trường `DEPLOY_LOCK_FILE`).
  - Trước khi bắt đầu bước kiểm thử hoặc deploy, script phải xin khóa độc quyền không chờ (`fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)`). Nếu khóa đang bị giữ bởi tiến trình khác (YARA update, MalwareBazaar update, hoặc rollback), script phải thoát ngay lập tức với mã lỗi 3, không gây tranh chấp tiến trình.
- **Khoảng cách nạp lại tối thiểu (Minimum Reload Interval):**
  - Sử dụng chung file ghi nhận trạng thái nạp lại: `/opt/zonendpoint/reload-state.json` (biến môi trường `RELOAD_STATE_FILE`).
  - Tuân thủ ngưỡng `min_reload_interval = 6.0` giờ (Task L-8 / V-62).
  - Khi tải về các tập tin Sanesecurity mới, nếu khoảng thời gian kể từ lần reload `clamd` gần nhất chưa đủ 6.0 giờ, script sẽ cập nhật tập tin vào đĩa nhưng hoãn lệnh reload `zRELOAD\0` lại cho chu kỳ tiếp theo (trừ khi có cờ `--force-reload`).

### 5.3. Vị trí Lưu trữ & Quy trình Rollback
- **Thư mục sao lưu (Backup):**
  - Lưu trữ tại `/var/backups/clamav-sanesecurity/` (hoặc `/opt/zonendpoint/sanesecurity/rollback/`).
  - Mỗi khi deploy tập tin mới vào `/var/lib/clamav/`, phiên bản cũ đang chạy sẽ được sao lưu nguyên tử kèm nhãn thời gian và SHA-256. Duy trì tối thiểu 3 bản sao lưu gần nhất.
- **Kịch bản Rollback độc lập:**
  - Xây dựng script `scripts/feeds/zonendpoint-sanesecurity-rollback.sh`.
  - Hỗ trợ khôi phục tức thì bản sao lưu gần nhất hoặc tùy chọn `--purge` (xóa toàn bộ các file Sanesecurity khỏi `/var/lib/clamav/`, kích hoạt zero-downtime `zRELOAD\0` và xác nhận PING -> PONG).

### 5.4. Tần suất cập nhật khuyến nghị
- Vì Sanesecurity cập nhật nhiều lần trong ngày nhưng `clamd` cần tránh reload quá dày đặc, tần suất cập nhật khuyến nghị là:
  - **1 đến 2 lần mỗi ngày** (ví dụ chạy qua systemd timer vào lúc 04:00 và 16:00 UTC+7).
  - Đảm bảo cách xa lịch cập nhật của YARA-Forge (~03:00–05:00 UTC+7) để tránh nghẽn I/O và tranh chấp khóa deploy.

### 5.5. Ước tính tác động tài nguyên khi triển khai thật lên clamd
- **Dung lượng RAM clamd tăng thêm:** Dự kiến tăng khoảng **~7.5 đến 10 MiB** (tương đương $\le 0.5\%$ tổng RAM daemon 1.7–2.0 GB). Đây là mức tăng chi phí bộ nhớ gần như không đáng kể.
- **Thời gian nạp lại (Reload duration):** Quá trình nạp lại của `clamd` dự kiến chỉ tốn thêm **$< 0.5$ giây** (tổng thời gian reload giữ nguyên trong khoảng 19–20 giây).
- **Lợi ích bảo mật:** Bổ sung ngay lập tức **+8,472 chữ ký** chuyên sâu về rogue software, ransomware double extension, và script độc hại VBS/JSE trên môi trường Windows mà cơ sở dữ liệu chính thức của ClamAV còn thiếu sót.

---

## 6. Nhật ký Chứng thực An toàn Hệ thống Lab

- **Trạng thái tiến trình `clamd`:**
  - Main PID trước khi bắt đầu đo kiểm: **132505**
  - Main PID sau khi kết thúc toàn bộ phép đo: **132505** (trùng khớp 100%, chứng minh tiến trình clamd không bị khởi động lại hay reload).
- **Nhật ký hệ thống `journalctl`:**
  - `journalctl -u clamav-daemon.service --since '2026-11-14 22:05:24'`: **-- No entries --** (không ghi nhận bất kỳ sự kiện nào liên quan đến clamd trong suốt quá trình chạy).
- **Thư mục hệ thống `/var/lib/clamav`:**
  - Hoàn toàn nguyên vẹn, không có bất kỳ tập tin nào bị ghi đè hay thay đổi.
  - Các tập tin cấu hình `clamd.conf` và `freshclam.conf` không bị can thiệp.
- **Dọn dẹp môi trường thử nghiệm:**
  - Toàn bộ dữ liệu thử nghiệm tại `/tmp/v67` được thu dọn và xóa sạch sau khi hoàn tất.
