# Zonendpoint MVP implementation plan

Status: proposed; implementation is gated on approval of the design in
`docs/architecture-review.md`.

## Acceptance criteria

1. Rust core compiles; frontend has no call to nonexistent `greet` and offers
   file/folder selection, start/cancel, progress, counts, and clear outcomes.
2. Typed outcomes distinguish CLEAN, INFECTED, UNKNOWN, ERROR, SKIPPED, and
   CANCELLED. Offline, timeout, malformed response, incomplete scan, and
   policy-limit cases never become CLEAN.
3. Mock ClamAV protocol tests cover framing, strict terminal parsing,
   fragmented and oversized responses, EOF/errors, timeout/cancel, and stream
   limits. Real ClamAV is marked NOT RUN if no isolated lab is available.
4. Cache uses one per-user path, hashes content before reuse, checks file
   identity/metadata before and after reading, records only completed verdicts,
   versions its schema, and fails visibly on cache errors.
5. Folder traversal has finite concurrency and retained memory, cancellation,
   progress, and a visible outcome/error for every encountered file. It does
   not follow links/reparse points outside selected scope.
6. Guard remains OFF by default. No quarantine, public feed, or external file
   submission is introduced.
7. Handoff records branch/snapshot, files, commands/results, run instructions,
   migration, limits, and NOT RUN/BLOCKED items without claiming QEEN PASS.

## Proposed sequence after approval

1. Add focused Rust tests for typed result parsing and INSTREAM framing; run
   them against an in-process mock daemon before connecting the production
   adapter.
2. Implement bounded/cancellable ClamAV client and typed file scan, with
   pre/post file metadata verification.
3. Implement the shared app-data cache path, safe versioned migration, and
   content-based cache hint policy; exercise migration and stale-content cases.
4. Implement bounded directory traversal and Tauri scan lifecycle/progress/
   cancellation commands.
5. Replace greeting UI with picker, scan controls, progress, and outcome
   summary; keep all statuses visible and Guard disabled.
6. Run frontend build, Rust fmt/check/clippy/tests, protocol mock tests, and
   native Tauri checks supported by this machine. Record all outcomes in
   `docs/daily.md` and `docs/handoffs/vic-to-qeen.md`.

## Scope deferred

Realtime Guard hardening, production server/gateway, feeds/hash sync, signed
updates, quarantine/restore, installer/signing, release operations, and
performance certification remain later phases.
