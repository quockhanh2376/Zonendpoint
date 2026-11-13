# Zonendpoint roadmap

Updated2026-11-07. Initial audience: **internal company pilot**, confirmed by Zon. Sequence is gated by evidence, not calendar promises. Future proposals are not authorization to deploy, subscribe, upload data or change endpoint/network protection.

Start each session at [project status](project-status.md). [Scope](v1.0.0-scope.md) defines the approved product; [implementation plan](implementation-plan.md) owns technical tasks. [Notebook review](notebook-review-2026-11-07.md) records source proposals, corrections and vendor references.

## P0 — Close the current scan candidate

Core code c7f5fd9: Quick scan13 native Windows core files+drivers and manual File/Folder Scan, Windowsx64. Independent Rust25/mock15/real-fixtures6 passed. Actual-system scan had588 CLEAN+1 protected-directory ERROR and correctly stayed incomplete. See [Quick scan QA](qa/quick-scan-c7f5fd9.md). Earlier01e7449 unsigned NSIS passed packaging tests but contains older product code.

Next task **V/Q-RELEASE-CURRENT**: choose a handoff product SHA; rebuild x64 NSIS, record hash/toolchain and independently test that artifact's native Quick scan/manual File/Folder picker, Stop/rescan, Light/Dark persistence, offline-not-CLEAN and install/shortcut/repair/uninstall/data preservation. Verify test overrides absent from release. Leave protected paths as explicit errors; no automatic elevation or permission changes. PASS must identify source SHA and artifact SHA256. Old picker/installer evidence cannot stand in for this snapshot. This is the recommended next execution task.

## P1 — Make the internal pilot supportable

**V/Q-LAB-RESILIENCE**: document/install-free run configuration and engine availability; exercise real tunnel disconnect/reset during VERSION/INSTREAM/response, cancellation at multiple stages, reconnect/rescan and signature-identity changes. Log observed terminal outcomes and stale-event exclusion; never upgrade incomplete/offline to CLEAN. Test-owned tunnels/processes only.

**V/Q-ENGINE-OPS**: audit current lab freshclam status, signature age, reload, permissions and recovery. Establish a runbook with engine/signature inventory, update failure alert, rollback/restore and lab-only fixtures. No extra feeds or multi-client tuning by assumption. Clamd remains loopback behind approved private transport; local127.0.0.1:13310 is a lab tunnel, not a production service promise.

**V/Q-PERF**: cold/warm Quick scan and representative fixture sets; record wall time, bytes, cache hits, process-tree memory (including WebView2), CPU, cancel latency and lab/network identity. Measure before adding workers/pools. Notebook RAM/CPU/latency/coverage figures are hypotheses, not release claims.

Exit: current package accepted, reproducible lab recovery/resilience evidence, measured baseline and written pilot support boundaries. Main merge remains an owner decision; signing/distribution policy is separate from a local unsigned smoke.

## P2 — Optional server-side detection enrichment

Design-only backlog **V/Q-FEED-LAB**: evaluate one approved source/rule set at a time on a disposable lab engine before client feed sync. Inventory all nine Notebook source groups in the [review](notebook-review-2026-11-07.md). Verify company-use license, credentials, provenance, update limits and native-engine compatibility. Stage, validate with the deployed ClamAV, measure false positives/performance and test rollback before activation. No malware download requirement and no automatic public file/hash/URL submission.

Client hash/delta sync is a separate **V/Q-IOC-SYNC** design, dependent on authenticated transport, signed/versioned data, provenance/revocation, decompression limits, transactional migration/rollback and conflict policy. Do not adopt the Notebook's whitelist-first CLEAN shortcut. Public feed or allow-policy information must not overwrite an observed detection. Cache semantics stay content+engine+signature+policy bound. Offline state remains explicit.

Exit: approved source and transport policies plus reproducible lab evidence; feature scope approved before product implementation. Internal company use does not imply personal/noncommercial API eligibility.

## P3 — Secure gateway/licensing (separate proposal)

**V/Q-SECCONNECT** remains Planned/Not implemented, outside v1.0.0. Follow the existing [gateway/licensing spec](zonendpoint_ed25519_secure_gateway_licensing_spec.md) and [unresolved protocol decisions](implementation-plan.md#later-phase-backlog--secure-gateway-and-licensing). Authenticate server and encrypt transport; resolve VERSION/cache identity, key rotation/revocation and token parsing before coding. Licensing is not a prerequisite for this internal scan pilot. No public listener or credential provisioning is authorized by this roadmap.

## P4 — Guard and response controls

Guard is scheduled for **v1.1** (after P0 / v1.0 release is approved and locked). Implementation tasks V-47..V-50 proceed on dedicated feature branches and must not merge into `main` before v1.0 is finalized.

Independent proposals after scan/pilot evidence: **V/Q-GUARD** (v1.1) for bounded Create/Modify/Rename handling, partial-file backoff, event-storm recovery, opt-in/default OFF and coexistence; **V/Q-NETWORK** for IP/domain attribution, observe-only first, reversible firewall policy and scoped audited overrides; **V/Q-RESPONSE** for quarantine/restore with recovery guarantees. Battle Mode is a bounded scheduling proposal dependent on Guard+trusted IOC context, not evidence of prevention. Existing extension filtering is not a trust signal.

No arbitrary blocking on this development host, global firewall changes, persistent allow rules, socket termination, automatic quarantine or AV-policy changes. IPv4/IPv6, UDP, VPN/management connectivity, false positives, expiry and rollback need a lab design before enforcement.

## P5 — Data protection and optional AI assistance

**V/Q-HARDENING**: evaluate DLL search policy and ASLR/DEP/CFG artifact evidence; threat-model process ACL changes without claiming user-space tamper immunity. **V/Q-DATA**: SQLCipher/DPAPI only after data inventory, user identity/recovery policy, plaintext-to-encrypted migration and backup/rollback tests. Keys must not enter frontend storage/logs; database encryption does not protect a compromised running user session.

**V/Q-AI**: optional BYOK explanation/triage, separated from scanner verdicts and response permissions. Explicit provider/data/retention choice, Rust-side protected key storage, redaction, cost/rate bounds and untrusted-input handling. AI must not independently declare CLEAN, execute analyzed scripts, change firewall policy or quarantine. No external AI call/upload is authorized by documenting this backlog.

## Maintenance

CheKi creates/updates Vic and Qeen tasks from Zon requests; Zon need not repeat prompts. Vic hands off an exact source/patch/artifact; Qeen runs an independent checkout and reports PASS/FAIL/BLOCKED/NOT RUN by mode. Update status/daily only on actual changes. No future feature is silently added to v1.0.0, and no dates or percentage detection claims are promised without evidence.
