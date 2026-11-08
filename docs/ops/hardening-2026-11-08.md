# Zonendpoint Lab Server Hardening Report

- **Date:** 2026-11-08
- **Target Host:** Lab ClamAV Engine Host `udevlevel73` (`100.96.250.245`)
- **Author:** Victor (Vic) — Task V-13
- **Approval:** Approved by Zon (pursuant to Runbook V-9, items 1–5)
- **Status:** COMPLETED & VERIFIED
- **Evidence Path:** `E:\Zonendpoint-tools\artifacts\engine-ops\2026-11-08-harden\`

---

## 1. Executive Summary & Safety Invariants

All 4 operational hardening tasks were executed on the lab server `udevlevel73` via SSH using passwordless sudo (`sudo -n`). Every safety invariant was strictly preserved:

| Safety Rule | Status | Verification Evidence |
|---|---|---|
| **No edits to engine config files** | PASS | `/etc/clamav/clamd.conf` and `/etc/clamav/freshclam.conf` were untouched. All changes used systemd drop-ins or `/usr/local/bin/` scripts. |
| **No clamd restart** | PASS | clamd `MainPID=2245` remained continuously running before, during, and after `systemctl daemon-reload`. |
| **sshd configuration tested before reload** | PASS | `sudo /usr/sbin/sshd -t` returned exit code 0 before reload. Service reloaded via `systemctl reload ssh` (PID 3464 maintained, no restart). |
| **PasswordAuthentication untouched** | PASS | Confirmed `passwordauthentication yes` preserved via `sshd -T`. `50-cloud-init.conf` was unmodified. |
| **Continuous TCP loopback & tunnel** | PASS | `127.0.0.1:3310` (loopback) and `127.0.0.1:13310` (Windows SSH tunnel) returned `PONG` at every step. |
| **Rollback tested in dry-run only** | PASS | `clamav-rollback-known-good.sh --dry-run` passed signature validation and simulation without altering active database files. |
| **Local-only alerting** | PASS | Update warnings log strictly to local systemd journal via `logger` with identifier `zonendpoint-clamav-monitor`. No external telemetry or network calls. |

---

## 2. Implemented Changes & Files Created

### 2.1 clamd & freshclam Systemd Drop-ins (Auto-Restart)

- **Purpose:** Automatically recover clamd and freshclam daemons if terminated unexpectedly.
- **Files Created:**
  1. `/etc/systemd/system/clamav-daemon.service.d/restart.conf`:
     ```ini
     [Service]
     Restart=on-failure
     RestartSec=5s
     ```
  2. `/etc/systemd/system/clamav-freshclam.service.d/restart.conf`:
     ```ini
     [Service]
     Restart=on-failure
     RestartSec=30s
     ```
- **Application Method:** Executed `sudo systemctl daemon-reload` only. No restart command was issued.
- **Verification:**
  - `systemctl show clamav-daemon.service -p Restart,RestartUSec,MainPID,ActiveState`
    -> `Restart=on-failure`, `RestartUSec=5s`, `MainPID=2245`, `ActiveState=active`.
  - `systemctl show clamav-freshclam.service -p Restart,RestartUSec,MainPID,ActiveState`
    -> `Restart=on-failure`, `RestartUSec=30s`, `MainPID=2388`, `ActiveState=active`.

### 2.2 Known-Good Database Backup & Rollback System

- **Purpose:** Maintain snapshot of verified ClamAV signature database to enable fast rollback if freshclam downloads a corrupted signature package.
- **Files Created:**
  1. `/usr/local/bin/clamav-backup-known-good.sh` (mode `0755`, owned by `root:root`):
     - Locates `main.cvd`, `daily.cld`, `bytecode.cvd` in `/var/lib/clamav`.
     - Verifies digital signatures via `sigtool --info <file>`. Aborts if verification fails.
     - Copies verified signatures, `.sign` files, and `freshclam.dat` to `/var/backups/clamav-known-good/<YYYY-MM-DD>/`.
     - Sets ownership to `clamav:clamav` with directory mode `0750` and file mode `0644`.
     - Automatically prunes older backups to retain exactly the 3 newest snapshots.
  2. `/usr/local/bin/clamav-rollback-known-good.sh` (mode `0755`, owned by `root:root`):
     - Supports `--dry-run` (`-n`) flag to simulate rollback without stopping services or touching files.
     - Selects latest or specified backup directory and verifies signatures with `sigtool --info`.
     - When run without `--dry-run`, stops services, restores verified database, fixes permissions, restarts clamd, tests PING, and restarts freshclam.
  3. `/etc/systemd/system/clamav-backup-known-good.service` (mode `0644`, owned by `root:root`):
     - Oneshot service executing `/usr/local/bin/clamav-backup-known-good.sh`.
  4. `/etc/systemd/system/clamav-backup-known-good.timer` (mode `0644`, owned by `root:root`):
     - Runs daily at `03:00:00 UTC` (`Persistent=true`, `RandomizedDelaySec=1800`).
- **Verification:**
  - Manual run of `/usr/local/bin/clamav-backup-known-good.sh` succeeded.
  - Verified snapshot created under `/var/backups/clamav-known-good/2026-11-08/`.
  - `sigtool --info` verified `main.cvd` (v63), `daily.cld` (v28176), `bytecode.cvd` (v339).
  - Retention test verified: temporary older directories pruned, exactly 3 copies retained.
  - Rollback dry-run verified: `clamav-rollback-known-good.sh --dry-run` successfully verified backup integrity and printed execution simulation without stopping clamd.

### 2.3 Update Health Alerting System

- **Purpose:** Detect if daily signatures are older than 48 hours or if freshclam encounters errors, logging a warning to local journald.
- **Files Created:**
  1. `/usr/local/bin/clamav-check-update-health.sh` (mode `0755`, owned by `root:root`):
     - Computes age of `daily.cld` / `daily.cvd`. If age > 172800 seconds (48 hours), logs `WARNING: daily database is older than 48 hours`.
     - Checks if `clamav-freshclam` is active and not failed.
     - Inspects `/var/log/clamav/freshclam.log` for recent `ERROR:` entries.
     - Logs via `logger -p user.warning -t zonendpoint-clamav-monitor "WARNING: ..."`.
     - Emits zero network traffic; strictly local logging.
  2. `/etc/systemd/system/clamav-check-update-health.service` (mode `0644`, owned by `root:root`).
  3. `/etc/systemd/system/clamav-check-update-health.timer` (mode `0644`, owned by `root:root`):
     - Runs every 30 minutes (`OnCalendar=*:0/30`, `Persistent=true`).
- **Verification:**
  - Manual execution confirmed health check pass: `OK: ClamAV update health check passed (daily age: 23h, freshclam active)`.
  - Simulated warning test confirmed journal logging: `Nov 08 07:34:50 udevlevel73 zonendpoint-clamav-monitor[14807]: TEST-SIMULATION WARNING: ...`.
  - Timer active: `systemctl list-timers clamav-check-update-health.timer` confirms active trigger.

### 2.4 SSH Keepalive Drop-in (sshd)

- **Purpose:** Prevent SSH port-forwarding tunnels (`127.0.0.1:13310`) from silently hanging over Tailscale NAT or firewalls during inactive scan intervals.
- **File Created:**
  `/etc/ssh/sshd_config.d/60-zonendpoint-keepalive.conf` (mode `0644`, owned by `root:root`):
  ```sshd_config
  # Zonendpoint Lab SSH KeepAlive Drop-in (V-13)
  ClientAliveInterval 30
  ClientAliveCountMax 3
  ```
- **Pre-Reload Syntax Verification:**
  - `sudo /usr/sbin/sshd -t` returned exit code 0 (`SSHD_SYNTAX_OK`).
- **Reload:**
  - `sudo systemctl reload ssh` (Process `ExecReload=/bin/kill -HUP $MAINPID`).
  - Active Main PID 3464 retained without termination.
- **Verification:**
  - `sudo /usr/sbin/sshd -T | grep -Ei 'clientalive|passwordauthentication'` confirmed:
    * `clientaliveinterval 30`
    * `clientalivecountmax 3`
    * `passwordauthentication yes` (preserved unmodified).
  - Open SSH sessions and Windows SSH tunnel remained uninterrupted.

---

## 3. Rollback Procedures (How to Undo All Changes)

If any operational issue requires reverting this hardening session, run the following commands over SSH:

### 3.1 Reverting clamd & freshclam Drop-ins
```bash
sudo rm -f /etc/systemd/system/clamav-daemon.service.d/restart.conf
sudo rm -f /etc/systemd/system/clamav-freshclam.service.d/restart.conf
sudo systemctl daemon-reload
```

### 3.2 Reverting Backup Job & Rollback Script
```bash
sudo systemctl disable --now clamav-backup-known-good.timer
sudo rm -f /etc/systemd/system/clamav-backup-known-good.timer
sudo rm -f /etc/systemd/system/clamav-backup-known-good.service
sudo rm -f /usr/local/bin/clamav-backup-known-good.sh
sudo rm -f /usr/local/bin/clamav-rollback-known-good.sh
sudo rm -rf /var/backups/clamav-known-good
sudo systemctl daemon-reload
```

### 3.3 Reverting Update Health Monitor
```bash
sudo systemctl disable --now clamav-check-update-health.timer
sudo rm -f /etc/systemd/system/clamav-check-update-health.timer
sudo rm -f /etc/systemd/system/clamav-check-update-health.service
sudo rm -f /usr/local/bin/clamav-check-update-health.sh
sudo systemctl daemon-reload
```

### 3.4 Reverting SSH Keepalive
```bash
sudo rm -f /etc/ssh/sshd_config.d/60-zonendpoint-keepalive.conf
sudo /usr/sbin/sshd -t
sudo systemctl reload ssh
```

---

## 4. Evidence Artifacts

All verification logs, configurations, and connectivity traces are archived on the host workstation under `E:\Zonendpoint-tools\artifacts\engine-ops\2026-11-08-harden\`:

- `clamd-freshclam-status.txt`: Systemd status, drop-ins, and verified clamd PID 2245.
- `backup-verification.txt`: Backup directory listing, sigtool verification of backup signatures, retention pruning test, and dry-run rollback simulation output.
- `update-health-status.txt`: Update monitor timer status, manual execution output, and systemd journal warning logs.
- `sshd-config-status.txt`: Syntax check output (`sshd -t`), effective configuration (`sshd -T`), and reload verification.
- `connectivity-evidence.txt`: Pre- and post-hardening PING and VERSION responses for both local loopback (`127.0.0.1:3310`) and Windows tunnel forwarder (`127.0.0.1:13310`).
- `scripts/`: Local copy of all deployed shell scripts, systemd unit files, and configuration drop-ins.
