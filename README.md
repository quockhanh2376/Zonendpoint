# Zonendpoint

Windows 11 endpoint security desktop client built with Tauri v2, React, TypeScript, Tailwind CSS, and Rust. The client is designed as a lightweight coordinator for a remote ClamAV daemon; signature databases remain on the server.

## Architecture

- **Desktop UI:** React + TypeScript in a Tauri v2 shell.
- **Rust core:** Tokio TCP client sends files using ClamAV `zINSTREAM` in 64 KiB chunks and computes SHA-256 while streaming.
- **Local cache:** SQLite with WAL mode and an indexed `scan_cache` table.
- **Real-time guard:** opt-in user-space `notify` watcher for Downloads, Desktop, and the system Temp directory.
- **Windows integration:** optional per-user context-menu install/uninstall scripts.

## Requirements

Install Node.js, Rust, and the Windows C++ Build Tools/WebView2 prerequisites for Tauri. A reachable ClamAV daemon is expected at `127.0.0.1:3310` by default; set `ZONENDPOINT_CLAMD` to override it. Use a trusted network/VPN because the native ClamAV TCP protocol is not encrypted by itself.

## Development

```powershell
npm install
$env:ZONENDPOINT_CLAMD = "127.0.0.1:3310"
npm run tauri dev
```

Commands scaffolded in Rust: `quick_scan`, `deep_scan`, `cache_stats`, and `set_guard`. Build a release package with `npm run tauri build`.

## Context menu

Run `scripts/install-context-menu.ps1` to add a per-user Explorer entry. Run `scripts/uninstall-context-menu.ps1` to remove it. The install script points to `src-tauri\target\release\zonendpoint.exe`.

## Memory target

The project uses a small Tauri shell and bounded 64 KiB scan buffers to support an under-120 MB client memory target. Measure release builds under representative workload before treating this as a verified limit.
