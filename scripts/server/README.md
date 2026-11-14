# ZonEndpoint Server Signature Count Service

Extracts active ClamAV signature count and metadata from `clamav-daemon` systemd journal logs to `/var/lib/zonendpoint/status/signatures.json`.

## Installation

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

## Output Format

File `/var/lib/zonendpoint/status/signatures.json` (permissions `0644`):
```json
{
  "signatures": 8616478,
  "loaded_at_utc": "2026-11-14T03:39:43Z",
  "engine": "ClamAV 1.5.4/28182"
}
```

## Uninstallation

```bash
sudo systemctl disable --now zonendpoint-sigcount.timer
sudo rm -f /etc/systemd/system/zonendpoint-sigcount.timer
sudo rm -f /etc/systemd/system/zonendpoint-sigcount.service
sudo rm -f /usr/local/bin/zonendpoint-sigcount.py
sudo systemctl daemon-reload
# Optional: remove generated status directory
sudo rm -rf /var/lib/zonendpoint/status
```
