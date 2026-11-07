# Zonendpoint

Zonendpoint is a Windows desktop client for **manual ClamAV scanning**. This
MVP is a supplementary scanner; it does not provide real-time protection,
quarantine, or replace Microsoft Defender/Sophos.

## What it does

- Starts with Quick scan for13 native Windows core files and the System32/drivers subtree. This is a limited file scan, not a full-system, memory or boot-integrity verdict. Selecting File/Folder changes the action to Scan; the target menu can return to Quick scan.
- Opens the native Windows picker for one file or folder.
- Streams files to a ClamAV daemon with bounded `zINSTREAM` frames.
- Shows per-file `CLEAN`, `INFECTED`, `UNKNOWN`, `ERROR`, `SKIPPED`, and
  `CANCELLED` outcomes, progress, and cancellation.
- Uses a per-user SQLite cache at `%LOCALAPPDATA%\Zonendpoint\cache.db`.
  Every cache lookup hashes file contents and checks current ClamAV
  `VERSION` plus the policy version; entries expire after 12 hours.
- Limits a file to 100,000,000 bytes, UI scan jobs to one (scanner semaphore capacity two), the folder queue to
  32 paths, and retained result details to 1,000. Oversized and incomplete
  work is reported and never becomes `CLEAN`.

Clamd's TCP protocol has no authentication or encryption. Keep the daemon on
loopback or a private/VPN address protected by network access controls. The
client rejects public addresses and hostnames. The default is
`127.0.0.1:3310`; set `ZONENDPOINT_CLAMD` to a private IP and port for an
isolated lab or trusted VPN. Files are sent only to that configured daemon.

## Development

```powershell
npm.cmd ci
cargo check --manifest-path src-tauri\Cargo.toml
cargo test --manifest-path src-tauri\Cargo.toml
npm.cmd run build
$env:ZONENDPOINT_CLAMD = "127.0.0.1:3310"
npm.cmd run tauri dev
```

The native picker returns a short-lived opaque target token to the UI;
the Rust command resolves it to the selected path. For a real-engine smoke
test, run an isolated ClamAV lab bound to loopback, then select only lab
fixtures. Do not point this client at a production endpoint as part of
development. If no lab daemon is available, real-engine integration is NOT
RUN; protocol regression tests use a local mock daemon.

Build the supported Windows x64 NSIS package with `npm.cmd run build:windows`. No32-bit/ARM Windows app package is supported; scanned file contents are not architecture-filtered. Native WebView/IPC launch, package identity, resource measurements and ClamAV integration require separate verification.

Start development/QA sessions at [project status](docs/project-status.md), then read [scope](docs/v1.0.0-scope.md). See [roadmap](docs/roadmap.md) for current priorities and [implementation plan](docs/implementation-plan.md) for task acceptance. Current product code and older installer evidence must not be conflated.
