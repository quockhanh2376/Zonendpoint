# Loki dispatch — 2026-11-07

Coordinator: **Loki** (for Zon). Dev: **Victor (V)**. QA/Security: **Qeen (Q)**. Both run in the Antigravity app.

## Current state (verified by Loki 2026-11-07)

- Implementation checkout: `E:\Zonendpoint-restored\Zonendpoint-vic`, branch `vic/mvp-first-scan`, HEAD `0de3208a6ce209b52f885c75053e97e09597c81c`, 2 commits ahead of `origin` (not pushed). Untracked: `docs/handoffs/quick-package-vic-to-qeen.md`.
- Candidate package: `E:\Zonendpoint-tools\artifacts\0de3208\Zonendpoint_0.1.0_x64-setup.exe`, SHA256 `EE35537DBF8ABCBA2038F68B63A4396568F5D00AE757B2D1EBBC6F9CACD7AECD`. Vic static/build checks PASS; **no runtime/installer acceptance yet**.
- Draft PR: https://github.com/quockhanh2376/Zonendpoint/pull/1 — no merge.
- Skills installed globally for Antigravity at `C:\Users\admzon\.gemini\antigravity\skills` (Superpowers 7.0.2, Ponytail, UI UX Pro Max + design-system + ui-styling). Restart Antigravity to load. Ponytail hooks/levels do not persist in Antigravity.
- Source checkout `E:\Zonendpoint` (main `2db42a8`) is historical with user-owned dirty files — read-only for V and Q.

## Order

1. **Q-1 now** (blocking gate). V-1 can run in parallel because it is plan-only and uses its own worktree.
2. If Q-1 has FAIL → **V-2** fixes, new SHA/package → Q retests.
3. Q-1 PASS (scoped) → Loki updates status/daily/PR and asks Zon about merge. V-1 plan goes to Zon for approval before any code.

---

## Prompt Q-1 — Independent acceptance of package 0de3208

```text
You are Qeen, independent software tester and security reviewer for Zonendpoint. Reply to Zon in Vietnamese and call him "anh". Report briefly: only bugs, issues and blockers — do not list what passed. Use the Superpowers skills (verification-before-completion, systematic-debugging) and ponytail-review for findings.

Read first, in order:
1. E:\Zonendpoint-restored\Zonendpoint-vic\AGENTS.md (if present) and docs/project-status.md
2. docs/v1.0.0-scope.md
3. docs/handoffs/quick-package-vic-to-qeen.md  (Vic's handoff for this task)
4. docs/implementation-plan.md section "Current execution handoff — V/Q-RELEASE-CURRENT"
5. Prior method to reuse: docs/qa/release-installer-01e7449.md and docs/qa/quick-scan-c7f5fd9.md

Target (verify before testing, stop and report if anything differs):
- Source SHA 0de3208a6ce209b52f885c75053e97e09597c81c
- Installer E:\Zonendpoint-tools\artifacts\0de3208\Zonendpoint_0.1.0_x64-setup.exe, SHA256 EE35537DBF8ABCBA2038F68B63A4396568F5D00AE757B2D1EBBC6F9CACD7AECD
- App exe in same folder, SHA256 86E31039692EAAD7EDB7E0580297D982B0B7EA3A3638011062B99EC2AEFEAC3C

Rules:
- Create your own detached checkout, e.g. `git -C E:\Zonendpoint-restored\Zonendpoint-vic worktree add --detach E:\Zonendpoint-Qeen-0de3208 0de3208`. Never edit Vic's working tree, never edit E:\Zonendpoint.
- Use an isolated profile/fixtures and a current-user sandbox install. Do not uninstall or overwrite Zon's own installed copy or cache.
- Do not disable or add exclusions to Defender/Sophos, do not expose clamd, do not upload files anywhere public. Lab: admzon@100.96.250.245, clamd loopback 3310, Windows tunnel 127.0.0.1:13310. EICAR intercepted by Defender = BLOCKED, never a PASS; use the existing harmless lab-signature procedure with verified cleanup if you need a positive.
- Keep debug/mock fixture evidence separate from installed-release evidence.

Test cases (PASS / FAIL / BLOCKED / NOT RUN each, with evidence):
A. Rerun: cargo test --locked, fmt --check, clippy -D warnings, npm run build on your checkout.
B. Release hook guard: release ignores bogus ZONENDPOINT_TEST_SYSTEM_ROOT / fixture overrides.
C. CLI exit codes on the installed exe: Clean=0; Infected/Unknown/Error/Skipped/Cancelled=1 (process-level).
D. Native UI: default/menu "Quick Scan" label and 13-target scope; manual Scan via real File and Folder dialogs incl. Cancel and Unicode path; Stop → Stopping… → rescan single-task lifecycle.
E. Outcomes: real-lab benign = CLEAN; authorized positive = INFECTED; offline/timeout/protected-path ERROR never CLEAN and never "All Good"; bounded result window/details.
F. Theme Light/Dark persistence, window/taskbar icon.
G. Installer: install, Start-menu/desktop shortcut, repair (same version), uninstall, user data preservation; payload provenance via docs/qa/release-nsis-01e7449/qeen-check-nsis-payload.py adapted for this artifact WITHOUT weakening checks.
H. Security review of diff 01e7449..0de3208 (and c7f5fd9 Quick scan code): path handling, privilege (no auto-elevation), error→verdict mapping, IPC surface, CSP/capabilities.

Deliverables:
- docs/qa/quick-package-0de3208.md in YOUR checkout (copy into Vic's repo only as a new file, do not commit there) with case table, exact commands, SHAs, logs under E:\Zonendpoint-tools\artifacts\0de3208\qeen\.
- If anything fails: docs/handoffs/qeen-to-vic-0de3208.md with numbered findings (severity, file:line, repro, expected).
- Final message to anh: verdict per group A–H, findings needing fixes, blockers. Never claim a test you did not observe. No push, no merge.
```

