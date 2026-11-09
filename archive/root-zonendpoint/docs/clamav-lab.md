# Ubuntu ClamAV lab

Setup verified 2026-11-07 (Asia/Bangkok).

- SSH: admzon@100.96.250.245, dedicated local key ~/.ssh/zonendpoint_lab.
- Ubuntu 26.04.1 LTS, ClamAV package 1.5.4; daily database 28175 (355849 signatures).
- clamav-daemon and clamav-freshclam enabled and active.
- clamd TCP binds only 127.0.0.1:3310. No external scan port or firewall change.
- Config backup: /etc/clamav/clamd.conf.zonendpoint-original. Reload/version commands enabled for updater notification and engine identity.
- Local Windows tunnel started (ssh process 5448); endpoint 127.0.0.1:13310. It lasts only while that SSH process runs.

Reopen tunnel in PowerShell:

```powershell
ssh -i "$env:USERPROFILE\.ssh\zonendpoint_lab" -o ExitOnForwardFailure=yes -N -L 127.0.0.1:13310:127.0.0.1:3310 admzon@100.96.250.245
```

Verified: Ubuntu clamdscan benign stdin OK; Windows framed zINSTREAM over SSH tunnel returned stream: OK. This is real-engine protocol smoke evidence, not native Zonendpoint integration acceptance.

NOT RUN: EICAR detection, full failure/cancel matrix, native MVP integration and installer. Restore intended MVP snapshot before Victor/Qeen integration tests.

VM reports 2026-11-06 16:19 UTC despite client date 2026-11-07; timedatectl reports NTP synchronized. Resolve clock discrepancy before time-sensitive licensing or timestamp comparisons.
