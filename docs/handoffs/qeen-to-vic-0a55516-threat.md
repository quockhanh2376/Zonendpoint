# Handoff: Q-5 Threat View Verification Findings — Qeen to Vic

- **Date:** 2026-11-07
- **From:** Qeen (Independent Tester & Security Reviewer)
- **To:** Victor (Vic) — Lead Developer
- **Target Snapshot / SHA:** `0a555169fb6e72db82456e568f385982a8021e39` (`0a55516`)
- **Report Reference:** `docs/qa/ui-simplify-0a55516.md` (Mục 6)
- **Evidence Directory:** `E:\Zonendpoint-tools\artifacts\0a55516\qeen\threat\`

---

## 1. Findings Requiring Fix

### Finding Q-5-01 (Low / UI Polish): `.results small` lacks `overflow-wrap: anywhere;` causing horizontal scroll on unbroken long threat names
- **Severity:** Low / UI Polish
- **Affected File:** `src/App.css` (dòng 135)
- **Observed Behavior:**
  - Trong `src/App.css`:
    ```css
    .result-path { overflow-wrap: anywhere; }
    .results small { grid-column: 2; color: var(--muted); }
    ```
  - Trong khi `.result-path` đã được gắn `overflow-wrap: anywhere;` để chống tràn khi đường dẫn tệp dài, phần tử `.results small` (dùng để hiển thị tên mối đe dọa hoặc thông báo lỗi) lại không có thuộc tính ngắt dòng này (giá trị tính toán runtime: `overflow-wrap: normal; word-break: normal; white-space: normal`).
  - Khi ClamAV trả về một chữ ký/tên mã độc dài liên tục không chứa khoảng trắng (ví dụ `Win.Test.LongName.Trojan.Dropper.Generic.VeryLongSignatureStringWithNoSpacesToTestTextWrappingAndLayoutIntegrityUnderExtremeLengthConditions.Variant1234567890.End` - 153 ký tự):
    - Phần tử `<small>` phình to từ `clientWidth: 217px` thành `scrollWidth: 1006px` (`hasOverflow: true`).
    - Vùng chứa danh sách kết quả `.results` phình to từ `clientWidth: 419px` thành `scrollWidth: 1208px` (`hasHorizontalScroll: true`).
    - Xuất hiện thanh cuộn ngang (horizontal scrollbar) bên trong danh sách kết quả thay vì tự động ngắt dòng xuống dưới.
- **Recommended Remediation:**
  - Bổ sung `overflow-wrap: anywhere;` vào `.results small` trong `src/App.css`:
    ```css
    .results small { grid-column: 2; color: var(--muted); overflow-wrap: anywhere; }
    ```
