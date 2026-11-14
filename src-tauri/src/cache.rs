use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::scanner::ScanStatus;

#[derive(Serialize)]
pub struct CacheStats {
    pub entries: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedVerdict {
    pub status: ScanStatus,
    pub threat_name: Option<String>,
}

pub fn default_path() -> Result<PathBuf> {
    #[cfg(windows)]
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .context("LOCALAPPDATA is unavailable")?;
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local").join("share"))
        })
        .context("user data directory is unavailable")?;
    Ok(base.join("Zonendpoint").join("cache.db"))
}

fn format_corrupt_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let vn_secs = now.saturating_add(7 * 3600);
    let days = (vn_secs / 86400) as i64;
    let rem = vn_secs % 86400;
    let hours = rem / 3600;
    let minutes = (rem % 3600) / 60;
    let seconds = rem % 60;

    let z = days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}{m:02}{d:02}-{hours:02}{minutes:02}{seconds:02}")
}

fn try_open(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("create cache directory")?;
    }
    let mut db = Connection::open(path).context("open scan cache")?;
    db.busy_timeout(std::time::Duration::from_secs(2))?;
    db.pragma_update(None, "journal_mode", "WAL")?;
    migrate(&mut db)?;
    Ok(db)
}

fn open(path: &Path) -> Result<Connection> {
    match try_open(path) {
        Ok(db) => Ok(db),
        Err(orig_err) => {
            if path.exists() {
                let ts = format_corrupt_timestamp();
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("cache.db");
                let mut corrupt_path = path.with_file_name(format!("{file_name}.corrupt-{ts}"));
                let mut counter = 1;
                while corrupt_path.exists() {
                    corrupt_path =
                        path.with_file_name(format!("{file_name}.corrupt-{ts}-{counter}"));
                    counter += 1;
                }
                if let Err(e) = std::fs::rename(path, &corrupt_path) {
                    bail!(
                        "failed to rename corrupt cache {}: {e}; original error: {orig_err}",
                        path.display()
                    );
                }
                let _ = std::fs::remove_file(path.with_file_name(format!("{file_name}-wal")));
                let _ = std::fs::remove_file(path.with_file_name(format!("{file_name}-shm")));

                try_open(path).with_context(|| {
                    format!(
                        "failed to initialize fresh cache db after corruption; original error: {orig_err}"
                    )
                })
            } else {
                Err(orig_err)
            }
        }
    }
}

fn migrate(db: &mut Connection) -> Result<()> {
    let tx = db.transaction()?;
    let version: i64 = tx.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version > 2 {
        bail!("cache schema version {version} is newer than supported version 2");
    }
    if version == 0 {
        // Old metadata-only rows have no trustworthy engine identity or content key.
        tx.execute_batch(
            "DROP TABLE IF EXISTS scan_cache;
             CREATE TABLE scan_cache (
               sha256 BLOB NOT NULL CHECK(length(sha256) = 32),
               engine_version TEXT NOT NULL,
               policy_version TEXT NOT NULL,
               status TEXT NOT NULL CHECK(status IN ('CLEAN', 'INFECTED')),
               threat_name TEXT,
               scanned_at INTEGER NOT NULL,
               PRIMARY KEY(sha256, engine_version, policy_version)
             );",
        )?;
        tx.pragma_update(None, "user_version", 2)?;
    } else if version == 1 {
        tx.execute_batch("ALTER TABLE scan_cache ADD COLUMN threat_name TEXT;")?;
        tx.pragma_update(None, "user_version", 2)?;
    }
    tx.commit()?;
    Ok(())
}

impl ScanStatus {
    fn cache_value(self) -> Result<&'static str> {
        match self {
            Self::Clean => Ok("CLEAN"),
            Self::Infected => Ok("INFECTED"),
            _ => bail!("only completed engine verdicts may be cached"),
        }
    }

    fn from_cache(value: &str) -> Result<Self> {
        match value {
            "CLEAN" => Ok(Self::Clean),
            "INFECTED" => Ok(Self::Infected),
            _ => bail!("invalid cached scan status"),
        }
    }
}

