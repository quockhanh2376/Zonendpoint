# Lab ClamAV Server Read-Only Audit (V/Q-ENGINE-OPS)

- **Date:** 2026-11-07
- **Host:** `udevlevel73` (`100.96.250.245` via Tailscale)
- **Auditor:** Victor (Vic) — Task V-9 (ENGINE-OPS)
- **Mode:** READ-ONLY (no package changes, no config edits, no service restart/reload, no freshclam run, no file writes outside session)
- **Raw Evidence Directory:** `E:\Zonendpoint-tools\artifacts\engine-ops\2026-11-07\`

---

## 1. Findings Summary Table

| Category | Item | Verdict | Evidence File | Summary / Impact |
|---|---|---|---|---|
| **OS & Service** | OS & ClamAV Version | **OK** | `01-os-systemd.txt` | Ubuntu 26.04.1 LTS (kernel 7.0.0-43-generic), ClamAV 1.5.4 (official deb packages `1.5.4+dfsg-0ubuntu0.26.04.1`). |
| **OS & Service** | Service Startup & Enabled | **OK** | `01-os-systemd.txt` | `clamav-daemon.service`, `clamav-freshclam.service`, `clamav-daemon.socket` are all enabled and active. |
| **Resilience** | Systemd Restart Policy | **RISK** | `01-os-systemd.txt` | Both services have `Restart=no`. If clamd or freshclam crashes or is killed by OOM, systemd will NOT automatically restart them. |
| **Signatures** | Signature Inventory | **OK** | `02-signature-inventory.txt` | Official Cisco Talos DB: `main.cvd` (v63, 3,287,027 sigs), `daily.cld` (v28176, 355,854 sigs), `bytecode.cvd` (v339, 80 sigs). Total: 3,642,961 signatures. Digital signatures verified OK. |
| **Signatures** | Third-party / Unofficial DB | **OK** | `02-signature-inventory.txt` | No unofficial signature files found in `/var/lib/clamav/`. Temporary lab test rule was cleanly uninstalled earlier. |
| **Update Health** | Freshclam Update Cycle | **OK** | `03-update-health.txt` | `Checks 24` (hourly checks). Up-to-date checks observed hourly in `freshclam.log`. `daily.cld` last updated 2026-11-07 08:19:15 UTC. |
| **Update Health** | Clamd Reload on Update | **OK** | `03-update-health.txt` | `NotifyClamd /etc/clamav/clamd.conf` triggers database reload upon update. Observed in clamd.log: `Database correctly reloaded (3628258 signatures)` at 08:19:27. |
| **Update Health** | Clamd SelfCheck | **OK** | `03-update-health.txt` | `SelfCheck 3600` configured. Periodic hourly self-check logs confirmed (`SelfCheck: Database status OK`). |
| **Update Health** | Update Failure Alerting | **RISK** | `03-update-health.txt`, `07-backup-recovery.txt` | No `OnErrorExecute` script configured in `freshclam.conf`; hook directory `/etc/clamav/onerrorexecute.d` is empty. Freshclam update failures would go unnoticed until manual inspection. |
| **Network Boundary**| Clamd Loopback Binding | **OK** | `04-listener-boundary.txt` | `TCPAddr 127.0.0.1`, `TCPSocket 3310`. `ss -ltnp` confirms clamd listens strictly on `127.0.0.1:3310`. Not exposed to external network or Tailscale IP. |
| **Network Boundary**| Unix Socket Binding | **OK** | `04-listener-boundary.txt` | `/run/clamav/clamd.ctl` mode 0666, group clamav. |
| **Network Boundary**| SSH Tunneling Settings | **OK** | `04-listener-boundary.txt` | `AllowTcpForwarding yes`, `GatewayPorts no` (prevents remote tunnel exposure). Windows port forward `127.0.0.1:13310 -> 127.0.0.1:3310` operates securely. |
| **Network Boundary**| SSH Keepalive Interval | **RISK** | `04-listener-boundary.txt` | `ClientAliveInterval 0`. Server sends no keepalive packets. Long idle scan connections across Tailscale NAT may silently hang or disconnect. |
| **Network Boundary**| SSH Password Authentication | **RISK** | `04-listener-boundary.txt` | `/etc/ssh/sshd_config.d/50-cloud-init.conf` specifies `PasswordAuthentication yes`. SSH key auth is active, but password auth is still permitted. |
| **Limits & Bounds** | Stream & File Size Limits | **NEEDS DECISION** | `05-limits-resources.txt` | `StreamMaxLength 25M`, `MaxFileSize 25M`, `MaxScanSize 100M`. Client streaming a file >25MB will trigger `INSTREAM size limit exceeded. ERROR`. Product decision needed on whether to raise limit or bound client-side. |
| **Limits & Bounds** | Timeout & Thread Limits | **OK** | `05-limits-resources.txt` | `ReadTimeout 180`, `CommandReadTimeout 30`, `SendBufTimeout 200`, `MaxScanTime 120000` (ms), `MaxThreads 12`. Adequate for single-client pilot. |
| **Resources** | RAM & Swap Headroom | **OK** | `05-limits-resources.txt` | Total RAM 7.2 GiB; used 1.7 GiB, free 4.8 GiB, available 5.6 GiB. Swap 4.0 GiB (0 used). Clamd RSS is 1.06 GiB (~14.5% RAM). Freshclam RSS is 46.5 MiB. |
| **Resources** | Disk Storage & Inodes | **OK** | `05-limits-resources.txt` | Root volume `/dev/mapper/ubuntu--vg-ubuntu--lv` has 46 GiB free (15% used), inodes 3% used. |
| **Permissions** | Service User & Ownership | **OK** | `06-permissions-ownership.txt` | Runs as dedicated system user `clamav:clamav` (uid 104, gid 108, shell `/bin/false`). `/var/lib/clamav` is `0755 clamav:clamav`. Sockets are properly isolated. |
| **Permissions** | Log File Permissions | **OK** | `06-permissions-ownership.txt` | `/var/log/clamav/clamav.log` and `freshclam.log` are `0640 clamav:clamav`. Restricts unauthorized local read. |
| **Backup & Rollback**| Known-good Database Backup | **RISK** | `07-backup-recovery.txt` | **No backup of CVD/CLD files exists anywhere on the server.** `/var/backups` contains only package states. |
| **Backup & Rollback**| Service Condition on Bad DB | **RISK** | `01-os-systemd.txt`, `07-backup-recovery.txt` | `clamav-daemon.service` has `ConditionPathExistsGlob=/var/lib/clamav/{main,daily}.{cld,cvd,inc}`. If `daily.cld` is corrupted or deleted during a failed update or manual repair, clamd will refuse to start. |

---

## 2. Detailed Audit Findings by Section

### 2.1 OS, Binaries, and Systemd Units (`01-os-systemd.txt`)
- **OS Platform:** Ubuntu 26.04.1 LTS (Resolute Raccoon), Linux Kernel `7.0.0-43-generic` x86_64 running under VMware hypervisor.
- **Binaries:**
  - `/usr/sbin/clamd`: ClamAV 1.5.4
  - `/usr/bin/freshclam`: ClamAV 1.5.4
  - `/usr/bin/sigtool`: ClamAV 1.5.4
- **Packages:** Installed via official Ubuntu archive (`1.5.4+dfsg-0ubuntu0.26.04.1`):
  - `clamav`
  - `clamav-base`
  - `clamav-daemon`
  - `clamav-freshclam`
- **Systemd Units:**
  - `clamav-daemon.service`: Active (running) since `Fri 2026-11-06 16:19:39 UTC` (PID 8042). UnitFileState: `enabled`.
  - `clamav-freshclam.service`: Active (running) since `Fri 2026-11-06 16:19:00 UTC` (PID 2711). UnitFileState: `enabled`.
  - `clamav-daemon.socket`: Active (running) since boot. UnitFileState: `enabled`.
- **Systemd Socket Generator:**
  `/run/systemd/generator/clamav-daemon.socket.d/listen.conf` is dynamically generated from `/etc/clamav/clamd.conf` and binds socket streams:
  - `ListenStream=127.0.0.1:3310`
  - `ListenStream=/var/run/clamav/clamd.ctl`
- **Restart Policy Risk:**
  Both `clamav-daemon` and `clamav-freshclam` have `Restart=no`. If either daemon terminates unexpectedly (crash, assertion, OOM-kill), it stays dead until manual administrator intervention.

### 2.2 Signature Inventory (`02-signature-inventory.txt`)
- **Database Directory:** `/var/lib/clamav`
- **Files Present:**
  - `main.cvd`: 89,072,577 bytes, mtime `2026-11-06 16:19:08 UTC`. Version 63, 3,287,027 signatures, builder `tomjudge`. Digital signature verification OK.
  - `daily.cld`: 86,238,208 bytes, mtime `2026-11-07 08:19:15 UTC`. Version 28176, 355,854 signatures, builder `svc.clamav-publisher`. Digital signature verification OK.
  - `bytecode.cvd`: 281,702 bytes, mtime `2026-11-06 16:19:12 UTC`. Version 339, 80 signatures, builder `nrandolp`. Digital signature verification OK.
  - `main-63.cvd.sign`: 9,078 bytes (detached cryptographic signature).
  - `bytecode-339.cvd.sign`: 9,078 bytes (detached cryptographic signature).
  - `freshclam.dat`: 90 bytes (metadata tracking UUID and mirror state).
- **Total Signatures:** 3,642,961 active official signatures.
- **Third-Party Signatures:** None. No custom rules, Sanesecurity, SecuriteInfo, or loose `.ndb`/`.hdb`/`.yar` files are present.

### 2.3 Update Health and Reload Verification (`03-update-health.txt`)
- **Freshclam Configuration (`/etc/clamav/freshclam.conf`):**
  - `DatabaseMirror db.local.clamav.net`, `database.clamav.net`
  - `Checks 24` (hourly checks)
  - `TestDatabases yes` (freshclam verifies integrity of downloaded CVD prior to install)
  - `ScriptedUpdates yes` (downloads binary patch diffs `.cdiff` to minimize bandwidth)
  - `NotifyClamd /etc/clamav/clamd.conf` (notifies clamd daemon upon successful database replacement)
  - `ConnectTimeout 30`, `ReceiveTimeout 0`, `MaxAttempts 5`
- **Update Logs:**
  `freshclam.log` confirms hourly wakeups and database checks (`daily.cld database is up-to-date`, `main.cvd database is up-to-date`, `bytecode.cvd database is up-to-date`).
- **Reload Evidence:**
  `clamav.log` records live in-memory reload events triggered by freshclam:
  ```
  Sat Nov  7 08:19:18 2026 -> Reading databases from /var/lib/clamav
  Sat Nov  7 08:19:27 2026 -> Database correctly reloaded (3628258 signatures)
  Sat Nov  7 08:19:27 2026 -> Activating the newly loaded database...
  ```
  Total reload duration: 9 seconds. The old database remains active serving scan requests while the new database loads in the background, resulting in zero scan downtime.
- **SelfCheck Verification:**
  `SelfCheck 3600` is active. Logs show regular hourly checks:
  `Sat Nov  7 14:54:47 2026 -> SelfCheck: Database status OK.`

### 2.4 Listener Boundary and Network Exposure (`04-listener-boundary.txt`)
- **Clamd Listener:**
  - `TCPAddr 127.0.0.1`
  - `TCPSocket 3310`
  - `ss -ltnp` confirms: `127.0.0.1:3310` (no binding to `0.0.0.0`, `::`, or the Tailscale interface `100.96.250.245`).
- **Unix Domain Socket:**
  `/run/clamav/clamd.ctl` exists with mode `0666` (`srw-rw-rw-`).
- **OpenSSH Daemon (`sshd -T`):**
  - `AllowTcpForwarding yes`: Enables local port forwarding `ssh -L 13310:127.0.0.1:3310`.
  - `GatewayPorts no`: Prevents clients from binding forwarded ports to external interfaces.
  - `PermitTunnel no`: Layer 2/3 TUN/TAP device forwarding is disabled.
  - `ClientAliveInterval 0`: Risk — no server keepalive probes are transmitted, which may cause stale TCP connections across Tailscale NAT.
  - `PasswordAuthentication yes` in `50-cloud-init.conf`: Risk — password authentication is still permitted on SSH.

### 2.5 Resource Allocations and Limits (`05-limits-resources.txt`)
- **Scanning Limits:**
  - `StreamMaxLength 25M`
  - `MaxFileSize 25M`
  - `MaxScanSize 100M`
  - `MaxThreads 12`
  - `MaxConnectionQueueLength 15`
  - `MaxQueue 100`
  - `ReadTimeout 180` (seconds)
  - `CommandReadTimeout 30` (seconds)
  - `SendBufTimeout 200` (seconds)
  - `MaxScanTime 120000` (ms = 120s)
- **Memory Footprint:**
  - System Total RAM: 7.2 GiB, Free: 4.8 GiB, Available: 5.6 GiB.
  - clamd RSS: 1,106,588 KB (~1.06 GiB, ~14.5% of RAM).
  - clamd VSZ: 1,301,720 KB.
  - freshclam RSS: 47,616 KB (~46.5 MiB).
  - Headroom: Clamd memory usage is stable with ~5.6 GiB RAM available. No swap usage observed.
- **Storage:**
  - Root partition `/`: 46 GiB free (15% used). Inodes: 3% used.

### 2.6 Ownership and Permissions (`06-permissions-ownership.txt`)
- **System Account:**
  User `clamav` (uid=104), Group `clamav` (gid=108), shell `/bin/false`, home `/var/lib/clamav`.
- **Directory Permissions:**
  - `/var/lib/clamav`: `drwxr-xr-x clamav:clamav` (`0755`). Database files are `-rw-r--r-- clamav:clamav` (`0644`).
  - `/var/log/clamav`: `drwxr-xr-x clamav:clamav` (`0755`). Log files `clamav.log` and `freshclam.log` are `-rw-r----- clamav:clamav` (`0640`).
  - `/run/clamav`: `drwxr-xr-x clamav:root` (`0755`). Socket `clamd.ctl` is `srw-rw-rw- clamav:clamav` (`0666`).
  - `/etc/clamav`: Configuration files owned by `root:root` mode `0644`, except `freshclam.conf` which is `0444 clamav:adm`.
- **Preserved Original Config:**
  `/etc/clamav/clamd.conf.zonendpoint-original` is preserved on disk for exact baseline comparison.

### 2.7 Backup, Failure Recovery, and Failure Modes (`07-backup-recovery.txt`)
- **Existing Backups:** NONE. No snapshot, tarball, or secondary copy of CVD files exists on the machine.
- **Automated Tasks:** No custom cron jobs or backup systemd timers are configured for ClamAV.
- **Failure Mode Analysis (Bad DB / Corrupt CVD):**
  1. *Online Freshclam Download Failure:* Freshclam writes new CVDs to temporary files (`.tmp`) and tests them with internal verification routines (`TestDatabases yes`). If verification fails, freshclam rejects the download and leaves existing active database files untouched.
  2. *Corrupt Database in `/var/lib/clamav`:* If a database file becomes corrupted on disk (e.g. storage error or bad manual injection), clamd's reload will fail and clamd will log an error while continuing to serve with the previously loaded in-memory database.
  3. *Daemon Restart with Corrupt Database:* If `clamd` restarts while `/var/lib/clamav/daily.cld` or `main.cvd` is missing or corrupt, clamd cannot initialize and the systemd unit `clamav-daemon.service` will enter a failed state.
  4. *Client Impact:* If `clamd` is down or unstartable, client connections to `127.0.0.1:13310` (or `127.0.0.1:3310`) will immediately fail with connection refused / timeout. In accordance with Zonendpoint safety rules, the client displays **"Cannot connect to ZonEndpoint server"** and records `ERROR` / `UNKNOWN` — never `CLEAN`.
  5. *Absence of Failure Notification:* Freshclam has no notification hook on failure (`OnErrorExecute` is unset; `/etc/clamav/onerrorexecute.d` is empty).