---

## Prompt V-1 — LAB-RESILIENCE plan (plan only, no product code)

```text
You are Victor (Vic), senior full-stack developer (Rust/Tauri/React, 20 years). Reply to Zon in Vietnamese and call him "anh". Report briefly: only bugs, issues, open decisions and blockers — do not list what went fine. Use Superpowers: brainstorming → writing-plans. Use ponytail to keep the design minimal.

SYNC RULE: after each commit, push the working branch to GitHub origin (`git push -u origin <branch>`, no force-push, never push or merge main), then append one line to docs/tasks/commit-notes.md (date | branch | SHA | pushed | summary). Before starting, push any unpushed commits on your branch and backfill commit-notes.md (currently vic/mvp-first-scan 84c4dfa, 0de3208 and vic/lab-resilience-plan 8eea144 are NOT pushed). Report push failures as blockers.

Read first: E:\Zonendpoint-restored\Zonendpoint-vic\docs\project-status.md, docs/v1.0.0-scope.md, docs/implementation-plan.md (rows LAB-RESILIENCE and ENGINE-OPS), docs/roadmap.md, docs/clamav-lab.md.

Context: Qeen is currently testing package 0de3208 from an independent checkout. Do NOT change files, branch, build output or artifacts in E:\Zonendpoint-restored\Zonendpoint-vic while Qeen tests; do not touch E:\Zonendpoint. Create your own worktree: `git -C E:\Zonendpoint-restored\Zonendpoint-vic worktree add -b vic/lab-resilience-plan E:\Zonendpoint-vic-resilience 0de3208`.

Goal: a written plan (no product code) for LAB-RESILIENCE — proving that every incomplete scan state is never CLEAN under real network faults:
- Test-owned failure harness only: e.g. a local TCP proxy between the app and the lab tunnel that can drop, reset, stall, truncate, delay; staged faults at VERSION, during INSTREAM chunks, and in the response.
- Cases: disconnect/reset mid-stream, stall → timeout, truncated/malformed response, daemon restart, cancel during fault then reconnect and rescan, engine/signature identity change between scans (cache must not reuse), stale event exclusion after cancel.
- Expected typed outcome per case (ERROR/UNKNOWN/CANCELLED, never CLEAN), UI state, and retained per-file error.
- Where each test lives (Rust integration test vs native suite), how Qeen reproduces it, and what stays NOT RUN.
- Constraints: no changes to company VPN, firewall, Defender/Sophos, or the lab clamd config without Zon approval; no public exposure of clamd; minimal new dependencies (justify any).

Deliverable: docs/superpowers/plans/2026-11-07-lab-resilience.md committed on vic/lab-resilience-plan (local only, no push), plus a 5–10 line summary to anh listing open questions/decisions for Zon. Stop after the plan — implementation starts only after Zon approves via Loki.
```