pub fn stats(path: &Path) -> Result<CacheStats> {
    let db = open(path)?;
    Ok(CacheStats {
        entries: db.query_row("SELECT COUNT(*) FROM scan_cache", [], |row| row.get(0))?,
    })
}

pub fn get(
    path: &Path,
    sha256: &[u8; 32],
    engine_version: &str,
    policy_version: &str,
    now: i64,
    ttl_seconds: i64,
) -> Result<Option<CachedVerdict>> {
    let db = open(path)?;
    let cutoff = now.saturating_sub(ttl_seconds.max(0));
    let row: Option<(String, Option<String>)> = db
        .query_row(
            "SELECT status, threat_name FROM scan_cache
             WHERE sha256=?1 AND engine_version=?2 AND policy_version=?3 AND scanned_at BETWEEN ?4 AND ?5",
            params![sha256.as_slice(), engine_version, policy_version, cutoff, now],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    match row {
        Some((status_str, threat_name)) => {
            let status = ScanStatus::from_cache(&status_str)?;
            if status == ScanStatus::Infected {
                let valid_threat = threat_name
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(ToString::to_string);
                match valid_threat {
                    Some(name) => Ok(Some(CachedVerdict {
                        status,
                        threat_name: Some(name),
                    })),
                    None => Ok(None),
                }
            } else {
                Ok(Some(CachedVerdict {
                    status,
                    threat_name: None,
                }))
            }
        }
        None => Ok(None),
    }
}

pub fn put(
    path: &Path,
    sha256: &[u8; 32],
    engine_version: &str,
    policy_version: &str,
    status: ScanStatus,
    threat_name: Option<&str>,
    scanned_at: i64,
) -> Result<()> {
    let db = open(path)?;
    let clean_threat_name = if status == ScanStatus::Infected {
        let name = threat_name.map(str::trim).filter(|s| !s.is_empty());
        if name.is_none() {
            bail!("infected verdict requires a threat name to be cached");
        }
        name
    } else {
        None
    };

    db.execute(
        "INSERT INTO scan_cache(sha256, engine_version, policy_version, status, threat_name, scanned_at)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(sha256, engine_version, policy_version)
         DO UPDATE SET status=excluded.status, threat_name=excluded.threat_name, scanned_at=excluded.scanned_at",
        params![
            sha256.as_slice(),
            engine_version,
            policy_version,
            status.cache_value()?,
            clean_threat_name,
            scanned_at
        ],
    )?;
    Ok(())
}

pub fn cleanup_expired(path: &Path, now: i64, ttl_seconds: i64) -> Result<usize> {
    if !path.exists() {
        return Ok(0);
    }
    let db = open(path)?;
    let cutoff = now.saturating_sub(ttl_seconds.max(0));
    let deleted = db.execute(
        "DELETE FROM scan_cache WHERE scanned_at < ?1",
        params![cutoff],
    )?;
    let _ = db.pragma_update(None, "wal_checkpoint", "TRUNCATE");
    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::ScanStatus;
    use std::{
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static TEST_PATH_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_path(label: &str) -> PathBuf {
        let count = TEST_PATH_COUNTER.fetch_add(1, Ordering::Relaxed);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "zonendpoint-{label}-{}-{count}-{nonce}.db",
            std::process::id()
        ))
    }

    #[test]
    fn cache_hit_requires_content_engine_policy_and_fresh_ttl() {
        let path = test_path("content");
        let hash1 = [1; 32];
        let hash2 = [2; 32];
        put(
            &path,
            &hash1,
            "clamd-1/sig-10",
            "policy-1",
            ScanStatus::Clean,
            None,
            100,
        )
        .unwrap();
        assert_eq!(
            get(&path, &hash1, "clamd-1/sig-10", "policy-1", 110, 20)
                .unwrap()
                .unwrap()
                .status,
            ScanStatus::Clean
        );
        assert!(
            get(&path, &hash2, "clamd-1/sig-10", "policy-1", 110, 20)
                .unwrap()
                .is_none()
        );
        assert!(
            get(&path, &hash1, "clamd-2/sig-10", "policy-1", 110, 20)
                .unwrap()
                .is_none()
        );
        assert!(
            get(&path, &hash1, "clamd-1/sig-10", "policy-2", 110, 20)
                .unwrap()
                .is_none()
        );
        assert!(
            get(&path, &hash1, "clamd-1/sig-10", "policy-1", 121, 20)
                .unwrap()
                .is_none()
        );
        assert!(
            get(&path, &hash1, "clamd-1/sig-10", "policy-1", 90, 20)
                .unwrap()
                .is_none()
        );
        assert!(
            get(&path, &hash1, "clamd-1/sig-10", "policy-1", 90, 20)
                .unwrap()
                .is_none()
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn legacy_cache_migration_discards_unverifiable_metadata_rows() {
        let path = test_path("migration");
        let db = Connection::open(&path).unwrap();
        db.execute_batch("CREATE TABLE scan_cache(path TEXT NOT NULL, size INTEGER NOT NULL, mtime INTEGER NOT NULL, sha256 TEXT NOT NULL, status TEXT NOT NULL); INSERT INTO scan_cache VALUES('x', 1, 1, '00', 'clean');").unwrap();
        drop(db);
        assert_eq!(stats(&path).unwrap().entries, 0);
        let db = open(&path).unwrap();
        let version: i64 = db
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 2);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn failed_migration_keeps_legacy_table_transactionally() {
        let path = test_path("migration-failure");
        let mut db = Connection::open(&path).unwrap();
        db.execute_batch("CREATE TABLE scan_cache(path TEXT NOT NULL, size INTEGER NOT NULL, mtime INTEGER NOT NULL, sha256 TEXT NOT NULL, status TEXT NOT NULL); INSERT INTO scan_cache VALUES('keep', 1, 1, '00', 'clean'); PRAGMA query_only=ON;").unwrap();
        assert!(migrate(&mut db).is_err());
        assert_eq!(
            db.query_row("SELECT path FROM scan_cache", [], |row| row
                .get::<_, String>(0))
                .unwrap(),
            "keep"
        );
        drop(db);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn cache_rejects_sha256_values_not_32_bytes() {
        let path = test_path("hash-length");
        let db = open(&path).unwrap();
        assert!(
            db.execute(
                "INSERT INTO scan_cache VALUES(?1, 'engine', 'policy', 'CLEAN', NULL, 1)",
                [vec![0u8; 31]]
            )
            .is_err()
        );
        drop(db);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn cache_hit_preserves_threat_name_for_infected() {
        let path = test_path("infected-name");
        let hash = [0x42; 32];
        let threat = "YARA.ZonQA_Benign_Marker.UNOFFICIAL";

        put(
            &path,
            &hash,
            "ClamAV 1.5.4/28176",
            "policy-v1",
            ScanStatus::Infected,
            Some(threat),
            1000,
        )
        .unwrap();

        let verdict = get(&path, &hash, "ClamAV 1.5.4/28176", "policy-v1", 1050, 3600)
            .unwrap()
            .expect("cache hit expected");

        assert_eq!(verdict.status, ScanStatus::Infected);
        assert_eq!(verdict.threat_name.as_deref(), Some(threat));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn cache_hit_clean_preserves_clean_verdict_without_threat_name() {
        let path = test_path("clean-no-threat");
        let hash = [0x55; 32];

        put(
            &path,
            &hash,
            "ClamAV 1.5.4/28176",
            "policy-v1",
            ScanStatus::Clean,
            None,
            1000,
        )
        .unwrap();

        let verdict = get(&path, &hash, "ClamAV 1.5.4/28176", "policy-v1", 1050, 3600)
            .unwrap()
            .expect("cache hit expected");

        assert_eq!(verdict.status, ScanStatus::Clean);
        assert_eq!(verdict.threat_name, None);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn legacy_v1_database_migrates_safely_and_treats_unnamed_infected_as_miss() {
        let path = test_path("legacy-v1-migration");

        // Manually construct a valid schema v1 database
        let db = Connection::open(&path).unwrap();
        db.execute_batch(
            "CREATE TABLE scan_cache (
               sha256 BLOB NOT NULL CHECK(length(sha256) = 32),
               engine_version TEXT NOT NULL,
               policy_version TEXT NOT NULL,
               status TEXT NOT NULL CHECK(status IN ('CLEAN', 'INFECTED')),
               scanned_at INTEGER NOT NULL,
               PRIMARY KEY(sha256, engine_version, policy_version)
             );
             PRAGMA user_version = 1;",
        )
        .unwrap();

        let hash_clean = [0x11; 32];
        let hash_infected_legacy = [0x22; 32];

        db.execute(
            "INSERT INTO scan_cache(sha256, engine_version, policy_version, status, scanned_at)
             VALUES (?1, 'ClamAV 1.5.4/28176', 'policy-v1', 'CLEAN', 1000)",
            params![hash_clean.as_slice()],
        )
        .unwrap();

        db.execute(
            "INSERT INTO scan_cache(sha256, engine_version, policy_version, status, scanned_at)
             VALUES (?1, 'ClamAV 1.5.4/28176', 'policy-v1', 'INFECTED', 1000)",
            params![hash_infected_legacy.as_slice()],
        )
        .unwrap();
        drop(db);

        // Open via open(&path) -> triggers migrate() from version 1 to 2
        let db = open(&path).unwrap();
        let version: i64 = db
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 2, "user_version must be bumped to 2");

        // Check entry count (both rows preserved)
        let count: i64 = db
            .query_row("SELECT COUNT(*) FROM scan_cache", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2, "Both rows preserved in migration");
        drop(db);

        // 1. CLEAN row should hit cache normally
        let clean_verdict = get(
            &path,
            &hash_clean,
            "ClamAV 1.5.4/28176",
            "policy-v1",
            1050,
            3600,
        )
        .unwrap()
        .expect("CLEAN entry should be a cache hit");
        assert_eq!(clean_verdict.status, ScanStatus::Clean);
        assert_eq!(clean_verdict.threat_name, None);

        // 2. Legacy INFECTED row with NULL threat_name MUST be treated as a cache miss!
        let infected_verdict = get(
            &path,
            &hash_infected_legacy,
            "ClamAV 1.5.4/28176",
            "policy-v1",
            1050,
            3600,
        )
        .unwrap();
        assert!(
            infected_verdict.is_none(),
            "Legacy infected entry without threat name MUST be treated as cache miss to trigger rescan"
        );

        // 3. Now simulate a re-scan updating the entry with a threat name
        put(
            &path,
            &hash_infected_legacy,
            "ClamAV 1.5.4/28176",
            "policy-v1",
            ScanStatus::Infected,
            Some("Win.Test.EICAR_Handoff"),
            1100,
        )
        .unwrap();

        // 4. Query again: now it should hit cache with the threat name
        let rescanned = get(
            &path,
            &hash_infected_legacy,
            "ClamAV 1.5.4/28176",
            "policy-v1",
            1120,
            3600,
        )
        .unwrap()
        .expect("Rescanned infected entry should now hit cache");
        assert_eq!(rescanned.status, ScanStatus::Infected);
        assert_eq!(
            rescanned.threat_name.as_deref(),
            Some("Win.Test.EICAR_Handoff")
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn error_and_unknown_never_cached_and_never_become_clean() {
        let path = test_path("error-never-cached");
        let hash = [0x99; 32];

        // Ensure put rejects non-terminal verdicts
        assert!(
            put(
                &path,
                &hash,
                "ClamAV 1.5.4",
                "policy-1",
                ScanStatus::Error,
                None,
                100,
            )
            .is_err()
        );
        assert!(
            put(
                &path,
                &hash,
                "ClamAV 1.5.4",
                "policy-1",
                ScanStatus::Unknown,
                None,
                100,
            )
            .is_err()
        );
        assert!(
            put(
                &path,
                &hash,
                "ClamAV 1.5.4",
                "policy-1",
                ScanStatus::Cancelled,
                None,
                100,
            )
            .is_err()
        );
        assert!(
            put(
                &path,
                &hash,
                "ClamAV 1.5.4",
                "policy-1",
                ScanStatus::Skipped,
                None,
                100,
            )
            .is_err()
        );

        // Ensure database check constraint rejects invalid status strings
        let db = open(&path).unwrap();
        assert!(
            db.execute(
                "INSERT INTO scan_cache(sha256, engine_version, policy_version, status, threat_name, scanned_at)
                 VALUES (?1, 'engine', 'policy', 'ERROR', NULL, 100)",
                params![hash.as_slice()],
            )
            .is_err()
        );
        assert!(
            db.execute(
                "INSERT INTO scan_cache(sha256, engine_version, policy_version, status, threat_name, scanned_at)
                 VALUES (?1, 'engine', 'policy', 'UNKNOWN', NULL, 100)",
                params![hash.as_slice()],
            )
            .is_err()
        );

        // Ensure from_cache rejects anything other than CLEAN or INFECTED
        assert!(ScanStatus::from_cache("ERROR").is_err());
        assert!(ScanStatus::from_cache("UNKNOWN").is_err());
        assert!(ScanStatus::from_cache("CANCELLED").is_err());
        assert!(ScanStatus::from_cache("SKIPPED").is_err());
        assert!(ScanStatus::from_cache("MALFORMED").is_err());

        drop(db);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn infected_without_threat_name_fails_to_cache() {
        let path = test_path("infected-empty-threat");
        let hash = [0x77; 32];

        // None threat name must fail
        assert!(
            put(
                &path,
                &hash,
                "ClamAV 1.5.4",
                "policy-1",
                ScanStatus::Infected,
                None,
                100,
            )
            .is_err()
        );

        // Blank/whitespace threat name must fail
        assert!(
            put(
                &path,
                &hash,
                "ClamAV 1.5.4",
                "policy-1",
                ScanStatus::Infected,
                Some("   "),
                100,
            )
            .is_err()
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn corrupt_cache_db_auto_recovers() {
        let path = test_path("corrupt-recovery");
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&path, b"not a valid sqlite database header").unwrap();
        assert!(path.exists());

        let hash = [0x88; 32];
        let result = put(
            &path,
            &hash,
            "engine-v1",
            "policy-1",
            ScanStatus::Clean,
            None,
            100,
        );
        assert!(result.is_ok(), "corrupt db must automatically recover: {result:?}");

        // Cache must work properly after recovery
        let cached = get(&path, &hash, "engine-v1", "policy-1", 110, 60).unwrap();
        assert_eq!(cached.unwrap().status, ScanStatus::Clean);

        // Verify a corrupt backup file was created
        let parent = path.parent().unwrap();
        let file_prefix = path.file_name().unwrap().to_string_lossy();
        let corrupt_found = std::fs::read_dir(parent).unwrap().any(|entry| {
            let name = entry.unwrap().file_name().to_string_lossy().into_owned();
            name.starts_with(&format!("{file_prefix}.corrupt-"))
        });
        assert!(corrupt_found, "corrupt backup file must exist");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn incompatible_schema_version_auto_recovers() {
        let path = test_path("incompatible-schema");
        {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch("PRAGMA user_version = 99;").unwrap();
        }
        assert!(path.exists());

        let hash = [0x99; 32];
        let result = put(
            &path,
            &hash,
            "engine-v1",
            "policy-1",
            ScanStatus::Clean,
            None,
            100,
        );
        assert!(result.is_ok(), "schema version 99 must recover: {result:?}");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn cleanup_expired_removes_old_entries() {
        let path = test_path("cleanup-expired");
        let hash1 = [0x11; 32];
        let hash2 = [0x22; 32];

        put(
            &path,
            &hash1,
            "engine-v1",
            "policy-1",
            ScanStatus::Clean,
            None,
            100,
        )
        .unwrap();

        put(
            &path,
            &hash2,
            "engine-v1",
            "policy-1",
            ScanStatus::Clean,
            None,
            500,
        )
        .unwrap();

        // Cutoff = 600 - 200 = 400. Entry at 100 should be deleted, entry at 500 kept.
        let deleted = cleanup_expired(&path, 600, 200).unwrap();
        assert_eq!(deleted, 1);

        assert!(
            get(&path, &hash1, "engine-v1", "policy-1", 600, 600)
                .unwrap()
                .is_none()
        );
        assert!(
            get(&path, &hash2, "engine-v1", "policy-1", 600, 600)
                .unwrap()
                .is_some()
        );

        let _ = std::fs::remove_file(&path);
    }
}
