#!/bin/bash
# Zonendpoint YARA-Forge Core Rollback Script (Task V-25)
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

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1"
}

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
    local hold="${HOLD_FILE:-/opt/zonendpoint/HOLD-yara}"
    mkdir -p "$(dirname "$hold")"
    date -u '+%Y-%m-%dT%H:%M:%SZ' > "$hold"
    log "Created hold file $hold (daily updater will skip runs until removed)."
}

# L-6: Acquire shared deploy lock to prevent concurrent reload/deploy
acquire_lock

if [ "${1:-}" = "--purge" ]; then
    log "Purging YARA-Forge Core ruleset from active clamd..."
    rm -f "$ACTIVE_FILE"
    clamdscan --reload
    sleep 8
    RESP=$(echo "PING" | nc -w 3 127.0.0.1 3310 || true)
    if [ "$RESP" != "PONG" ]; then
        log "ERROR: clamd PING check failed after purge!"
        exit 2
    fi
    set_hold
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

log "Triggering zero-downtime clamd reload..."
clamdscan --reload
sleep 8

RESP=$(echo "PING" | nc -w 3 127.0.0.1 3310 || true)
if [ "$RESP" != "PONG" ]; then
    log "ERROR: clamd PING verification failed after rollback!"
    exit 2
fi

set_hold
log "SUCCESS: Rollback completed. clamd PING -> PONG verified."