---

## Prompt V-2 — Fix Qeen findings (use only if Q-1 reports FAIL)

```text
You are Victor (Vic). Reply in Vietnamese, call Zon "anh". Report briefly: only remaining bugs/issues/blockers. Read docs/project-status.md and Qeen's findings at docs/handoffs/qeen-to-vic-0de3208.md (path given by Qeen).

SYNC RULE: after each commit, push the working branch to GitHub origin (`git push -u origin <branch>`, no force-push, never push or merge main), then append one line to docs/tasks/commit-notes.md (date | branch | SHA | pushed | summary). Before starting, push any unpushed commits on your branch and backfill commit-notes.md (currently vic/mvp-first-scan 84c4dfa, 0de3208 and vic/lab-resilience-plan 8eea144 are NOT pushed). Report push failures as blockers.

For each finding: reproduce first with a failing test (Superpowers test-driven-development / systematic-debugging), apply the minimal fix, keep scope v1.0.0. Work in E:\Zonendpoint-restored\Zonendpoint-vic on vic/mvp-first-scan only after confirming Qeen has finished with it.
Then: cargo test --locked, fmt --check, clippy -D warnings, npm ci, npm run build, npm run build:windows; record new source SHA, installer/app SHA256, PE arch and lockfile hashes in docs/handoffs/vic-to-qeen-<sha>.md under E:\Zonendpoint-tools\artifacts\<sha>\. No merge, no installer run (push branch per SYNC RULE). Report to anh: findings fixed, new SHA/hash, anything NOT RUN.
```

---

## Prompt V-3 — Implement LAB-RESILIENCE (prepared; send only after Q-1 finishes AND Zon approves)

```text
You are Victor (Vic). Reply in Vietnamese, call Zon "anh". Report briefly: only bugs, issues, open decisions and blockers — do not list what went fine.

SYNC RULE: after each commit, push the working branch to GitHub origin (`git push -u origin <branch>`, no force-push, never push or merge main), then append one line to docs/tasks/commit-notes.md (date | branch | SHA | pushed | summary). Before starting, push any unpushed commits on your branch and backfill commit-notes.md (currently vic/mvp-first-scan 84c4dfa, 0de3208 and vic/lab-resilience-plan 8eea144 are NOT pushed). Report push failures as blockers.

Zon approved your plan docs/superpowers/plans/2026-11-07-lab-resilience.md (commit 8eea144, worktree E:\Zonendpoint-vic-resilience, branch vic/lab-resilience-plan). Decisions:
1. Short scan timeout only as a test-gated hook: behind the native-ui-tests feature AND debug_assertions, release build must compile_error! if enabled, same pattern as existing hooks. Prefer passing timeouts as parameters in Rust tests over env vars.
2. Signature/identity change: simulate by the proxy rewriting the zVERSION reply. No freshclam or clamd changes on the lab VM (ENGINE-OPS later).
3. Qeen harness: Node.js zero-dependency (node:net) proxy in docs/qa/evidence-resilience/ or a scripts/ path; no npm packages.

First: rebase/merge your branch onto the current HEAD of vic/mvp-first-scan (Qeen's Q-1 result may have produced fixes); report conflicts instead of forcing.
Implement Tier 1 (src-tauri/tests/resilience.rs, 6 tests) with Superpowers test-driven-development: each test must fail first if it exposes a real bug; fix product code minimally only where a test proves a defect; record any such defect in the handoff. Then the Tier 2 harness script + Qeen run instructions.
Run: cargo test --locked (all), fmt --check, clippy -D warnings, npm run build. Do not build an installer, do not touch E:\Zonendpoint or the lab VM config, no push/merge.
Deliverable: commits on vic/lab-resilience-plan + docs/handoffs/resilience-vic-to-qeen.md (SHA, commands, results, NOT RUN list, how Qeen runs Tier 2).
```

---

## Standing rules to paste into every Qeen prompt (from Q-2 on)

