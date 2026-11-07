# Zonendpoint architecture review

Historical baseline, preserved from2026-10-31. The failures and approval-pending language below describe that baseline, not current product status. Read [project-status.md](project-status.md), [roadmap.md](roadmap.md) and the current architecture addendum before interpreting them. For example, current CSP is configured and the scanner/UI/cache have since changed; this old review is not a new audit.

Date: 2026-10-31 (Asia/Saigon)
Branch: `main`, based on `2db42a8` (`feat: use SQLite cache for scan results`)

This review records the baseline before the first MVP implementation. It is not
a security certification or a claim that the client is ready for deployment.

## Existing project and host skills

- Repository is `E:\Zonendpoint`, remote `origin` is the requested GitHub URL.
- Existing local changes at review start: modified `package-lock.json` and
  untracked `docs/`. Both have been preserved; the untracked directory held
  the two required source documents. There is no `AGENTS.md` in the repo tree.
- Host surfaced Ponytail skills (`ponytail:ponytail` and
  `ponytail:ponytail-review`) and Superpowers skills (brainstorming, planning,
  TDD, debugging, and verification). This establishes skill availability in
  this thread, not that plugin lifecycle hooks are trusted or active.
- No plugin was installed or configured during this review.

## Baseline build/check results

Commands run on Windows PowerShell, Node/npm and Rust toolchain present:

| Command | Result |
| --- | --- |
| `npm run build` | PASS; TypeScript and Vite production bundle completed. |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | FAIL; Rust source is not rustfmt formatted. No source was changed by this check. |
| `cargo check --manifest-path src-tauri/Cargo.toml` | FAIL; `src-tauri/src/lib.rs:24` passes `String` where `scanner::scan_file` requires `&str`. |
| `cargo test --manifest-path src-tauri/Cargo.toml` | NOT RUN to completion; the initial combined baseline command exceeded its wait window while resolving/building crates. Re-run after design approval. |
| Native Tauri launch / ClamAV lab integration | NOT RUN. |

Cargo resolution created `src-tauri/Cargo.lock` during baseline. It is tracked
as a generated baseline artifact for now; no existing lockfile was overwritten.

## Findings

### Blocking the requested MVP

1. `src/App.tsx` is the Vite/Tauri greeting template and invokes `greet`, which
   is not registered in `src-tauri/src/lib.rs`; there is no file/folder picker,
   scan progress, cancellation, or typed result display.
2. `scanner.rs` sends `zINSTREAM\0` and zero terminator, but has no bounded
   stream size, timeout, cancellation, or semaphore. It uses `read_to_end`,
   allowing an unbounded response and hanging on a daemon that does not close.
3. The response parser uses substring checks (`FOUND`, then `OK`); malformed,
   empty, incomplete, and daemon `ERROR` replies can be misreported. The
   returned verdict is an untyped string.
4. Scanner reads cache by `(path,size,mtime)` before reading/hashing bytes, so
   same-metadata content replacement can reuse a stale verdict. Scanner and
   `cache_stats` also construct different database locations (`APPDATA` vs
   Tauri `app_data_dir`). Cache stores arbitrary result text and has no schema
   version or migration guard.
5. `deep_scan` accumulates all results in an unbounded `Vec`, aborts on folder
   enumeration errors, and silently drops per-file scan errors. It has no
   bounded producer/consumer queue, progress, or cancellation.
6. Guard is opt-in by default, but only handles Create events. Watcher errors
   end its receive loop; its unbounded channel and per-file thread/runtime
   creation have no backpressure. Modify/Rename and reconciliation are absent.

### Policy / scope notes

- No automatic quarantine is implemented; keep it out of the first slice.
- Extension-based skip lists, public threat-intel services, feeds, and hash
  sync are not acceptable as MVP trust inputs. The user instruction overrides
  older optimistic statements in the architecture document.
- `tauri.conf.json` currently has `csp: null`; review this separately before
  production packaging. Tauri capability/picker integration also needs to be
  confined to user-selected paths.
- Existing `main.rs` has a CLI `--scan` branch that prints a result; align it
  with typed status semantics or remove only if callers are checked.

## Proposed safe MVP design (approval pending)

- Keep Tauri v2, React/TypeScript, Tokio, SQLite, and the current single
  ClamAV client path. Use an explicit typed per-file outcome with status,
  reason, and SHA-256 where available. Only strict `stream: ... OK` and
  `stream: ... FOUND` terminal replies map to CLEAN/INFECTED; daemon errors,
  limit violations, parse failures, and unavailable daemon remain explicit.
- Use an injectable `AsyncRead`/daemon transport boundary for protocol tests;
  implement correct NUL `zINSTREAM`, bounded chunks/total bytes, terminator,
  capped NUL-framed response parsing, separate connect/read/total timeout and
  cancellation, and one bounded semaphore. No persistent connection pool.
- For cache, always hash bytes while scanning and compare pre/post file
  identity metadata before accepting a result. Any metadata change invalidates
  the result. Cache only completed typed engine verdicts, and key reuse on the
  content SHA-256 plus daemon/policy identity and bounded TTL. Treat cache DB
  failure as a visible ERROR. Use one Tauri per-user app-data DB path and a
  transactional versioned migration; preserve the old table until migration
  commits.
- Traverse folders with an iterative bounded queue and a small fixed worker
  count, no symlink/junction following, per-path error outcomes, cancellation,
  and bounded retained details plus aggregate counts. Tauri events report
  progress; frontend owns start/cancel and clearly renders UNKNOWN/ERROR/
  SKIPPED/CANCELLED.
- Keep real-time Guard OFF; no auto-quarantine, extension allowlist, feed,
  known-hash sync, or production endpoint changes in this slice.

Only architecture already fixed by the user is treated as decided. Detailed
limits (file size, timeout, response bytes, worker count, retained UI details)
are implementation defaults to document and test; no choice changes intended
protection scope or data destinations. Implementation is gated on approval of
this design.
