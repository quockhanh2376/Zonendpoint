"""Shared, disk-persisted abuse.ch request budget for MalwareBazaar and ThreatFox."""
import datetime
import json
import logging
import os
import pathlib
import tempfile
import time

MAX_PER_HOUR = 3
MAX_PER_DAY = 400
HOUR = 3600
DAY = 86400
DEFAULT_STATE = "/opt/zonendpoint/abusech/rate_limit.json"
LEGACY_STATE = "/opt/zonendpoint/malwarebazaar/rate_limit.json"
ORDER = ("malwarebazaar", "threatfox")


class RateLimitError(Exception):
    pass


class RateLimiter:
    def __init__(self, state_file=DEFAULT_STATE, legacy_file=LEGACY_STATE, other_waiting=False):
        self.state_file = pathlib.Path(state_file)
        self.legacy_file = pathlib.Path(legacy_file)
        self.other_waiting = other_waiting

    def _read(self, path):
        if not path.exists():
            return []
        raw = json.loads(path.read_text(encoding="utf-8"))
        if not isinstance(raw, dict) or "requests" not in raw:
            raise ValueError(f"invalid abuse.ch rate-limit history in {path}; refusing requests")
        rows = raw["requests"]
        if not isinstance(rows, list):
            raise ValueError(f"invalid abuse.ch rate-limit history in {path}; refusing requests")
        result = []
        for row in rows:
            if isinstance(row, (int, float)):
                result.append({"timestamp": float(row), "source": "malwarebazaar"})
            elif isinstance(row, dict) and isinstance(row.get("timestamp"), (int, float)):
                result.append({"timestamp": float(row["timestamp"]), "source": row.get("source", "malwarebazaar")})
            else:
                raise ValueError(f"invalid abuse.ch rate-limit record in {path}; refusing requests")
        return result

    def _records(self):
        if self.state_file.exists():
            return self._read(self.state_file)
        if self.legacy_file.exists():
            return self._read(self.legacy_file)
        return []

    def get_status(self, now=None):
        now = time.time() if now is None else now
        rows = [r for r in self._records() if 0 <= now - r["timestamp"] < DAY]
        hourly = [r for r in rows if now - r["timestamp"] < HOUR]
        return {
            "hourly_used": len(hourly),
            "hourly_max": MAX_PER_HOUR,
            "daily_used": len(rows),
            "daily_max": MAX_PER_DAY,
            "hourly_remaining": max(0, MAX_PER_HOUR - len(hourly)),
            "daily_remaining": max(0, MAX_PER_DAY - len(rows)),
        }

    def check_and_record(self, source="malwarebazaar", now=None, other_waiting=None):
        if source not in ORDER:
            raise ValueError("unknown abuse.ch feed source")
        # Linux server lock keeps simultaneous manual invocations inside the same budget.
        lock_path = self.state_file.with_suffix(self.state_file.suffix + ".lock")
        lock_path.parent.mkdir(parents=True, exist_ok=True)
        with open(lock_path, "a+") as lock:
            try:
                import fcntl
                fcntl.flock(lock, fcntl.LOCK_EX)
            except ImportError:
                pass
            return self._check_and_record_locked(source, now, self.other_waiting if other_waiting is None else other_waiting)

    def _check_and_record_locked(self, source, now, other_waiting):
        now = time.time() if now is None else now
        rows = [r for r in self._records() if 0 <= now - r["timestamp"] < DAY]
        hourly = [r for r in rows if now - r["timestamp"] < HOUR]
        if len(hourly) >= MAX_PER_HOUR:
            raise RateLimitError(f"Shared abuse.ch rate limit exceeded: {len(hourly)}/3 requests in last 60 minutes; stopping without retry.")
        if len(rows) >= MAX_PER_DAY:
            raise RateLimitError(f"Daily rate limit exceeded: {len(rows)}/400 requests in 24h window across abuse.ch feeds; stopping without retry.")
        if other_waiting:
            if hourly:
                last = max(hourly, key=lambda r: r["timestamp"])
                waiting = ORDER[1] if last["source"] == ORDER[0] else ORDER[0]
            else:
                waiting = ORDER[0]
            if source != waiting:
                raise RateLimitError(f"Shared abuse.ch turn belongs to {waiting}; stopping without retry.")
        rows.append({"timestamp": now, "source": source})
        self.state_file.parent.mkdir(parents=True, exist_ok=True)
        payload = {"requests": rows, "updated_at": datetime.datetime.now(datetime.timezone.utc).isoformat()}
        fd, temp = tempfile.mkstemp(prefix=self.state_file.name + ".", dir=self.state_file.parent)
        try:
            with os.fdopen(fd, "w", encoding="utf-8") as f:
                json.dump(payload, f)
                f.flush()
                os.fsync(f.fileno())
            os.replace(temp, self.state_file)
        finally:
            if os.path.exists(temp):
                os.unlink(temp)
        logging.getLogger("abusech_rate_limit").info("Shared abuse.ch request recorded (%s; %d/hour, %d/day)", source, len(hourly)+1, len(rows))
