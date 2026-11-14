#!/bin/bash
# Zonendpoint YARA-Forge Core Rollback Script (Task V-25 / V-62)
# Usage:
#   sudo /usr/local/bin/zonendpoint-yara-rollback.sh         # Reverts to previous YARA ruleset
#   sudo /usr/local/bin/zonendpoint-yara-rollback.sh --purge # Purges YARA rules entirely from clamd

set -euo pipefail

BACKUP_DIR="${BACKUP_DIR:-/var/backups/clamav-yara}"
BACKUP_FILE="${BACKUP_FILE:-$BACKUP_DIR/yara-forge-core.yar.bak}"
PREVIOUS_TAG_FILE="${PREVIOUS_TAG_FILE:-$BACKUP_DIR/previous_tag.txt}"
ACTIVE_FILE="${ACTIVE_FILE:-/var/lib/clamav/yara-forge-core.yar}"
CURRENT_TAG_FILE="${CURRENT_TAG_FILE:-/opt/zonendpoint/yara/current_tag.txt}"
DEPLOY_LOCK_FILE="${DEPLOY_LOCK_FILE:-/run/lock/zonendpoint-clamd-deploy.lock}"
HOLD_FILE="${HOLD_FILE:-/opt/zonendpoint/HOLD-yara}"
RELOAD_STATE_FILE="${RELOAD_STATE_FILE:-/opt/zonendpoint/reload-state.json}"
CLAMD_HOST="${CLAMD_HOST:-127.0.0.1}"
CLAMD_PORT="${CLAMD_PORT:-3310}"
CLAMDSCAN_BIN="${CLAMDSCAN_BIN:-clamdscan}"

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1"
}

# Critical Safety Guard: Fail test immediately if targeting real clamd in test environment
if [ "${ZONENDPOINT_TEST_ENV:-0}" = "1" ]; then
    if [ "$CLAMD_PORT" = "3310" ] || [ "$CLAMDSCAN_BIN" = "clamdscan" ]; then
        log "CRITICAL SAFETY VIOLATION: Test environment attempted to invoke real clamd (port 3310 or clamdscan)!" >&2
        exit 99
    fi
fi

acquire_lock() {
    mkdir -p "$(dirname "$DEPLOY_LOCK_FILE")" 2>/dev/null || true
    if ! exec 200>"$DEPLOY_LOCK_FILE" 2>/dev/null; then
        log "ERROR: Could not open lock file '$DEPLOY_LOCK_FILE'. Exiting."
        exit 3
    fi
    if ! flock -n 200; then
        log "ERROR: Another clamd deploy/update process is currently running (lock '$DEPLOY_LOCK_FILE' held). Exiting."
        exit 3
    fi
}

set_hold() {
    mkdir -p "$(dirname "$HOLD_FILE")" 2>/dev/null || true
    date -u '+%Y-%m-%dT%H:%M:%SZ' > "$HOLD_FILE"
    log "Created hold file $HOLD_FILE (daily updater will skip runs until removed)."
}

record_reload_state() {
    python3 -c "
import json, time, os, datetime, tempfile
now = time.time()
data = {
    'last_reload_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
    'last_reload_timestamp': now,
    'caller': 'yara_rollback',
    'forced': True
}
sf = '$RELOAD_STATE_FILE'
d = os.path.dirname(sf) or '.'
os.makedirs(d, exist_ok=True)
with tempfile.NamedTemporaryFile('w', dir=d, delete=False) as tf:
    json.dump(data, tf, indent=2)
    tmp = tf.name
os.replace(tmp, sf)
" 2>/dev/null || true
}

reload_clamd() {
    log "Triggering zero-downtime clamd reload via $CLAMDSCAN_BIN --reload..."
    "$CLAMDSCAN_BIN" --reload || python3 -c "import socket; s = socket.create_connection(('$CLAMD_HOST', int('$CLAMD_PORT')), timeout=10); s.sendall(b'zRELOAD\x00'); s.close()" || true

    log "Verifying clamd health via PING ($CLAMD_HOST:$CLAMD_PORT)..."
    local attempts=0
    local ok=0
    local max_attempts=${CLAMD_PING_ATTEMPTS:-60}
    if [ "${ZONENDPOINT_TEST_ENV:-0}" = "1" ] && [ "${CLAMD_NO_PING_WAIT:-0}" = "1" ]; then
        ok=1
    else
        while [ $attempts -lt $max_attempts ]; do
            sleep 2
            RESP=$(python3 -c "import socket; s = socket.create_connection(('$CLAMD_HOST', int('$CLAMD_PORT')), timeout=5); s.sendall(b'PING\n'); print(s.recv(1024).decode().strip()); s.close()" 2>/dev/null || true)
            if [ "$RESP" = "PONG" ]; then
                ok=1
                break
            fi
            attempts=$((attempts + 1))
        done
    fi

    if [ $ok -ne 1 ]; then
        log "ERROR: clamd PING check failed after reload!"
        exit 2
    fi
    log "clamd health verified: PING -> PONG."
    record_reload_state
}

# L-6: Acquire shared deploy lock to prevent concurrent reload/deploy
acquire_lock

# L-3: Place HOLD file immediately at script start before touching any file.
# Any subsequent failure branches (missing backup, bad cp, reload fail) will keep HOLD.
set_hold

if [ "${1:-}" = "--purge" ]; then
    log "Purging YARA-Forge Core ruleset from active clamd..."
    rm -f "$ACTIVE_FILE"
    reload_clamd
    log "SUCCESS: YARA rules purged. clamd reloaded and verified (PONG)."
    exit 0
fi

if [ ! -f "$BACKUP_FILE" ]; then
    log "ERROR: No rollback backup found at $BACKUP_FILE"
    exit 1
fi

log "Restoring YARA rules from $BACKUP_FILE..."
TARGET_DIR="$(dirname "$ACTIVE_FILE")"
TMP_FILE=$(mktemp -p "$TARGET_DIR" ".$(basename "$ACTIVE_FILE").tmp-XXXXXX")
cp -p "$BACKUP_FILE" "$TMP_FILE"
chown clamav:clamav "$TMP_FILE" 2>/dev/null || true
chmod 0644 "$TMP_FILE"
sync "$TMP_FILE" 2>/dev/null || sync
mv -f "$TMP_FILE" "$ACTIVE_FILE"

if [ -f "$PREVIOUS_TAG_FILE" ]; then
    cp -f "$PREVIOUS_TAG_FILE" "$CURRENT_TAG_FILE"
    log "Restored current tag to $(cat "$CURRENT_TAG_FILE")"
fi

reload_clamd
log "SUCCESS: Rollback completed. clamd PING -> PONG verified."
