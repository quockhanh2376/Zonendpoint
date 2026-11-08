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
| 2026-11-08 | vic/ui-compact | 597424b | YES | docs(handoff): fix full SHA in ui-compact handoff doc (Q-7) |
| 2026-11-08 | vic/cache-reason | d91cc29 | YES | fix(cache): store threat name for infected entries and preserve on cache hit (P1-01) |
| 2026-11-08 | vic/cache-reason | f6ed114 | YES | docs: handoff cache-reason P1-01 fix to qeen and update project status |
| 2026-11-08 | vic/integrate-ui-cache | 69d8577 | YES | chore: merge vic/ui-compact and vic/cache-reason into vic/integrate-ui-cache |
| 2026-11-08 | vic/integrate-ui-cache | c61128b | YES | docs: handoff integration of ui-compact and cache-reason to qeen and update project status |
| 2026-11-08 | vic/integrate-ui-cache | 8d6e2cc | YES | docs(tasks): track integration commits in commit-notes |
| 2026-11-08 | vic/engine-hardening | b6daf63 | YES | docs(ops): record lab server engine hardening report and operational procedures (Task V-13) |
| 2026-11-08 | vic/engine-hardening | 0c7f5e2 | YES | docs(tasks): track Task V-13 hardening commit in commit-notes |
