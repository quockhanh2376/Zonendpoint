use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::scanner::ScanStatus;

#[derive(Serialize)]
pub struct CacheStats {
    pub entries: i64,
}

pub struct CachedVerdict {
    pub status: ScanStatus,
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

fn open(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("create cache directory")?;
    }
    let mut db = Connection::open(path).context("open scan cache")?;
    db.busy_timeout(std::time::Duration::from_secs(2))?;
    db.pragma_update(None, "journal_mode", "WAL")?;
    migrate(&mut db)?;
    Ok(db)
}

fn migrate(db: &mut Connection) -> Result<()> {
    let tx = db.transaction()?;
    let version: i64 = tx.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version > 1 {
        bail!("cache schema version {version} is newer than supported version 1");
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
               scanned_at INTEGER NOT NULL,
               PRIMARY KEY(sha256, engine_version, policy_version)
             );",
        )?;
        tx.pragma_update(None, "user_version", 1)?;
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
    let value: Option<String> = db
        .query_row(
            "SELECT status FROM scan_cache
         WHERE sha256=?1 AND engine_version=?2 AND policy_version=?3 AND scanned_at BETWEEN ?4 AND ?5",
            params![sha256.as_slice(), engine_version, policy_version, cutoff, now],
            |row| row.get(0),
        )
        .optional()?;
    value
        .map(|status| {
            Ok(CachedVerdict {
                status: ScanStatus::from_cache(&status)?,
            })
        })
        .transpose()
}

pub fn put(
    path: &Path,
    sha256: &[u8; 32],
    engine_version: &str,
    policy_version: &str,
    status: ScanStatus,
    scanned_at: i64,
) -> Result<()> {
    let db = open(path)?;
    db.execute(
        "INSERT INTO scan_cache(sha256, engine_version, policy_version, status, scanned_at)
         VALUES(?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(sha256, engine_version, policy_version)
         DO UPDATE SET status=excluded.status, scanned_at=excluded.scanned_at",
        params![
            sha256.as_slice(),
            engine_version,
            policy_version,
            status.cache_value()?,
            scanned_at
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::ScanStatus;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_path(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "zonendpoint-{label}-{}-{nonce}.db",
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
        assert_eq!(version, 1);
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
                "INSERT INTO scan_cache VALUES(?1, 'engine', 'policy', 'CLEAN', 1)",
                [vec![0u8; 31]]
            )
            .is_err()
        );
        drop(db);
        let _ = std::fs::remove_file(path);
    }
}
