# Handoff: Task V-41 (Merge ThreatFox into main) — Vic to Qeen

**Date**: 2026-11-11  
**From**: Victor (Vic)  
**To**: Qeen  
**Task**: V-41 (Merge origin/jacky/threatfox into main)  
**Approval**: Zon approved 2026-11-11 ("Merge vào main")  
**Branch**: `vic/merge-v41`  
**Base Commit (main trước)**: `a9152229b4024da2b65a934dd4fa9fc81c612e55` (`a915222`)  
**Merged Branch**: `origin/jacky/threatfox` (tip `4e25f22a6f7033e02257962a6fd8a7c210fa83b5`, J-02 Jacky + V-39 + V-40, Q-28/Q-29 PASS 51/51 tests)  
**Merge Commit**: `bae722d54a9a2e6ff9bce18bb2b7e3d5508bf9be` (`bae722d`)  
**Code Snapshot SHA**: `3e62bf47bfbe3b1da207c4273010b9d62cecf2e7` (`3e62bf4`)  
**Main SHA mới**: `3e62bf47bfbe3b1da207c4273010b9d62cecf2e7` (`3e62bf4`)  
**Status**: MERGED TO MAIN & PUSHED TO ORIGIN  

---

## 1. Summary of Merged Feeds & Current Status

### A. ThreatFox Feed & Signatures (Tasks J-02, V-39, V-40, V-41)
- **Feeds live in clamd**:
  - **MalwareBazaar**: 1,171,240 hashes (`/var/lib/clamav/malwarebazaar.hsb`).
  - **ThreatFox**: 21,476 unique SHA-256 signatures (`/var/lib/clamav/threatfox.hsb`).
  - **Total custom feed signatures**: 1,192,716 signatures in clamd (clamd live total: 8,614,224 signatures).
- **ThreatFox Database (`/opt/zonendpoint/threatfox/threatfox.db`)**:
  - Total unique IOCs: **134,686** (deduplicated by `(ioc, ioc_type)`).
  - Breakdown: `sha256`: 45,040; `ip:port`: 29,291; `domain`: 28,311; `url`: 18,974; `md5`: 7,058; `sha1`: 6,012.
  - ClamAV deployment net deployable SHA-256: 45,040 - 23,564 (trùng MalwareBazaar) = **21,476**.
- **Operation Policy**:
  - Chạy hoàn toàn thủ công (manual execution only). Tuyệt đối **không cài đặt systemd timer hay cron job tự động**.
  - Rate limit dùng chung abuse.ch: 3 requests/giờ (sliding window 3600s), 400 requests/24h (`/opt/zonendpoint/abusech/rate_limit.json`).
  - Khóa bí mật Auth-Key cô lập tại `/etc/zonendpoint/abusech.key` (`0600 root:root`), 0 phơi nhiễm qua argv/logs/client, URL export được che giấu `<AUTH-KEY>`.
  - Phạm vi phi thương mại (Community API Terms) phòng vệ trên 1–2 máy nội bộ theo đúng phê duyệt của Zon.

### B. Q-29 Minor Fix & Duplicate Rows Policy (Task V-41)
- **Export thật có dòng trùng**: File CSV export thực tế của ThreatFox có 899 dòng trùng lặp `(ioc, ioc_type)` (trong đó có 1 dòng trùng SHA-256). SQLite gộp bằng `ON CONFLICT(ioc, ioc_type) DO UPDATE` và giữ bản mới nhất (malware, confidence, last_seen) theo đúng phê duyệt của Zon.
- **Sửa log `import_rows`**: Biến `sha256_count` chuyển sang đếm trực tiếp từ DB qua `conn.execute("SELECT count(*) FROM hashes").fetchone()[0]` (45.040) thay vì lấy số dòng CSV thô (45.041). Do đó `net_deployable_sha256 = sha256_count - mb_dupes = 45.040 - 23.564 = 21.476`, khớp chính xác 100% với số dòng xuất ra file chữ ký `.hsb`.
- **Unit test**: Thêm test `test_import_rows_log_reports_unique_sha256_and_matches_hsb_export` trong `scripts/feeds/test_zonendpoint_threatfox.py` (15/15 PASS).
- **Tài liệu**: Cập nhật chi tiết quy tắc gộp dòng trùng và đối soát số liệu trong `docs/ops/threatfox.md`.

---

## 2. Conflicts & Resolution

| File | Nature of Conflict | Resolution |
|---|---|---|
| Docs & code | Không có xung đột (Clean merge) | Base nhánh `origin/jacky/threatfox` xuất phát từ `main` tại `a915222`. `origin/main` không có commit mới sau `a915222`. Merge commit `bae722d` thực hiện qua cờ `--no-ff` bảo toàn 100% lịch sử commit. |

---

## 3. Quality Gates Verification

1. **`cargo fmt --check`**: PASS (clean, 0 diffs).
2. **`cargo clippy --all-targets -- -D warnings`**: PASS (0 warnings).
3. **`cargo test`**: 68/68 PASS (49 lib tests + 1 bin test + 18 resilience integration tests).
4. **Python Test Suites trên Lab (`/tmp/v41_verify`, Python 3.14.4)**:
   - `test_abusech_rate_limit.py`: **4/4 PASS** (0.004s).
   - `test_zonendpoint_threatfox.py`: **15/15 PASS** (0.025s).
   - `test_zonendpoint_malwarebazaar.py`: **28/28 PASS** (26.575s).
   - Tổng cộng: **47/47 PASS**, tiêu thụ đúng **0 request thật** tới abuse.ch.
5. **Cài đặt lại binary trên Lab**:
   - `/usr/local/bin/zonendpoint-threatfox.py` (755 root:root) được cài lại với bản cập nhật dòng log.
   - Kiểm tra `diff -u`: duy nhất 1 dòng log được thay đổi (`counts_by_type.get` → `SELECT count(*) FROM hashes`).
   - Không chạy `fetch`/`deploy`, không đụng tới DB, `.hsb` hay clamd.
   - clamd daemon `active` (PID 2245, 8.614.224 chữ ký, RSS ~2,305 GB, bind `127.0.0.1:3310`, SSH tunnel `13310 -> 3310` ổn định).
