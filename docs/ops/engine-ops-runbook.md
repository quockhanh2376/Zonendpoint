# Zonendpoint Engine Operations Runbook (DRAFT)

- **Target System:** Lab ClamAV Engine Host `udevlevel73` (`100.96.250.245`)
- **Author:** Victor (Vic) — Task V-9 (ENGINE-OPS)
- **Status:** DRAFT — Operational guidelines and proposed changes.
- **Safety Rule:** Every step or command that modifies files, services, or configurations on the server is explicitly labeled **`[REQUIRES ZON APPROVAL]`**.

---

## 1. ClamAV Service Health Inspection (Read-Only)

Run these commands over SSH (`admzon@100.96.250.245`) without elevated privileges:

### 1.1 Systemd Service & Socket States
```bash
# Check daemon, updater, and socket states
systemctl status clamav-daemon clamav-freshclam clamav-daemon.socket --no-pager

# Verify active states
systemctl is-active clamav-daemon clamav-freshclam clamav-daemon.socket
```
Expected output: all three units return `active`.

### 1.2 Loopback Listener & Socket Verification
```bash
# Check TCP port 3310 binding (must be 127.0.0.1:3310 only)
ss -ltnp 'sport = :3310'

# Check local control socket
ls -la /run/clamav/clamd.ctl
```
Expected output: `LISTEN` on `127.0.0.1:3310`, socket mode `srw-rw-rw-` owned by `clamav:clamav`.

### 1.3 Clamd Protocol Verification (Local PING / VERSION)
```bash
# Send PING over loopback
nc -z -w 3 127.0.0.1 3310 && echo "PING" | nc -w 3 127.0.0.1 3310

# Request VERSION string
echo "VERSION" | nc -w 3 127.0.0.1 3310
```
Expected output: `PONG` and `ClamAV 1.5.4/...`.

### 1.4 Windows Client Tunnel Check (Windows Host)
From the Windows development/pilot workstation:
```powershell
# Verify Windows forwarder on 127.0.0.1:13310
Test-NetConnection -ComputerName 127.0.0.1 -Port 13310
```
Expected output: `TcpTestSucceeded : True`.

### 1.5 Log & SelfCheck Inspection
```bash
# Check last 30 lines of freshclam update log
sudo tail -n 30 /var/log/clamav/freshclam.log

# Check clamd selfcheck status
sudo grep -Ei "(SelfCheck|Database correctly reloaded)" /var/log/clamav/clamav.log | tail -n 20
```

---

## 2. Update Health & Failure Alerting

### 2.1 Current State
- `freshclam` runs as a daemon checking 24 times daily (`Checks 24`).
- Upon downloading new signatures, `freshclam` verifies the database (`TestDatabases yes`) and triggers `NotifyClamd /etc/clamav/clamd.conf`.
- Clamd reloads the new signatures in-memory while continuing to serve active scans.
- **Deficiency:** If updates fail consecutively due to CDN issues, network block, or DNS errors, `freshclam` logs to `/var/log/clamav/freshclam.log` but fires no notification.

### 2.2 Proposed Alerting Mechanism `[REQUIRES ZON APPROVAL]`
Implement an alert script invoked via `OnErrorExecute` in `/etc/clamav/freshclam.conf` or a systemd failure notification:

1. **Option A: Freshclam `OnErrorExecute` Hook**
   Add to `/etc/clamav/freshclam.conf`:
   ```ini
   OnErrorExecute /usr/local/bin/clamav-update-alert.sh "%v"
   ```
   Where `clamav-update-alert.sh` writes an alert to `/var/log/zonendpoint-ops.log` or sends a webhook/syslog alert when an update error occurs.

2. **Option B: Daily Signature Freshness Cron Monitor**
   A scheduled script checking the age of `/var/lib/clamav/daily.cld`:
   ```bash
   #!/bin/bash
   # Alert if daily.cld is older than 48 hours
   AGE=$(( $(date +%s) - $(stat -c %Y /var/lib/clamav/daily.cld) ))
   if [ "$AGE" -gt 172800 ]; then
       logger -t zonendpoint-ops "WARNING: ClamAV daily database is older than 48 hours!"
   fi
   ```

