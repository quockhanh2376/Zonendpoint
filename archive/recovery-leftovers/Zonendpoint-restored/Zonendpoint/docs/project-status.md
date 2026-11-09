# Project status

**Read this file first at the start of every Vic/Qeen session.** Updated 2026-11-01.

## Phase

v1.0.0 manual-scan MVP; native UI branding and scan-control retest is complete for its tested scope. Current product rules are in [`v1.0.0-scope.md`](v1.0.0-scope.md). Real ClamAV validation is the next acceptance gate before expanding Guard or Auto-Quarantine.

## Worktrees and snapshots

| Purpose | Location | Branch / snapshot |
|---|---|---|
| Source checkout and project docs | `E:\Zonendpoint` | `main`, HEAD `2db42a8124f19384d154588fac41d7bab791bb97`; pre-existing user changes include `package-lock.json`, icons, `src-tauri/Cargo.lock`, and untracked source docs/assets. Preserve them. |
| Vic implementation | `E:\Zonendpoint-vic` | `vic/mvp-first-scan`; code snapshot `e3ed872ddfd4a95d9bb28a3cc6da53a656e0b0ae`; handoff-doc commit `24876647ebf1f621b3c0a567e60efd40500d6154`. Recheck branch/HEAD before work. |
| Qeen QA | `E:\Zonendpoint-QA-e3ed872` | Detached at code SHA `e3ed872ddfd4a95d9bb28a3cc6da53a656e0b0ae`; Qeen handoff snapshot `24876647ebf1f621b3c0a567e60efd40500d6154`. |

## QA state

Qeen reports **PASS for the tested native UI/icon/theme persistence, pickers, and unified Start → Stop → Stopping… → Start lifecycle only** at code SHA `e3ed872ddfd4a95d9bb28a3cc6da53a656e0b0ae`. Scan lifecycle used a local loopback mock; it is not evidence of real ClamAV detection. Real ClamAV integration and Windows installer/shortcut are **NOT RUN**. See [Qeen report](../../Zonendpoint-QA-e3ed872/docs/qa/report-e3ed872.md), [Qeen handoff](../../Zonendpoint-QA-e3ed872/docs/handoffs/qeen-to-vic.md), and [Vic handoff](../../Zonendpoint-vic/docs/handoffs/vic-to-qeen.md). The earlier [`qa/report-2db42a8.md`](qa/report-2db42a8.md) is a historical FAIL for the prior baseline, not the current code snapshot.

## Blockers and next work

**Blocker:** no isolated real-ClamAV lab result is recorded. Real-engine behavior remains unverified; do not represent mock `stream: OK` as ClamAV detection.

**Next:** prepare/use an isolated ClamAV lab and run native integration on the exact current Vic snapshot. Use benign fixtures and EICAR only inside the lab. If an existing endpoint AV blocks the fixture first, report interference/BLOCKED rather than Zonendpoint detection. Record ClamAV and signature version, command/environment, and observed UI/core result. Do not alter production configuration or security policy and do not submit company files to public services.

**Acceptance criteria:**

- Native Tauri connects only to the intended private/loopback lab daemon; daemon response and engine/signature identity are recorded.
- Clean fixture is CLEAN only after a complete successful scan; infected lab fixture is INFECTED only when the lab daemon reports it; offline, timeout, malformed/incomplete response, cancellation, oversize/policy limit never become CLEAN.
- Start/Stop lifecycle remains single-task and reports confirmed cancellation; per-file errors and final counts remain visible in folder scans.
- Record exact code SHA, commands, expected/actual results, and any AV interference. If the lab is unavailable, mark the integration NOT RUN and retain this blocker.

After this gate, validate a Windows package/installer/shortcut when a release artifact exists; current status remains NOT RUN. Do not expand Guard/Auto-Quarantine ahead of the real-engine gate.

## Later phase

SecConnect/Ed25519 gateway and licensing: **Planned / Not implemented**, outside v1.0.0. Dependencies, architecture conflicts, and decisions are tracked in [`implementation-plan.md`](implementation-plan.md) and the [licensing specification](zonendpoint_ed25519_secure_gateway_licensing_spec.md). No gateway or licensing implementation is authorized by the current scope.

## Source documents

- [v1.0.0 scope](v1.0.0-scope.md)
- [Roadmap](roadmap.md)
- [System architecture specification](zonendpoint_system_architecture_specs.md)
- [Ed25519 secure gateway and licensing specification](zonendpoint_ed25519_secure_gateway_licensing_spec.md)
- [Architecture review](architecture-review.md)
- [Vic/Qeen workflow prompts](Zonendpoint_Vic_Qeen_Codex_Prompts.md)