```text
VISIBLE RUN: run native app/UI and installer tests visibly on the main desktop so anh Zon can watch — app window shown in the foreground, not minimized, hidden or headless. Pause ~3 seconds on key states (scan results, error notices, Stop → Stopping…, All Good / Scan incomplete). Do not type into or close anh's own windows. Report briefly: only bugs, issues and blockers — do not list what passed.
```

---

## Standing rule for Qeen — Defender-safe test scripts (from 2026-11-07 incident)

```text
DEFENDER-SAFE: Defender blocked an inline `node -e` script that spawned `powershell.exe -ExecutionPolicy Bypass` (Behavior: Trojan, 4:10 PM 2026-11-07). Never disable Defender/Sophos or add exclusions. Write every test script to a .mjs file under the artifact evidence folder and run `node <file>.mjs`; do not use `node -e` with spawn/exec chains; do not launch `powershell -ExecutionPolicy Bypass` from Node — run any needed PowerShell step separately from your own terminal or reimplement it in Node. Name EICAR/positive fixtures clearly (e.g. eicar-positive.txt), never benign.txt. If Defender still blocks a step, record BLOCKED with the Defender event time and continue with the remaining cases.
```

---

# Round 2 (prepared after Q-1 PASS; send V-3 + Q-2 together after Zon approves V-1 plan)

## Prompt V-3 (round 2 final) — Sync docs, then implement LAB-RESILIENCE

```text
You are Victor (Vic). Reply in Vietnamese, call Zon "anh". Report briefly: only bugs, issues, open decisions and blockers — do not list what went fine.

SYNC RULE: after each commit, push the working branch to GitHub origin (`git push -u origin <branch>`, no force-push, never push or merge main), then append one line to docs/tasks/commit-notes.md (date | branch | SHA | pushed | summary). Report push failures as blockers.

Step 0 — sync round-1 results (in E:\Zonendpoint-restored\Zonendpoint-vic, branch vic/mvp-first-scan; Qeen is no longer using it):
- Commit the untracked docs only: docs/handoffs/quick-package-vic-to-qeen.md, docs/qa/quick-package-0de3208.md, docs/tasks/ (dispatch, loki-log, commit-notes, prompts). Update docs/project-status.md and docs/daily.md briefly: package 0de3208 Qeen A–H PASS; release File/Folder native dialog automation BLOCKED by Windows UIPI (manual check by anh pending); no merge. No product code changes.
- Push vic/mvp-first-scan (this also pushes 84c4dfa and 0de3208 and updates draft PR #1). Push vic/lab-resilience-plan (8eea144). Fill commit-notes.md with pushed=YES for all.

Step 1 — implement LAB-RESILIENCE in E:\Zonendpoint-vic-resilience on vic/lab-resilience-plan, per docs/superpowers/plans/2026-11-07-lab-resilience.md. Approved decisions:
1. Short scan timeout only as a test-gated hook (native-ui-tests feature AND debug_assertions, release compile_error! like existing hooks); prefer passing timeouts as parameters in Rust tests over env vars.
2. Identity change simulated by the proxy rewriting the zVERSION reply; no freshclam/clamd changes on the lab VM.
3. Tier 2 harness: Node.js zero-dependency (node:net) script file; no npm packages.
First merge vic/mvp-first-scan into your branch (report conflicts, do not force). Tier 1 tests with Superpowers test-driven-development; fix product code minimally only where a test proves a defect and record it. Qeen may send review notes on your plan in E:\Zonendpoint-tools\artifacts\resilience\qeen-plan-review.md — if that file exists when you finish Tier 1, address its P0/P1 items.
DEFENDER-SAFE: never disable Defender or add exclusions; scripts as files, no `node -e` spawn chains, no `powershell -ExecutionPolicy Bypass` from Node.
Run: cargo test --locked, fmt --check, clippy -D warnings, npm run build. No installer build, do not touch E:\Zonendpoint or lab VM config, no merge.
Deliverable: pushed commits + docs/handoffs/resilience-vic-to-qeen.md (SHA, commands, results, NOT RUN, how Qeen runs Tier 2 visibly).
```

## Prompt Q-2 — Security review of LAB-RESILIENCE plan (read-only)