---

## 3. Known-Good Database Backup & Rollback Procedure

Because no known-good database backup currently exists on the server, a corrupt or failed database can prevent clamd from starting.

### 3.1 Initial Known-Good Backup Creation `[REQUIRES ZON APPROVAL]`
Execute the following commands on the lab server to snapshot the current verified database:
```bash
# 1. Create a dedicated backup directory with clamav ownership
sudo mkdir -p /var/backups/clamav-known-good
sudo chown -R clamav:clamav /var/backups/clamav-known-good
sudo chmod 0750 /var/backups/clamav-known-good

# 2. Copy current verified CVD/CLD files
sudo cp -p /var/lib/clamav/*.c[vl]d /var/backups/clamav-known-good/
sudo cp -p /var/lib/clamav/*.sign /var/backups/clamav-known-good/ 2>/dev/null || true
sudo cp -p /var/lib/clamav/freshclam.dat /var/backups/clamav-known-good/

# 3. Verify digital signatures of backup files
sigtool --info /var/backups/clamav-known-good/main.cvd
sigtool --info /var/backups/clamav-known-good/daily.cld
sigtool --info /var/backups/clamav-known-good/bytecode.cvd
```

### 3.2 Automated Daily Backup Job `[REQUIRES ZON APPROVAL]`
Install a daily systemd service/timer or cron script (`/etc/cron.daily/zonendpoint-clamav-backup`):
```bash
#!/bin/bash
set -euo pipefail
BACKUP_DIR="/var/backups/clamav-known-good"
DB_DIR="/var/lib/clamav"

# Only backup if daily.cld passes sigtool verification
if sigtool --info "$DB_DIR/daily.cld" >/dev/null 2>&1; then
    mkdir -p "$BACKUP_DIR"
    cp -p "$DB_DIR"/*.c[vl]d "$BACKUP_DIR/"
    cp -p "$DB_DIR"/*.sign "$BACKUP_DIR/" 2>/dev/null || true
    cp -p "$DB_DIR"/freshclam.dat "$BACKUP_DIR/"
    logger -t zonendpoint-ops "ClamAV known-good database backup updated successfully."
else
    logger -p user.err -t zonendpoint-ops "ERROR: daily.cld failed verification; skipping backup."
fi
```

### 3.3 Rollback Procedure (In Case of Corrupt Database) `[REQUIRES ZON APPROVAL]`
If clamd fails to start or freshclam installs a damaged database:

1. **Stop services:**
   ```bash
   sudo systemctl stop clamav-freshclam
   sudo systemctl stop clamav-daemon
   ```
2. **Restore known-good database:**
   ```bash
   sudo cp -p /var/backups/clamav-known-good/* /var/lib/clamav/
   sudo chown clamav:clamav /var/lib/clamav/*
   ```
3. **Verify restored files:**
   ```bash
   sigtool --info /var/lib/clamav/daily.cld
   ```
4. **Restart daemon:**
   ```bash
   sudo systemctl start clamav-daemon
   systemctl status clamav-daemon --no-pager
   ```
5. **Verify clamd responsiveness:**
   ```bash
   echo "PING" | nc -w 3 127.0.0.1 3310
   ```
6. **Restart updater (after resolving upstream issue):**
   ```bash
   sudo systemctl start clamav-freshclam
   ```

---

## 4. Manual Signature Reload Procedure

If new signatures or test rules are deployed and need immediate reload without restarting the daemon:

### Option A: Via clamdscan (Recommended) `[REQUIRES ZON APPROVAL]`
```bash
sudo -u clamav clamdscan --reload
```

### Option B: Via SIGUSR2 Signal `[REQUIRES ZON APPROVAL]`
```bash
sudo systemctl reload clamav-daemon
# Or directly:
sudo kill -USR2 $(pidof clamd)
```

