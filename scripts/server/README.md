# ZonEndpoint Server Signature Count Service & Restricted Tunnel Access

Extracts active ClamAV signature count and metadata from `clamav-daemon` systemd journal logs to `/var/lib/zonendpoint/status/signatures.json`, and provides locked-down SSH access for remote client tunnel and status synchronization.

## 1. Signature Count Service

### Installation

1. Copy script and systemd units:
   ```bash
   sudo cp scripts/server/zonendpoint-sigcount.py /usr/local/bin/zonendpoint-sigcount.py
   sudo chmod 0755 /usr/local/bin/zonendpoint-sigcount.py

   sudo cp scripts/server/zonendpoint-sigcount.service /etc/systemd/system/
   sudo cp scripts/server/zonendpoint-sigcount.timer /etc/systemd/system/
   ```

2. Reload systemd and start:
   ```bash
   sudo systemctl daemon-reload
   sudo systemctl start zonendpoint-sigcount.service
   sudo systemctl enable --now zonendpoint-sigcount.timer
   ```

3. Verify:
   ```bash
   sudo systemctl status zonendpoint-sigcount.timer
   cat /var/lib/zonendpoint/status/signatures.json
   ```

### Output Format

File `/var/lib/zonendpoint/status/signatures.json` (permissions `0644`), standard UTC+7 (`Asia/Ho_Chi_Minh`):
```json
{
  "signatures": 8616497,
  "loaded_at": "2026-11-14T11:41:49+07:00",
  "engine": "ClamAV 1.5.4/28182"
}
```
*(Note: client/tunnel also backwards-compatibly parses legacy `loaded_at_utc` with `Z` suffix)*

### Uninstallation

```bash
sudo systemctl disable --now zonendpoint-sigcount.timer
sudo rm -f /etc/systemd/system/zonendpoint-sigcount.timer
sudo rm -f /etc/systemd/system/zonendpoint-sigcount.service
sudo rm -f /usr/local/bin/zonendpoint-sigcount.py
sudo systemctl daemon-reload
# Optional: remove generated status directory
sudo rm -rf /var/lib/zonendpoint/status
```

---

## 2. Restricted Tunnel & Status Access (Task V-59 / X-2)

To prevent client credentials from having unrestricted shell or sudo access to the lab server, client connections use dedicated restricted keys under the locked system account `zontunnel`.

### Account Architecture
- **User**: `zontunnel` (system account, no sudo/adm groups, shell `/bin/sh` to allow SSH command execution, password locked via `passwd -l`).
- **Keys**:
  - `zonendpoint_tunnel`: Ed25519 key restricted strictly to local port forwarding (`127.0.0.1:3310`). Interactive shell and arbitrary commands are rejected (`command="/usr/bin/false"`).
  - `zonendpoint_status`: Ed25519 key restricted strictly to reading status (`command="cat /var/lib/zonendpoint/status/signatures.json"`). Port forwarding is disabled.

### Server sshd Configuration
Appended to `/etc/ssh/sshd_config`:
```sshd
Match User zontunnel
    AuthenticationMethods publickey
    PasswordAuthentication no
    AllowTcpForwarding local
    PermitOpen 127.0.0.1:3310
    X11Forwarding no
    AllowAgentForwarding no
    PermitTTY no
    GatewayPorts no
```

### Lab Rollback Procedure
If rollback is ever required:
1. Restore `/etc/ssh/sshd_config` from backup (e.g. `/etc/ssh/sshd_config.bak-20261114-v59`) or remove the `Match User zontunnel` block.
2. Validate and reload sshd:
   ```bash
   sudo sshd -t
   sudo systemctl reload ssh
   ```
3. Remove restricted user:
   ```bash
   sudo userdel -r zontunnel
   ```
