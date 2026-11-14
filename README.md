# Zonendpoint

Zonendpoint is a Windows desktop client for **manual ClamAV scanning**,
designed for **personal, non-commercial use on 1–2 machines**. This MVP is a
supplementary scanner; it does not provide real-time protection, quarantine,
or replace Microsoft Defender/Sophos. The application requires a **separate,
dedicated ClamAV server** (on loopback or reached over an encrypted SSH tunnel /
private VPN).

## What it does

- Starts with Quick scan for13 native Windows core files and the System32/drivers subtree. This is a limited file scan, not a full-system, memory or boot-integrity verdict. Selecting File/Folder changes the action to Scan; the target menu can return to Quick scan.
- Opens the native Windows picker for one file or folder.
- Streams files to a ClamAV daemon with bounded `zINSTREAM` frames.
- Shows per-file `CLEAN`, `INFECTED`, `UNKNOWN`, `ERROR`, `SKIPPED`, and
  `CANCELLED` outcomes, progress, and cancellation.
- Uses a per-user SQLite cache at `%LOCALAPPDATA%\Zonendpoint\cache.db`.
  Every cache lookup hashes file contents and checks current ClamAV
  `VERSION` plus the policy version; entries expire after 12 hours.
- Limits scanning to **25 MiB per file** (26,214,400 bytes). Files larger than
  25 MiB are marked `SKIPPED`, and the scan is considered incomplete and is
  never treated or reported as clean (`CLEAN`).
- Limits UI scan jobs to one (scanner semaphore capacity two), the folder queue to
  32 paths, and retained result details to 1,000. Oversized and incomplete
  work is reported and never becomes `CLEAN`.

Clamd's TCP protocol has no authentication or encryption. The client requires a
separate, dedicated ClamAV server instance kept on loopback (`127.0.0.1:3310`)
or accessed through a secure SSH tunnel / private VPN protected by network access
controls. The client strictly rejects public addresses and hostnames. Set
`ZONENDPOINT_CLAMD` to a private IP and port or tunnel endpoint when using a
custom configuration. Files are sent only to that configured daemon.

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
