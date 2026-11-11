# J-02 ThreatFox — Jacky to Qeen handoff

## Code snapshot
- Branch: `jacky/threatfox`
- Base: `a9152229b4024da2b65a934dd4fa9fc81c612e55` (V-38 / origin main snapshot supplied by Loki)
- Code Snapshot SHA: recorded after commit

## Change summary
- Shared abuse.ch limiter used by both MalwareBazaar and ThreatFox: total 3 per sliding hour, 400 per 24h, atomic state writes, lock around check/write, source-tagged history, migration from legacy MalwareBazaar timestamp list, fail closed on malformed history. When both feeds are waiting, operators pass `--threatfox-pending` / `--malwarebazaar-pending` to enforce the alternating turn.
- MalwareBazaar behavior otherwise retained; its mock suite should be rerun.
- ThreatFox manual `check`, `fetch-full`, `export-hsb`, `deploy-hsb`, plus dedicated rollback; transactional SQLite IOC and SHA-256 storage; CSV fields resolved by header aliases; key path redaction; common guarded clamd deployment lifecycle.
- Terms, known feed behavior, command instructions, and uncertainty are in `docs/ops/threatfox.md`.

## Verification
- `python scripts/feeds/test_abusech_rate_limit.py`: NOT RUN (Python runtime unavailable in this Windows environment).
- `python scripts/feeds/test_zonendpoint_threatfox.py`: NOT RUN (same blocker).
- `python scripts/feeds/test_zonendpoint_malwarebazaar.py`: NOT RUN (same blocker).
- `git diff --check`: PASS.
- No real API requests, key use, SSH, lab activity, live clamd reload, or Qeen acceptance occurred.

## Required QA / operator follow-up
1. Run all three test suites on Linux and fix any regressions before acceptance.
2. Verify current authenticated ThreatFox CSV headers/values against the parser; docs remain login-gated, so the mock schema is not proof of real export compatibility.
3. Check limiter migration using a copy of the actual old MalwareBazaar state; verify alternating invocation flags when both feeds are pending.
4. On lab, run manual check/fetch-full; inspect DB count/IOC types and generated `.hsb`; `deploy-hsb` only after temporary validation and operator review. Preserve clamd 127.0.0.1:3310, do not change config, and install no timer.
5. Rollback: `sudo /usr/local/bin/zonendpoint-threatfox-rollback.sh` (restore) or `--purge`.
