# Operations Report: Lab YARA Ruleset Decoupling & Automated Daily Updater (Task V-25)

**Date**: 2026-11-09  
**Operator**: Victor (Vic)  
**Branch**: `vic/yara-daily`  
**Target Host**: Lab ClamAV Engine Host `udevlevel73` (`admzon@100.96.250.245`, Ubuntu 26.04.1 LTS x86_64)  
**ClamAV Daemon**: ClamAV 1.5.4, PID 2245  
**Status**: COMPLETED & VERIFIED PASS  

---

## 1. Executive Summary & Approval Scope

In accordance with Zon's approval for Task V-25, two major operational enhancements were executed and verified on the lab ClamAV server:
1. **Part A: Decoupled ZonQA test control rule from production clamd**:
   - `zonqa-control.yar` moved out of ClamAV database directory `/var/lib/clamav/` into dedicated QA directory `/opt/zonendpoint/qa/`.
   - Clamd reloaded with zero downtime; live clamd no longer loads test signatures.
   - QA verification confirmed using temporary DB (`clamscan -d /opt/zonendpoint/qa/zonqa-control.yar <file>`).
2. **Part B: Automated daily YARA-Forge Core updates via systemd timer**:
   - Timer scheduled nightly with randomized jitter to query official releases from `YARAHQ/yara-forge`.
   - Automatic download, checksum checking, ClamAV-specific filtering (`filter_yara_clamav.py`, excluding all `SUSP_` heuristics).
   - Pre-flight validation gate on isolated temporary DB (syntax loading, rule count sanity, benign clean check, ZonQA marker detection).
   - Atomic zero-downtime deployment, automated backup retention, clamd socket reload, and PING -> PONG verification.
   - Idempotent: halts immediately without downloading if no new release exists.

---

## 2. Part A: Gỡ luật test ZonQA khỏi clamd thật

### 2.1 Action Executed
- Created QA rule directory: `/opt/zonendpoint/qa/` (mode `0755`, owner `root:root`).
- Moved test rule:
  ```bash
  sudo mv /var/lib/clamav/zonqa-control.yar /opt/zonendpoint/qa/zonqa-control.yar
  sudo chown root:root /opt/zonendpoint/qa/zonqa-control.yar
  sudo chmod 0644 /opt/zonendpoint/qa/zonqa-control.yar
  ```
- Triggered zero-downtime clamd reload: `sudo clamdscan --reload`.

### 2.2 Verification Evidence
1. **Signature Count Adjustment**:
   - Pre-change: 3,631,937 signatures.
   - Post-reload: **3,631,936 signatures** (exact delta: -1).
   - `/var/log/clamav/clamav.log`:
     ```text
     Mon Nov  9 14:44:11 2026 -> Reading databases from /var/lib/clamav
     Mon Nov  9 14:44:20 2026 -> Database correctly reloaded (3631936 signatures)
     Mon Nov  9 14:44:20 2026 -> Activating the newly loaded database...
     ```
2. **Live clamd Benign Marker Scan**:
   - Streaming `ZONQA_BENIGN_MARKER_TEST_STRING_20261108_CLAMAV_LAB` directly to clamd on `127.0.0.1:3310`:
     - Response: `stream: OK` (Clean verdict, not flagged).
3. **QA Standalone / Temporary DB Scan**:
   - Scanning benign marker via temporary DB:
     ```bash
     clamscan -d /opt/zonendpoint/qa/zonqa-control.yar <target>
     ```
     - Result: `YARA.ZonQA_Benign_Marker.UNOFFICIAL FOUND` (exit code 1, Infected files: 1).

---

## 3. Part B: Automated Daily YARA-Forge Core Updater

### 3.1 Architecture & Components

| Component | Path | Description |
|---|---|---|
| **Updater Script** | `/usr/local/bin/zonendpoint-yara-update.py` | Python 3.14 script executing release query, download, filtering, temp DB validation, atomic swap, and reload. |
| **Filter Script** | `/opt/zonendpoint/yara/filter_yara_clamav.py` | ClamAV compatibility filter with heuristic `SUSP_` exclusion and validation loop. |
| **Rollback Script** | `/usr/local/bin/zonendpoint-yara-rollback.sh` | Fast rollback script restoring previous ruleset or purging YARA completely. |
| **Current Tag** | `/opt/zonendpoint/yara/current_tag.txt` | Tracks the currently installed upstream release tag. |
| **Backup Storage** | `/var/backups/clamav-yara/` | Stores `yara-forge-core.yar.bak` and `previous_tag.txt`. |
| **Systemd Service** | `/etc/systemd/system/zonendpoint-yara-update.service` | Oneshot unit executing updater script with journal logging (`SyslogIdentifier=zonendpoint-yara-update`). |
| **Systemd Timer** | `/etc/systemd/system/zonendpoint-yara-update.timer` | Timer firing nightly with randomized delay. |

