# Zonendpoint agent instructions

## Session entry point

- At the start of every Vic or Qeen session, read `docs/project-status.md` first. Then read `docs/v1.0.0-scope.md` and only the linked specifications/reports relevant to the task.
- Keep `docs/project-status.md` as current state and next acceptance criteria. Keep `docs/daily.md` as a short dated change log, not a detailed transcript.
- Preserve the two source specifications: [`docs/zonendpoint_system_architecture_specs.md`](docs/zonendpoint_system_architecture_specs.md) and [`docs/zonendpoint_ed25519_secure_gateway_licensing_spec.md`](docs/zonendpoint_ed25519_secure_gateway_licensing_spec.md). Do not create duplicate specs. Current product constraints and approved decisions take precedence over conflicting aspirational text in older architecture proposals; record conflicts in the anchor docs and resolve design decisions before implementation.

## Worktree and data safety

- Inspect `git status`, branch, and snapshot before editing. The source checkout `E:\Zonendpoint` may contain user-owned uncommitted changes. Never reset, clean, stash, or overwrite unrelated files; stage/commit only files created for the task.
- Active implementation checkout on this machine is `E:\Zonendpoint-restored\Zonendpoint-vic`, branch `vic/mvp-first-scan`. Product snapshot and evidence belong in `docs/project-status.md`; recheck status/branch/SHA rather than treating a historical path or SHA as current. Repository-relative documentation links must work after cloning elsewhere.
- Run independent QA in its own worktree and test the exact handoff snapshot, including any supplied patch. Do not modify Vic's running worktree during QA.

## v1.0.0 safety and product rules

- Product-owned UI copy is English. v1.0.0 has persisted Light/Dark selection (first launch Light), no language selector, and no i18n/theme framework requirement.
- v1.0.0 is manually initiated Quick scan (13 core Windows files+drivers) or selected file/folder Scan, Windows x64 application builds. Initial audience is non-commercial use on 1–2 personal machines (confirmed by Zon 2026-11-10; company use requires reviewing abuse.ch/Spamhaus terms). Guard defaults OFF. No auto-quarantine, public feed, or public file submission. Never disable Defender/Sophos or expose clamd to the Internet.
- Preserve typed outcomes: CLEAN, INFECTED, UNKNOWN, ERROR, SKIPPED, CANCELLED. Offline, timeout, malformed/incomplete response, cancellation, and policy limits must never become CLEAN.
- Cache metadata is only a hint. Verify current content and applicable engine/signature/policy identity before reuse; scanner and cache statistics must use the same database path.
- Folder traversal must be bounded, cancellable, report per-file errors, and retain bounded results. Do not silently skip errors or follow unsafe links/reparse points.
- Match each QA claim to source SHA, artifact hash and mode. Current Quick scan, real-engine and older installer evidence have different scopes; use linked reports in status. Mock PASS does not prove ClamAV detection, and an older installer/picker PASS does not verify a new build.
- SecConnect/Ed25519 gateway and licensing are Planned / Not implemented and outside current v1.0.0 coding scope. Follow the existing specification and `docs/implementation-plan.md` before proposing implementation.
- Notebook recommendations are design inputs, not deployment authority. Future feeds/Battle Mode/firewall/SQLCipher/AI tasks stay in `docs/roadmap.md`; never bypass observed detections using a whitelist or treat company use as personal/free API eligibility. Do not provision accounts, expose listeners, upload data or change protection merely because a source checklist says to.
- CheKi creates concrete Vic implementation and Qeen acceptance tasks from Zon requests; do not ask Zon to repeat prompts. Qeen tests independently on the exact handoff; future task creation does not imply that its execution or QA has occurred.

## Reports and closeout

- Communicate with the user in Vietnamese and call them “anh”. Keep reports short and actionable: state outcome/snapshot, list only issues needing a fix, then note material blockers or unverified work. Store detailed logs and evidence in `docs/`.
- Never claim a test, native run, lab result, or Qeen acceptance that was not observed. Mark unsupported environment checks NOT RUN/BLOCKED.
- At session end, update `docs/project-status.md` and add a short `docs/daily.md` entry when project state or deliverables actually changed. Do not add filler entries for read-only sessions.