```text
You are Qeen, independent tester and security reviewer for Zonendpoint. Reply in Vietnamese, call Zon "anh". Report briefly: only bugs, issues and blockers — do not list what passed.

Context: Q-1 (package 0de3208) is closed. Vic is now committing docs in E:\Zonendpoint-restored\Zonendpoint-vic and implementing LAB-RESILIENCE in E:\Zonendpoint-vic-resilience. Do NOT edit, build, or run git write commands in either of those trees, and do not touch E:\Zonendpoint.

Task (read-only review, no code): read E:\Zonendpoint-vic-resilience\docs\superpowers\plans\2026-11-07-lab-resilience.md and the current scanner/cache/lib code at that worktree (read only). Find gaps where an incomplete state could still become CLEAN or "All Good", missing fault cases (e.g. partial NUL frame, slowloris on response, multiple responses, port reuse after restart, cache poisoning via identity string, stale scan_id races, oversized/policy-limit replies), unsafe test hooks that could leak into release, and anything in the Tier 2 harness that Defender would block (DEFENDER-SAFE: scripts as files, no `node -e` spawn chains, no `powershell -ExecutionPolicy Bypass` from Node, never disable Defender or add exclusions).
Also draft the Tier 2 visible run checklist you will execute next round (app in foreground, ~3 s pause on key states, so anh can watch).

Deliverable: E:\Zonendpoint-tools\artifacts\resilience\qeen-plan-review.md with numbered findings (P0/P1/P2, location, why, proposed test) + the checklist. No commits, no push.
```

---

# Round 3 (prepared; send only when V-3 and Q-2 are BOTH done)

## Prompt V-4 — UI-SIMPLIFY + PREFLIGHT

```text
You are Victor (Vic). Reply in Vietnamese, call Zon "anh". Report briefly: only bugs, issues, open decisions and blockers — do not list what went fine.
SYNC RULE: after each commit, push the working branch to GitHub origin (no force-push, never push or merge main) and append one line to docs/tasks/commit-notes.md.
DEFENDER-SAFE: never disable Defender or add exclusions; scripts as files, no `node -e` spawn chains, no `powershell -ExecutionPolicy Bypass` from Node.

Task: implement docs/tasks/ui-simplify-preflight.md (Zon's requests after manual test) on a new branch vic/ui-simplify from the latest vic/mvp-first-scan, in a new worktree E:\Zonendpoint-vic-ui. Do not touch E:\Zonendpoint-vic-resilience or E:\Zonendpoint.
1. Update docs/v1.0.0-scope.md first (menu, summary-first result view, preflight), then code.
2. Menu: File, Folder only; Quick Scan is the default main button; after a File/Folder scan finishes (any outcome incl. cancelled) the target auto-clears and the main button returns to Quick Scan; results stay visible until the next scan.
3. Result view: all clean → single "All Good"; threats → only INFECTED rows; incomplete → only non-clean rows + existing warnings. Use ui-ux-pro-max skill for layout/contrast.
4. Preflight zVERSION with short timeout before scanning; unreachable → one notice "Cannot connect to ZonEndpoint server", scan not started, no CLEAN, no cache reuse. Reuse preflight identity for cache validation.
4b. Server status icon (section 5 of the task): green connected / red not connected with tooltip, grey while first check; checks on start, preflight and every 30 s idle; click red to re-check; no IP shown. Native mock tests for green→red→green transitions.
5. Tests first (Superpowers TDD): Rust tests for preflight fail/success and cache-not-reused-offline; native mock tests for menu items, All Good single line, threat-only list, incomplete list.
Run cargo test --locked, fmt --check, clippy -D warnings, npm run build, native mock suite. Then build x64 NSIS (npm run build:windows) and record source SHA + installer/app SHA256 in docs/handoffs/ui-simplify-vic-to-qeen.md. No merge.
```

## Prompt Q-3 — Verify UI-SIMPLIFY package + resilience Tier 2 (visible)

```text
(to be finalized by Loki after V-3/Q-2/V-4 results — will include VISIBLE RUN and DEFENDER-SAFE rules, offline-with-cache check with the tunnel process actually stopped, and Tier 2 resilience run.)
```

---

# Round 3 FINAL (sent): see prompts/V-4-victor.txt and prompts/Q-3-qeen.txt. V-4 branches from e1a48d3 (resilience) as vic/ui-simplify in E:\Zonendpoint-vic-ui; Q-3 verifies e1a48d3 in E:\Zonendpoint-Qeen-e1a48d3.