### 3.2 Systemd Timer Configuration & Schedule

- **Unit file**: `/etc/systemd/system/zonendpoint-yara-update.timer`
  ```ini
  [Unit]
  Description=Zonendpoint Daily YARA-Forge Core Feed Update Timer
  Documentation=https://github.com/YARAHQ/yara-forge

  [Timer]
  # Fires daily between 20:00 and 22:00 UTC (03:00 - 05:00 AM ICT night)
  OnCalendar=*-*-* 20:00:00 UTC
  RandomizedDelaySec=7200
  Persistent=true

  [Install]
  WantedBy=timers.target
  ```
- **Execution Window**:
  - **UTC**: 20:00:00 – 22:00:00 UTC.
  - **ICT (Vietnam Local Time, UTC+7)**: 03:00:00 – 05:00:00 AM ICT ("giờ ngẫu nhiên trong đêm").
  - **Observed Next Trigger**: `Mon 2026-11-09 21:21:50 UTC` (~04:21:50 AM ICT).

### 3.3 Four-Step Verification Protocol

1. **Step 1: Release Check**:
   - Queries `https://api.github.com/repos/YARAHQ/yara-forge/releases/latest`.
   - Reads `tag_name`. If `tag_name == current_tag`, logs no-update message and halts immediately (0 bytes downloaded).
2. **Step 2: Core Package Download & Checksum**:
   - Downloads `yara-forge-rules-core.zip`.
   - Computes SHA256; validates against upstream `.sha256` asset or release body hash if available.
   - Extracts `packages/core/yara-rules-core.yar`.
   - Filters rules using `filter_yara_clamav.py` (excludes all `SUSP_` rules, checks imports, syntax, and endian functions).
3. **Step 3: Pre-flight Validation on Temporary DB**:
   - **Rule count sanity check**: Must be between 2,500 and 8,000 rules.
   - **Syntax & engine loading**: Runs `clamscan -d <temp_db>/yara-forge-core.yar /etc/hosts`. Requires exit code 0 and 0 LibClamAV errors.
   - **Benign sample clean check**: Scans `/etc/hosts`, `/usr/bin/python3`, `/etc/passwd`. Requires exit code 0 and 0 infections.
   - **Positive control check**: Temporarily loads `/opt/zonendpoint/qa/zonqa-control.yar` into temp DB, scans benign test marker. Requires exit code 1 and `YARA.ZonQA_Benign_Marker.UNOFFICIAL FOUND`.
   - **Failure handling**: If any check fails, active database remains unmodified, error logged, and script exits non-zero.
4. **Step 4: Atomic Deployment & Rollback Retention**:
   - Backs up active `/var/lib/clamav/yara-forge-core.yar` to `/var/backups/clamav-yara/yara-forge-core.yar.bak`.
   - Backs up tag to `/var/backups/clamav-yara/previous_tag.txt`.
   - Atomically deploys to `/var/lib/clamav/yara-forge-core.yar` via `.tmp` file and `os.replace` (owner `clamav:clamav`, mode `0644`).
   - Updates `/opt/zonendpoint/yara/current_tag.txt`.
   - Reloads clamd: `clamdscan --reload`.
   - Verifies loopback health: `127.0.0.1:3310` PING -> PONG. If failed, triggers emergency rollback to backup file.

---

## 4. Manual Test Run Evidence

### 4.1 First Manual Execution (Upgrading to Release 20261108)

Command executed on lab server:
```bash
sudo /usr/local/bin/zonendpoint-yara-update.py
```

