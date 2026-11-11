#!/bin/bash
# Zonendpoint ThreatFox ClamAV Hash Signature Rollback Script (Task J-02)
# Usage:
#   sudo /usr/local/bin/zonendpoint-threatfox-rollback.sh         # Reverts to previous .hsb backup
#   sudo /usr/local/bin/zonendpoint-threatfox-rollback.sh --purge # Purges threatfox.hsb entirely from clamd

set -euo pipefail

BACKUP_DIR="/var/backups/clamav-threatfox"
BACKUP_FILE="$BACKUP_DIR/threatfox.hsb.bak"
ACTIVE_FILE="/var/lib/clamav/threatfox.hsb"

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1"
}

reload_clamd() {
    log "Triggering zero-downtime clamd reload via zRELOAD socket command..."
    python3 -c "import socket; s = socket.create_connection(('127.0.0.1', 3310), timeout=10); s.sendall(b'zRELOAD\x00'); s.close()" || clamdscan --reload || true

    log "Verifying clamd health via PING..."
    local attempts=0
    local ok=0
    while [ $attempts -lt 60 ]; do
        sleep 2
        RESP=$(python3 -c "import socket; s = socket.create_connection(('127.0.0.1', 3310), timeout=5); s.sendall(b'PING\n'); print(s.recv(1024).decode().strip()); s.close()" 2>/dev/null || true)
        if [ "$RESP" = "PONG" ]; then
            ok=1
            break
        fi
        attempts=$((attempts + 1))
    done

    if [ $ok -ne 1 ]; then
        log "ERROR: clamd PING check failed after reload!"
        exit 2
    fi
    log "clamd health verified: PING -> PONG."
}

if [ "${1:-}" = "--purge" ]; then
    log "Purging ThreatFox hash signatures from active clamd..."
    rm -f "$ACTIVE_FILE"
    reload_clamd
    log "SUCCESS: threatfox.hsb purged. clamd reloaded and verified (PONG)."
    exit 0
fi

if [ ! -f "$BACKUP_FILE" ]; then
    log "ERROR: No rollback backup found at $BACKUP_FILE"
    exit 1
fi

log "Restoring ThreatFox signatures from $BACKUP_FILE..."
TMP_FILE="${ACTIVE_FILE}.tmp"
cp -p "$BACKUP_FILE" "$TMP_FILE"
chown clamav:clamav "$TMP_FILE" 2>/dev/null || true
chmod 0644 "$TMP_FILE"
mv -f "$TMP_FILE" "$ACTIVE_FILE"

reload_clamd
log "SUCCESS: Rollback completed. clamd PING -> PONG verified."
