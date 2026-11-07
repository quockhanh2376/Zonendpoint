# Zonendpoint MVP implementation plan

Updated2026-11-07. The original first-scan sequence below is retained as historical implementation context; it is implemented to the observed scope in [project status](project-status.md), not a fresh approval-pending task. The original [architecture review](architecture-review.md) describes the old baseline. Current ordering is in [roadmap](roadmap.md). New future features remain design-gated; do not interpret historical proposals or the Notebook checklist as permission to implement/deploy all of them.

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

## Historical first-scan sequence

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

Realtime Guard, production gateway, feeds/hash sync, signed updates,
quarantine/restore, signing/distribution and performance certification remain later phases. Unsigned NSIS packaging was observed on01e7449; a new Quick-scan package is not yet verified. See current status rather than treating all packaging as NOT RUN.

## Later-phase backlog — Secure Gateway and licensing

Status: **Planned / Not implemented**. This is not part of v1.0.0.

Input: `docs/zonendpoint_ed25519_secure_gateway_licensing_spec.md` — Ed25519 device challenge-response, signed time-limited license tokens, a Rust `zon-gateway` service/CLI, server-side revocation/renewal, and proxying authenticated ClamAV traffic.

Dependencies include a separate gateway service and deployment, Ed25519 key/token lifecycle, secure client-key storage, registry/config management, and a gateway-aware ClamAV transport in the Tauri client. `ed25519-dalek` v2 is specified but is not a current dependency. No gateway or licensing code/dependency has been added.

Architecture conflicts and decisions to resolve before implementation:

- The current client connects directly to clamd on a private/loopback address, performs `zVERSION`, then opens a scan stream. The proposed gateway uses a challenge/auth packet before ClamAV passthrough; the current client protocol and its engine/signature-version cache identity do not work through this handshake as written. Define authenticated `zVERSION`/engine identity flow and whether connections are authenticated separately or reused.
- The challenge proves the client to the gateway but does not authenticate the gateway or encrypt traffic. Choose TLS/mTLS or an explicitly pinned/signed server identity plus an encrypted transport before sending files or license data.
- The example `--listen 0.0.0.0:3311` can expose the gateway beyond the trusted network. Decide the supported bind addresses and enforce VPN/firewall boundaries; clamd itself must remain private and must not be Internet-facing.
- `MASTER_PUBLIC_KEY` is described as embedded and is also present in `clients.toml`. Choose one authoritative source and define safe key rotation. Define client private-key protection (Windows DPAPI/ACL), backup, revocation, and rotation.
- Define whether registry fallback without a signed token is allowed, how `max(token_expiry, config_expiry)` interacts with revocation and plan overrides, and how the client displays status while offline. Client clock/UI data must not authorize scans.
- Fix the reference parser before reuse: unknown `PLAN_CODE` currently maps to `Lifetime`; `days * 86_400` can overflow. Reject unknown plans and use checked, policy-validated time arithmetic. Also specify canonical token encoding, malformed-packet responses, and handshake limits/rate limits.

Implementation stays gated on these protocol and deployment decisions. Preserve the current scanner/cache behavior and v1.0.0 scope until the gateway design is approved.

## Current execution handoff — V/Q-RELEASE-CURRENT

Recommended next task, not yet executed. Product baseline:c7f5fd9; documentation may advance independently. Target audience:internal company pilot. Recheck branch/status before work; freeze an exact code SHA before building.

1. Vic records source SHA, clean/patch state, toolchain and `package-lock.json`/`Cargo.lock` identity; runs existing Rust25, fmt/clippy, frontend and native regressions as appropriate. Investigate the recorded transient retained-results test if it recurs; do not erase evidence by retrying silently.
2. Build with `npm.cmd ci` then `npm.cmd run build:windows` from the implementation checkout. Record the x64 PE architecture, NSIS version, source SHA and artifact SHA256 in a dedicated handoff. No native-ui-tests feature or test root/target overrides in release. Runtime app version is currently0.1.0; a1.0 release/version change is a separate decision.
3. Qeen uses an independent checkout at the handoff SHA and the identified package with isolated profile/fixtures. Test actual File and Folder dialogs (including Cancel/Unicode), initial Quick scan scope, Manual Scan/menu return, Stop→Stopping→rescan, theme/icon persistence, bounded result window and non-CLEAN outcomes. Debug-hook fixture cases are recorded separately from release picker/actual-system cases.
4. Use a lab engine for benign/authorized positive/offline cases. Protected Windows paths may legitimately yield ERROR/incomplete; never elevate, exclude them silently or report full-system CLEAN to obtain a green screenshot. AV-intercepted EICAR remains explicitly BLOCKED; use existing harmless lab-signature procedure only with owned lab cleanup.
5. Test current-user sandbox install/shortcut/repair/uninstall/data preservation for this new artifact; no uninstall/overwrite of the user's running installation. Record logs/screenshots in dedicated artifacts, not public sensitive data. Signing, different-version upgrade and multi-user deployment stay NOT RUN unless actually tested.
6. Handoff product+artifact SHA to Qeen; update status/daily and existing draft PR with evidence/remaining gates. Do not merge main or publish a release without the owner's decision.