Observed output:
```text
[2026-11-09 14:48:04] Starting Zonendpoint YARA-Forge Core updater...
[2026-11-09 14:48:04] Current installed release tag: '20261103'
[2026-11-09 14:48:04] Querying latest release from https://api.github.com/repos/YARAHQ/yara-forge/releases/latest...
[2026-11-09 14:48:05] Latest upstream release tag: '20261108'
[2026-11-09 14:48:05] Downloading from https://github.com/YARAHQ/yara-forge/releases/download/20261108/yara-forge-rules-core.zip to /opt/zonendpoint/yara/work/yara-forge-rules-core-20261108.zip...
[2026-11-09 14:48:06] Downloaded 1,744,187 bytes.
[2026-11-09 14:48:06] Downloaded zip SHA256: 9a01ef66c573991036cbd2e805f59d5b5e8ca3a56375e567b6a3433855f74707
[2026-11-09 14:48:06] No upstream checksum provided for core zip; recorded local SHA256.
[2026-11-09 14:48:06] Extracted packages/core/yara-rules-core.yar (7,843,506 bytes).
[2026-11-09 14:48:06] Filtering rules via /opt/zonendpoint/yara/filter_yara_clamav.py (excluding SUSP_ heuristics & running validation loop)...
[2026-11-09 14:48:08] Filter completed: 3,676 rules accepted, 1,456 rejected.
[2026-11-09 14:48:08] Verified: 0 SUSP_ heuristic rules in accepted ruleset.
[2026-11-09 14:48:08] === Running Pre-flight Validation on Temporary DB ===
[2026-11-09 14:48:08] Rule count sanity check PASS (3676 rules within [2500, 8000]).
[2026-11-09 14:48:08] Testing rule load via clamscan on benign /etc/hosts...
[2026-11-09 14:48:08] Rule load test PASS (0 load errors).
[2026-11-09 14:48:08] Scanning 3 benign system files with temporary DB...
[2026-11-09 14:48:09] Benign samples test PASS (0 False Positives).
[2026-11-09 14:48:09] Testing ZonQA positive control rule on temporary DB...
[2026-11-09 14:48:10] ZonQA positive control marker detection PASS.
[2026-11-09 14:48:10] All temporary DB pre-flight checks PASSED successfully.
[2026-11-09 14:48:10] === Deploying Filtered Ruleset Atomically ===
[2026-11-09 14:48:10] Backing up current /var/lib/clamav/yara-forge-core.yar -> /var/backups/clamav-yara/yara-forge-core.yar.bak...
[2026-11-09 14:48:10] Recorded previous release tag: 20261103
[2026-11-09 14:48:10] Atomically updated /var/lib/clamav/yara-forge-core.yar.
[2026-11-09 14:48:10] Triggering zero-downtime clamd reload via clamdscan --reload...
[2026-11-09 14:48:18] Verifying clamd health via PING...
[2026-11-09 14:48:18] ============================================================
[2026-11-09 14:48:18] SUCCESS: YARA-Forge Core update completed successfully!
[2026-11-09 14:48:18]   Release Tag:    20261108
[2026-11-09 14:48:18]   Accepted Rules: 3,676
[2026-11-09 14:48:18]   Active File:    /var/lib/clamav/yara-forge-core.yar
[2026-11-09 14:48:18]   SHA256:         fdbf84cff5e50a274f8e53476f22cff38c06bb65dc38fdeae4ef5d689433d6cc
[2026-11-09 14:48:18]   clamd Status:   PONG (127.0.0.1:3310 connected)
[2026-11-09 14:48:18] ============================================================
[2026-11-09 14:48:18] Cleaned up temporary work files.
```

### 4.2 Second Manual Execution (Idempotency / No-Update Verification)

Command executed on lab server immediately following upgrade:
```bash
sudo /usr/local/bin/zonendpoint-yara-update.py
```

Observed output (completed in <1 second, 0 bytes downloaded):
```text
[2026-11-09 14:48:29] Starting Zonendpoint YARA-Forge Core updater...
[2026-11-09 14:48:29] Current installed release tag: '20261108'
[2026-11-09 14:48:29] Querying latest release from https://api.github.com/repos/YARAHQ/yara-forge/releases/latest...
[2026-11-09 14:48:30] Latest upstream release tag: '20261108'
[2026-11-09 14:48:30] Installed release '20261108' is already up-to-date with upstream. No update needed. Halting.
```

### 4.3 Post-Update Active Database & Hash Inventory

| File | Target Path | Size | SHA256 Hash |
|---|---|---|---|
| Active YARA-Forge Core (Release `20261108`, 3,676 rules) | `/var/lib/clamav/yara-forge-core.yar` | 5,825,979 B | `fdbf84cff5e50a274f8e53476f22cff38c06bb65dc38fdeae4ef5d689433d6cc` |
| Rollback Backup (Release `20261103`, 3,678 rules) | `/var/backups/clamav-yara/yara-forge-core.yar.bak` | 5,828,643 B | `f341d83a2ae0dae55f17ace41d2ed59d951fd2b79543b0dac130f81cec960ce6` |
| Decoupled QA Control Marker | `/opt/zonendpoint/qa/zonqa-control.yar` | 283 B | `2b7cbfc0382626715eae1b5a6e71cb1564840f355185bec978054d9514de56a9` |

