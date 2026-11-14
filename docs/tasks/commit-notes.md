# Commit notes (tracking for Loki)

One line per commit, newest last: `date | branch | SHA | pushed | summary`. Vic appends after every commit and push. Feature branches only; never push/merge `main`, never force-push.

| Date | Branch | SHA | Pushed | Summary |
|---|---|---|---|---|
| 2026-11-07 | vic/mvp-first-scan | 84c4dfa | YES | fix: use approved Quick Scan label consistently |
| 2026-11-07 | vic/mvp-first-scan | 0de3208 | YES | fix(cli): return success only for clean scan outcomes |
| 2026-11-07 | vic/lab-resilience-plan | 8eea144 | YES | docs(resilience): LAB-RESILIENCE test plan |
| 2026-11-07 | vic/mvp-first-scan | e000c01 | YES | docs: record Qeen acceptance of package 0de3208 and round-1 sync |
| 2026-11-07 | vic/lab-resilience-plan | 614763d | YES | chore: merge vic/mvp-first-scan into vic/lab-resilience-plan |
| 2026-11-07 | vic/lab-resilience-plan | 88e7195 | YES | feat(resilience): implement LAB-RESILIENCE Tier 1 integration tests and Tier 2 proxy harness |
| 2026-11-07 | vic/lab-resilience-plan | 998967f | YES | docs(tasks): update commit-notes with round-1 and resilience commits |
| 2026-11-07 | vic/ui-simplify | 8c687b9 | YES | feat(ui): simplify target selection, result view and clamd preflight status |
| 2026-11-07 | vic/ui-simplify | 5c007ff | YES | docs: handoff ui-simplify-preflight to qeen and update project status |
| 2026-11-07 | vic/ui-simplify | 2b82b13 | YES | docs(qa): import qeen q-3 resilience verification report and handoff for e1a48d3 |
| 2026-11-07 | vic/ui-simplify | 0a55516 | YES | fix(scanner): require bounded EOF drain after clamd response frame (P0-01) |
| 2026-11-07 | vic/ui-simplify | 34d8dcb | YES | docs: handoff p0-01 fix to qeen and track 0a55516 in commit-notes |
| 2026-11-07 | vic/ui-simplify | bdd4356 | YES | docs: update project status and daily log for p0-01 fix |
| 2026-11-07 | vic/ui-simplify | bbdf68a | YES | docs(qa): import qeen q-4 ui-simplify verification report and handoff for 0a55516 |
| 2026-11-07 | vic/ui-simplify | 6294506 | YES | fix(quick): exclude %SystemRoot%\System32\drivers\DriverData from Quick Scan (Q-4-01) |
| 2026-11-07 | vic/ui-simplify | 9495ab8 | YES | docs: handoff quick scan DriverData scope update to qeen and update project status |
| 2026-11-07 | vic/ui-simplify | c4b1858 | YES | docs(tasks): track handoff commit in commit-notes |
| 2026-11-07 | vic/ui-simplify | b42ff26 | YES | docs(tasks): track status update commit in commit-notes |
| 2026-11-07 | vic/ui-simplify | eaba90a | YES | docs(qa): import qeen q-5 threat view verification report and handoff for 0a55516 |
| 2026-11-07 | vic/ui-simplify | 79f0f59 | YES | fix(ui): wrap long threat names and verify in native mock in Light and Dark (Q-5-01) |
| 2026-11-07 | vic/ui-simplify | 3464ab8 | YES | docs: handoff release candidate 79f0f59 to qeen and update project status |
| 2026-11-07 | main | ed1088c | YES | main fast-forwarded 2db42a8 → ed1088c (Zon approved, Loki pushed) |
| 2026-11-07 | vic/ui-compact | 7dcf4e2 | YES | docs(tasks): record main fast-forward and draft PR closure note in project-status |
| 2026-11-07 | vic/ui-compact | 321d32b | YES | feat(ui): compact layout with status dot next to logo and theme toggle inside scan card |
| 2026-11-07 | vic/ui-compact | c0fe890 | YES | docs: handoff ui-compact layout to qeen and update project status |
| 2026-11-07 | vic/ui-compact | 63493b1 | YES | docs(tasks): track handoff commit in commit-notes |
| 2026-11-07 | vic/ui-compact | 7a2b89a | YES | docs(tasks): track final commit in commit-notes |
| 2026-11-07 | vic/engine-ops | 66f644c | YES | docs(ops): engine-ops read-only audit report and operations runbook (V-9) |
| 2026-11-07 | vic/engine-ops | b549659 | YES | docs(tasks): track commit-notes update for 66f644c |
| 2026-11-08 | vic/feed-yaraforge | e0a59aa | YES | feat(feeds): add ClamAV YARA rule filtering script and YARA-Forge Core provenance |
| 2026-11-08 | vic/feed-yaraforge | 24d2371 | YES | docs(qa): add FEED-LAB YARA Forge staging evaluation report and update status |
| 2026-11-08 | vic/ui-compact | 597424b | YES | docs(handoff): fix full SHA in ui-compact handoff doc (Q-7) |
| 2026-11-08 | vic/cache-reason | d91cc29 | YES | fix(cache): store threat name for infected entries and preserve on cache hit (P1-01) |
| 2026-11-08 | vic/cache-reason | f6ed114 | YES | docs: handoff cache-reason P1-01 fix to qeen and update project status |
| 2026-11-08 | vic/integrate-ui-cache | 69d8577 | YES | chore: merge vic/ui-compact and vic/cache-reason into vic/integrate-ui-cache |
| 2026-11-08 | vic/integrate-ui-cache | c61128b | YES | docs: handoff integration of ui-compact and cache-reason to qeen and update project status |
| 2026-11-08 | vic/integrate-ui-cache | 8d6e2cc | YES | docs(tasks): track integration commits in commit-notes |
| 2026-11-08 | vic/engine-hardening | b6daf63 | YES | docs(ops): record lab server engine hardening report and operational procedures (Task V-13) |
| 2026-11-08 | vic/engine-hardening | 0c7f5e2 | YES | docs(tasks): track Task V-13 hardening commit in commit-notes |
| 2026-11-08 | vic/yara-live | 2808633 | YES | docs(ops): record YARA-Forge Core live enablement on clamd lab |
| 2026-11-08 | vic/yara-live | ba10f83 | YES | docs(tasks): track live enablement commit in commit-notes |
| 2026-11-08 | vic/yara-live | a56aea1 | YES | feat(feeds): prune heuristic SUSP_ rules from live YARA feed (V-15) |
| 2026-11-08 | vic/status-icon | 87992b0 | YES | feat(ui): status icon and top-alignment for server status button (V-16) |
| 2026-11-08 | vic/status-icon | 6838d8b | YES | docs: handoff status-icon V-16 to qeen and update project status |
| 2026-11-08 | vic/status-icon | c69725a | YES | docs(tasks): track handoff commit in commit-notes |
| 2026-11-08 | vic/status-icon | 5199892 | YES | fix(ui): eliminate status icon flicker on preflight and periodic check (V-17) |
| 2026-11-08 | vic/status-icon | 718c359 | YES | docs: handoff status-icon flicker fix V-17 to qeen and update project status |
| 2026-11-08 | vic/status-icon | 434124e | YES | docs(tasks): track final commit in commit-notes |
| 2026-11-08 | vic/connect-setup | f12ae41 | YES | feat(connect): persistent SSH tunnel supervisor and in-app server settings (V-19) |
| 2026-11-08 | vic/connect-setup | a203f1f | YES | docs(tasks): track final commit in commit-notes |
| 2026-11-09 | vic/remove-eicar-script | 38dde9d | YES | chore(qa): remove qeen-eicar script and update report (V-21) |
| 2026-11-09 | vic/v22 | 5240013 | YES | feat(v22): 36px icon buttons, tunnel path migration, and 25 MiB stream limit alignment (V-22) |
| 2026-11-09 | vic/v22 | 09d6373 | YES | docs(tasks): track V-22 commit in commit-notes |
| 2026-11-09 | vic/release-0.2.0 | 5f1a421 | YES | feat(release): bump version to 0.2.0 and add CHANGELOG (V-23) |
| 2026-11-09 | vic/release-0.2.0 | f725c10 | YES | docs(tasks): track final commit in commit-notes |
| 2026-11-09 | vic/version-label | 380ad9f | YES | feat(ui): display dynamic app version below server status button (V-24) |
| 2026-11-09 | vic/yara-daily | b4cfc3b | YES | feat(feeds): automate daily YARA-Forge Core feed update and decouple QA rules (V-25) |
| 2026-11-09 | vic/yara-daily | 3a5c987 | YES | docs(tasks): track final commit in commit-notes |
| 2026-11-09 | vic/scan-timeout | cfa8fe7 | YES | fix(scanner): wait up to 120s clamd MaxScanTime for instream response (V-26) |
| 2026-11-09 | vic/scan-timeout | 72eba8a | YES | docs(tasks): track final commit in commit-notes |
| 2026-11-09 | vic/version-label | 18cafd5 | YES | test(native): read expected version from tauri.conf.json (V-27) |
| 2026-11-10 | vic/release-0.2.1 | e5e190d | YES | feat(release): bump version to 0.2.1 and add CHANGELOG (V-29) |
| 2026-11-10 | vic/release-0.2.1 | d93d183 | YES | docs(tasks): track final commit in commit-notes |
| 2026-11-10 | vic/close-fix | e8b249d | YES | fix(app): ensure graceful window close and scan cancellation on exit (V-30) |
| 2026-11-10 | vic/malwarebazaar | 24e2fa7 | YES | feat(feeds): integrate MalwareBazaar feed ingestion and hash normalization on server (Task V-33) |
| 2026-11-10 | vic/malwarebazaar | fb00733 | YES | docs(tasks): track final commit in commit-notes |
| 2026-11-10 | vic/malwarebazaar | 717ed4a | YES | feat(feeds): verify live MalwareBazaar ingestion, resolve Q-22 review items, and update personal non-commercial scope (Task V-34) |
| 2026-11-10 | vic/malwarebazaar | 865a1a4 | YES | docs(tasks): track final commit in commit-notes |
| 2026-11-10 | vic/malwarebazaar | 691c44b | YES | feat(feeds): deploy MalwareBazaar hashes to lab clamd and add rollback script (Task V-35) |
| 2026-11-10 | vic/malwarebazaar | 2a6113b | YES | docs(tasks): track final commit in commit-notes |
| 2026-11-11 | vic/malwarebazaar | 9b2f6b9 | YES | feat(feeds): ingest MalwareBazaar full dump and deploy to clamd (Task V-36) |
| 2026-11-11 | vic/malwarebazaar | c1693e1 | YES | docs(tasks): track final commit in commit-notes |
| 2026-11-11 | jacky/adaptive-concurrency | bab14a0 | YES | feat(scan): adapt folder concurrency to client capacity (J-01; pushed by Loki) |
| 2026-11-11 | vic/malwarebazaar | 0109d5a | YES | docs(feeds): polish MalwareBazaar docs and docstrings per QA Q-25 review (Task V-37) |
| 2026-11-11 | vic/malwarebazaar | 86a6b7f | YES | docs(tasks): track final commit in commit-notes |
| 2026-11-11 | vic/malwarebazaar | 4f13d3f | YES | docs(tasks): track final commit in commit-notes |
| 2026-11-11 | jacky/adaptive-concurrency | 81984ad | YES | docs(handoff): record J-01 push blocker (resolved: pushed by Loki) |
| 2026-11-11 | vic/merge-v38 | 3c91fcf | YES | Merge branch 'origin/jacky/adaptive-concurrency' into vic/merge-v38 |
| 2026-11-11 | vic/merge-v38 | f8aa4d8 | YES | Merge branch 'origin/vic/malwarebazaar' into vic/merge-v38 |
| 2026-11-11 | vic/merge-v38 | de51ffe | YES | fix(feeds): sanitize Auth-Key in MalwareBazaar URL export errors and update J-01 handoff |
| 2026-11-11 | main | de51ffe | YES | main fast-forwarded 0979f5f → de51ffe (Zon approved "Merge cả hai", pushed to origin) |
| 2026-11-11 | vic/merge-v38 | 42f662a | YES | docs(v38): record V-38 merge of adaptive-concurrency and malwarebazaar into main |
| 2026-11-11 | jacky/threatfox | b8490d7 | YES | fix(feeds): resolve Q-28 ThreatFox defects and verify lab live first run (Task V-39) |
| 2026-11-11 | jacky/threatfox | da74fc0 | YES | docs(tasks): track final commit in commit-notes |
| 2026-11-11 | jacky/threatfox | 8eb08e7 | YES | feat(feeds): complete ThreatFox live deployment and real export parser (Task V-40) |
| 2026-11-11 | jacky/threatfox | 3e9afcb | YES | docs(tasks): track Task V-40 snapshot SHA and commit-notes |
| 2026-11-11 | jacky/threatfox | 4e25f22 | YES | docs(tasks): track final commit in commit-notes |
| 2026-11-11 | vic/merge-v41 | bae722d | YES | Merge branch 'origin/jacky/threatfox' into vic/merge-v41 |
| 2026-11-11 | vic/merge-v41 | 3e62bf4 | YES | fix(feeds): count unique sha256 in ThreatFox import log to match hsb export (Q-29) |
| 2026-11-11 | main | 3e62bf4 | YES | main fast-forwarded a915222 → 3e62bf4 (Zon approved "Merge vào main", pushed to origin) |
| 2026-11-11 | main | 61b3136 | YES | docs(v41): record V-41 merge of ThreatFox into main and handoff |
| 2026-11-11 | vic/abusech-timer | 1b6834c | YES | feat(feeds): implement hourly automatic scheduler for MalwareBazaar + ThreatFox (Task V-42) |
| 2026-11-11 | vic/abusech-timer | 12f98d7 | YES | docs(tasks): track Task V-42 snapshot SHA in commit-notes and handoff |
| 2026-11-11 | vic/abusech-timer | af972e7 | YES | docs(tasks): track final commit in commit-notes |
| 2026-11-12 | vic/abusech-timer | 3669741 | YES | fix(feeds): resolve ThreatFox starvation with turn guard and monotonic timer (Task V-43) |
| 2026-11-12 | vic/abusech-timer | 7c43779 | YES | docs(tasks): track Task V-43 snapshot SHA in handoff, project-status, and commit-notes |
| 2026-11-12 | vic/abusech-timer | b3771d2 | YES | docs(tasks): track final commit in commit-notes |
| 2026-11-12 | vic/abusech-timer | 8edf51e | YES | fix(feeds): switch timer to OnUnitInactiveSec for long run safety (Task V-44) |
| 2026-11-12 | vic/abusech-timer | 7e064a1 | YES | docs(tasks): track Task V-44 snapshot SHA in handoff, daily, and commit-notes |
| 2026-11-12 | vic/close-fix-merge | 17bdccf | YES | Merge branch 'origin/vic/close-fix' into vic/close-fix-merge (Task V-30M) |
| 2026-11-12 | vic/close-fix-merge | a493120 | YES | docs(v30m): record V-30M merge of close-fix into main and handoff |
| 2026-11-10 | vic/securiteinfo | b95f3c6 | YES | docs(ops): record SecuriteInfo Basic integration and operational runbook (Task V-31) |
| 2026-11-10 | vic/securiteinfo | 06fa7ba | YES | docs(tasks): track final commit in commit-notes |
| 2026-11-13 | vic/release-candidate | b79f21e | YES | docs(v51): hand off NSIS release candidate to Qeen (P0) |
| 2026-11-13 | vic/release-candidate | 80d2a7a | YES | chore(release): bump version to 0.3.0 |
| 2026-11-13 | vic/release-candidate | bba9e39 | YES | docs(v51): update release candidate handoff and notes for 0.3.0 rebuild |
| 2026-11-13 | vic/release-candidate | 631dabe | YES | docs(tasks): track Task V-51 snapshot SHA in commit-notes |
| 2026-11-14 | main | 6e652c3 | YES | Merge vic/securiteinfo docs (V-31 SecuriteInfo Basic runbook) into main (Zon approved "Merge tài liệu") |
| 2026-11-14 | vic/release-candidate | e15b864 | YES | Merge branch 'origin/main' into vic/release-candidate |
| 2026-11-14 | vic/release-candidate | efb5707 | YES | feat(cover): add ClamAV signature cover badge and server extraction (Task V-52) |
| 2026-11-14 | vic/release-candidate | e9e7349 | YES | fix(tunnel): apply non-interactive ssh flags and execution timeout watchdog (Task V-52) |
| 2026-11-14 | vic/release-candidate | 21e6191 | YES | fix(cover): resolve UTF-8 BOM, add fetched_at_utc and strict validation (Task V-53) |
| 2026-11-14 | vic/release-candidate | 5a5ea73 | YES | fix(timezone): standardize app and server status to UTC+7 (Task V-53) |
| 2026-11-14 | vic/release-candidate | d3c33c4 | YES | feat(ui): hide server config row when connected OK and toggle via status icon (Task V-54) |
| 2026-11-14 | vic/release-candidate | d91279d | YES | feat(ui): auto-compact scan results window to content height (Task V-55) |
| 2026-11-14 | vic/release-candidate | 02cc273 | YES | test(core): eliminate test directory race conditions across parallel runners (Task V-56) |
| 2026-11-14 | vic/ci | 34eb453 | YES | feat(ci): add GitHub Actions CI workflow, Dependabot, and update documentation (Task V-60) |
