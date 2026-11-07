# Native Folder picker follow-up — 8e8204a

This closes the earlier folder-picker desktop-tooling BLOCKED entry in report-8e8204a.md. Product code unchanged; tested QA executable built from8e8204aa10f478362263b60b8e46226367e0052f at E:\Zonendpoint-tools\qeen-target\debug\zonendpoint.exe. App launched directly, with fixture injection unset.

Observed via native desktop control:
1. Select target -> Folder opened actual Windows Select Folder dialog.
2. Navigated to E:\Zonendpoint-tools\picker-fixture and clicked Select Folder. App showed exact folder path; Start scan enabled.
3. Scanned through real ClamAV using temporary loopback SSH tunnel3310->Ubuntu3310. Result: Scan complete, Processed2, clean2, threats0, unknown0, errors0, skipped0, cancelled0; one.txt and two.txt visible; green/check All Good. Window grew to display both results.
4. Reopened real Folder dialog and clicked Cancel. Existing All Good/summary/results remained.
5. Clicked All Good to rescan. Scanning showed same selected folder; completed2 clean again (verified content cache hits). Selection survived Cancel.

Outcome:PASS for actual Folder selection, native UI->Rust->real engine folder scan, Cancel preserving selection/results, and rescan. Earlier file-picker Unicode PASS remains valid.

No EICAR claim: exact EICAR Windows detection remains BLOCKED by active endpoint AV; app safely reports error, never CLEAN. Installer, resource benchmark and full real-network reset/cancel matrix remain NOT RUN. Draft PR stays open pending acceptance of remaining validation limits; no merge.

Only dedicated benign fixtures were used; no security policy or AV exclusions changed. Temporary3310 SSH tunnel and owned QA app were stopped after testing; existing13310 tunnel retained.