- **Total Active Signatures**: **3,631,934** (3,628,258 baseline CVD/CLD + 3,676 YARA rules).
- Clamd log confirmation:
  ```text
  Mon Nov  9 14:48:18 2026 -> Database correctly reloaded (3631934 signatures)
  Mon Nov  9 14:48:18 2026 -> Activating the newly loaded database...
  ```

---

## 5. Rollback Procedures

### 5.1 Single-Command Rollback to Previous Version (<10 Seconds)

To revert immediately to the previous verified release (`20261103`), run on the lab server:
```bash
sudo /usr/local/bin/zonendpoint-yara-rollback.sh
```

This script:
1. Restores `/var/backups/clamav-yara/yara-forge-core.yar.bak` to `/var/lib/clamav/yara-forge-core.yar`.
2. Restores `/opt/zonendpoint/yara/current_tag.txt` from `/var/backups/clamav-yara/previous_tag.txt`.
3. Sets ownership `clamav:clamav` and mode `0644`.
4. Triggers `clamdscan --reload`.
5. Verifies loopback health: `PING` -> `PONG`.

### 5.2 Emergency Complete Purge of YARA Rules

To remove all custom YARA rules from clamd instantly without affecting standard ClamAV CVD/CLD signatures:
```bash
sudo /usr/local/bin/zonendpoint-yara-rollback.sh --purge
```
Or manually:
```bash
sudo rm -f /var/lib/clamav/yara-forge-core.yar && sudo clamdscan --reload
```

### 5.3 Automated Hold Mechanism & Resuming Updates (L-3)

Executing `zonendpoint-yara-rollback.sh` (or `--purge`) automatically creates a hold file:
```bash
/opt/zonendpoint/HOLD-yara
```

While this file exists:
- `zonendpoint-yara-update.py` will detect the hold file and skip the scheduled daily run immediately.
- Prevents the daily updater from downloading and re-installing the problematic YARA ruleset overnight.

**Chính sách Quản lý Khóa & Gỡ bỏ HOLD:**
- Khóa triển khai (`/run/lock/zonendpoint-clamd-deploy.lock`): Không bao giờ xóa lock bằng `rm -f`. Khóa kernel `flock` tự động giải phóng khi tiến trình nắm giữ kết thúc.
- File HOLD (`/opt/zonendpoint/HOLD-yara`): HOLD chỉ gỡ thủ công sau khi kiểm tra có biên bản điều tra nguyên nhân sự cố. Tuyệt đối không xóa tùy tiện trong quá trình vận hành.
- Triển khai lab dùng duy nhất `E:\Zonendpoint-tools\coordination\deploy\deploy-feeds.sh` (Loki, sau khi Qeen nghiệm thu).

---

## 6. Safety Invariants Verification

All task safety constraints were strictly verified:

1. **Clamd Loopback Binding**: `ss -ltnp 'sport = :3310'` confirms listening strictly on `127.0.0.1:3310`. Not exposed to public/Tailscale interfaces.
2. **SSH PasswordAuthentication**: `sudo /usr/sbin/sshd -T | grep passwordauthentication` confirms `passwordauthentication yes` preserved unmodified.
3. **Existing Timers Active**:
   - `clamav-freshclam.service` (daemon): `active`
   - `clamav-backup-known-good.timer`: `active`
   - `clamav-check-update-health.timer`: `active`
   - `clamav-daemon.service`: `active`
4. **Zero EICAR / Malware**: No malware or EICAR strings were introduced.

---

## 7. Kiểm tra Sandbox Systemd sau Triển khai (Task L-12)

Sau khi triển khai service unit vào `/etc/systemd/system/zonendpoint-yara-update.service`:
```bash
# 1. Kiểm tra tính hợp lệ cú pháp của unit
systemd-analyze verify /etc/systemd/system/zonendpoint-yara-update.service

# 2. Đánh giá mức độ an ninh (sandbox security score)
systemd-analyze security zonendpoint-yara-update.service
```
Dự kiến điểm exposure giảm mạnh nhờ các rào chắn sandbox:
- `ProtectSystem=strict` kèm `ReadWritePaths=/opt/zonendpoint /var/lib/clamav /var/backups /run/lock`
- `PrivateDevices=yes`, `ProtectKernelTunables=yes`, `ProtectKernelModules=yes`, `ProtectControlGroups=yes`
- `RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX`
- `NoNewPrivileges=yes`, `PrivateTmp=yes`