Acceptance:exact-artifact native path and installer flows observed, no release hooks, expected errors retained/no false All Good, private lab transport and original app data preserved. Report PASS/FAIL/BLOCKED by case and snapshot, not a blanket production-ready statement.

## Technical backlog from Notebook

Ordering/status live in [roadmap](roadmap.md); source/corrections live in [Notebook review](notebook-review-2026-11-07.md). These are task definitions, not completed implementations or approved deployments.

| Task | Vic deliverable before implementation | Qeen acceptance evidence |
| --- | --- | --- |
| LAB-RESILIENCE | Controlled test-owned tunnel/daemon failure harness; stage VERSION, stream and response boundaries; no changes to company VPN/AV | Real disconnect/reset, cancel/reconnect/rescan, identity change and stale event exclusion; each incomplete state never CLEAN |
| ENGINE-OPS | Read-only inventory first, then proposed update/reload/backup/runbook appropriate to existing lab | Signature freshness/reload failure/recovery and listener boundary observed; no assumed feed installation |
| PERF | Reproducible fixture list and cold/warm metrics collection including all owned WebView processes | Hardware/network/engine/signature identity and raw measurements; repeatability, cancel latency and bounded details; no borrowed Notebook percentage claims |
| FEED-LAB | One source entitlement/provenance record, staged ClamAV-compatible artifact and known-good rollback | Actual deployed-engine load, benign false positives, controlled positive fixture, reload failure/rollback; generic yarac alone insufficient |
| IOC-SYNC | Typed IOC schema, identity/canonicalization/expiry and per-source evidence; signed manifest+monotonic version/key rotation, decompression bounds, snapshot/delta gap recovery and transactional staging | Tampered/replayed/expired/incomplete delta, rollback, revocation/tombstone, wrong-hash-length, duplicates/conflicts and offline behavior. No partial activation or allow-policy→CLEAN conversion |
| GUARD/NETWORK/RESPONSE | Separate opt-in threat model and least-privilege/reversible design; lifecycle, path/network attribution, false-positive policy and recovery | Disposable Windows lab; event storm/overflow/rename/locked files, IPv4/IPv6/UDP as scoped, management/VPN reachability, rule ownership/expiry and rollback; no unowned firewall or quarantine changes |
| HARDENING/DATA | Inventory current mitigations/data sensitivity; proposal for DLL search/process ACL and optional SQLCipher+DPAPI identity/migration/backup, with dependencies justified | Artifact mitigation evidence, normal stop/update/uninstall/recovery, migration interruption/wrong identity/corruption/rollback; no claim of immunity against admin/kernel compromise |
| AI | Optional provider/BYOK contract, Rust-side secret boundary, selected/redacted data preview, consent/retention/cost bounds | Key absent from frontend/logs, no unapproved transmission, injected script/prompt cannot execute tools or alter verdict/firewall/quarantine, rate/cost/provider failure handled |

### Future IOC data contract (design criteria, not schema migration)

- Distinguish SHA-256 from URL/domain/IP; validate exact32-byte binary hashes and canonical forms. Keep source ID, source record/version, timestamps/expiry, license/provenance, confidence and withdrawal separately; one last-write-wins `source_feed` row loses required evidence.
- Dedup by typed indicator and source observation; define comparable severity vocabulary before computing a maximum. Keep conflicts visible. A signed administrator allow policy, if ever approved, is a scoped/expiring/audited disposition and never overwrites engine INFECTED or manufactures CLEAN. Existing six typed scanner outcomes remain; a new policy/status model needs explicit approval.
- No “safe extension” shortcut or metadata-only clean reuse. Existing cache depends on current content, stable file identity, live engine/signature identity, policy version and TTL. A revoked feed/version must invalidate affected trust. Fail closed on missing identity rather than claim offline CLEAN.
- Fetch/validate in staging; verify bounded size/count and authenticated signed manifest before transactionally activating one complete version. Test interrupted updates, expired data, rollback/recovery and revoked keys. SQLite encryption is orthogonal to authenticity and correctness; zeroize is not proof that frontend/WebView copies never exist.
- Keep API secrets and source downloads server-side when possible; do not add a client settings dashboard until a selected integration requires it. Public enrichment and AI data disclosure require a separate approved contract; no automatic upload of files, hashes, URLs, paths or company telemetry.