### Verification
Check `clamav.log` to confirm the reload was completed:
```bash
sudo tail -n 20 /var/log/clamav/clamav.log | grep -Ei "(Reading databases|Database correctly reloaded|Activating)"
```

---

## 5. Failure Scenario: clamd Down & Client Behavior

### 5.1 Symptoms
- `clamav-daemon` is stopped, crashed, or port 3310 is unreachable.
- Windows SSH tunnel `127.0.0.1:13310` is closed or disconnected.

### 5.2 Zonendpoint Client Safety Guarantees
In accordance with Zonendpoint MVP safety rules:
1. **Never Report CLEAN:** When the engine connection fails, times out, or drops, the verdict must **NEVER** fall back to `CLEAN`.
2. **Explicit Offline State:** The client UI displays an offline warning:
   > *"Cannot connect to ZonEndpoint server"*
3. **File Verdicts:** Any active scans in progress must record `ERROR` or `UNKNOWN`.
4. **Lifecycle Control:** The scan lifecycle must transition back to idle, allowing the user to start a new scan once connectivity is restored.

### 5.3 Recovery Steps
1. Verify SSH tunnel on Windows host:
   ```powershell
   # Check if tunnel process is running
   Get-Process -Name ssh -ErrorAction SilentlyContinue | Where-Object { $_.CommandLine -like "*13310*" }
   ```
2. Check remote clamd service status:
   ```bash
   ssh -i ~/.ssh/zonendpoint_lab -o BatchMode=yes admzon@100.96.250.245 "systemctl status clamav-daemon --no-pager"
   ```
3. If clamd is stopped, restart it `[REQUIRES ZON APPROVAL]`:
   ```bash
   ssh -i ~/.ssh/zonendpoint_lab -o BatchMode=yes admzon@100.96.250.245 "sudo systemctl start clamav-daemon"
   ```

---

## 6. Proposed Change List for Zon `[ALL REQUIRE ZON APPROVAL]`

The following operational improvements are submitted for Zon's review and approval before execution:

| # | Proposal | Current Setting | Proposed Value | Justification |
|---|---|---|---|---|
| **1** | Systemd Auto-Restart for clamd | `Restart=no` in `clamav-daemon.service` | `Restart=on-failure`, `RestartSec=5s` (via systemd drop-in `/etc/systemd/system/clamav-daemon.service.d/restart.conf`) | Ensures clamd automatically recovers from transient crashes or OOM termination. |
| **2** | Systemd Auto-Restart for freshclam | `Restart=no` in `clamav-freshclam.service` | `Restart=on-failure`, `RestartSec=30s` (via systemd drop-in `/etc/systemd/system/clamav-freshclam.service.d/restart.conf`) | Ensures freshclam daemon recovers automatically if stopped abnormally. |
| **3** | Known-Good Database Backup Job | No database backup exists | Daily cron job creating verified copy in `/var/backups/clamav-known-good/` | Protects against corrupt database downloads or filesystem damage preventing clamd startup. |
| **4** | Freshclam Failure Alerting | No `OnErrorExecute` configured | Configure `OnErrorExecute` or freshness check script logging to syslog | Provides visibility into prolonged upstream CDN or update failures. |
| **5** | SSH Tunnel Keepalive | `ClientAliveInterval 0` in sshd | `ClientAliveInterval 30`, `ClientAliveCountMax 3` | Prevents SSH port-forwarding tunnel `127.0.0.1:13310` from silently hanging over Tailscale NAT during idle periods. |
| **6** | Stream / File Size Limits Alignment | `StreamMaxLength 25M`, `MaxFileSize 25M` | Evaluate whether to increase to `50M` or keep `25M` and enforce explicit client-side skip/error | Files >25MB streamed via INSTREAM currently fail with `INSTREAM size limit exceeded. ERROR`. Requires product decision. |
| **7** | SSH Password Authentication Hardening | `PasswordAuthentication yes` in `50-cloud-init.conf` | `PasswordAuthentication no` | Server is accessed via Ed25519 key `zonendpoint_lab`. Disabling password auth reduces brute-force attack surface. |
