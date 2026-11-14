use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

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
    if version > 3 {
        bail!("cache schema version {version} is newer than supported version 3");
    }
    if version < 3 {
        // Schema v3 binds cache entries to signature generation
        tx.execute_batch(
            "DROP TABLE IF EXISTS scan_cache;
             CREATE TABLE scan_cache (
               sha256 BLOB NOT NULL CHECK(length(sha256) = 32),
               engine_version TEXT NOT NULL,
               sig_generation TEXT NOT NULL,
               policy_version TEXT NOT NULL,
               status TEXT NOT NULL CHECK(status IN ('CLEAN', 'INFECTED')),
               threat_name TEXT,
               scanned_at INTEGER NOT NULL,
               PRIMARY KEY(sha256, engine_version, sig_generation, policy_version)
             );",
        )?;
        tx.pragma_update(None, "user_version", 3)?;
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

pub const BATCH_MAX_ROWS: usize = 200;
pub const BATCH_MAX_DURATION: Duration = Duration::from_secs(1);

#[derive(Clone, Debug)]
pub struct PendingCachePut {
    pub sha256: [u8; 32],
    pub engine_version: String,
    pub sig_generation: String,
    pub policy_version: String,
    pub status: ScanStatus,
    pub threat_name: Option<String>,
    pub scanned_at: i64,
}

pub struct CacheSessionInner {
    conn: Connection,
    pending: Vec<PendingCachePut>,
    last_flush: Instant,
}

impl CacheSessionInner {
    fn flush(&mut self) -> Result<()> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO scan_cache(sha256, engine_version, sig_generation, policy_version, status, threat_name, scanned_at)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(sha256, engine_version, sig_generation, policy_version)
                 DO UPDATE SET status=excluded.status, threat_name=excluded.threat_name, scanned_at=excluded.scanned_at",
            )?;
            for item in self.pending.drain(..) {
                stmt.execute(params![
                    item.sha256.as_slice(),
                    item.engine_version,
                    item.sig_generation,
                    item.policy_version,
                    item.status.cache_value()?,
                    item.threat_name,
                    item.scanned_at,
                ])?;
            }
        }
        tx.commit()?;
        self.last_flush = Instant::now();
        Ok(())
    }
}

impl Drop for CacheSessionInner {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

#[derive(Clone)]
pub struct CacheSession {
    inner: Arc<Mutex<CacheSessionInner>>,
}

impl CacheSession {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = open(path)?;
        Ok(Self {
            inner: Arc::new(Mutex::new(CacheSessionInner {
                conn,
                pending: Vec::with_capacity(BATCH_MAX_ROWS),
                last_flush: Instant::now(),
            })),
        })
    }

    pub fn get_sync(
        &self,
        sha256: &[u8; 32],
        engine_version: &str,
        sig_generation: &str,
        policy_version: &str,
        now: i64,
        ttl_seconds: i64,
    ) -> Result<Option<CachedVerdict>> {
        let inner = self
            .inner
            .lock()
            .map_err(|e| anyhow::anyhow!("cache lock poisoned: {e}"))?;
        let cutoff = now.saturating_sub(ttl_seconds.max(0));

        // 1. Check pending in-memory batch first
        for item in inner.pending.iter().rev() {
            if &item.sha256 == sha256
                && item.engine_version == engine_version
                && item.sig_generation == sig_generation
                && item.policy_version == policy_version
                && item.scanned_at >= cutoff
                && item.scanned_at <= now
            {
                if item.status == ScanStatus::Infected {
                    let valid_threat = item
                        .threat_name
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(ToString::to_string);
                    return Ok(valid_threat.map(|name| CachedVerdict {
                        status: item.status,
                        threat_name: Some(name),
                    }));
                } else {
                    return Ok(Some(CachedVerdict {
                        status: item.status,
                        threat_name: None,
                    }));
                }
            }
        }

        // 2. Query SQLite connection with prepare_cached
        let mut stmt = inner.conn.prepare_cached(
            "SELECT status, threat_name FROM scan_cache
             WHERE sha256=?1 AND engine_version=?2 AND sig_generation=?3 AND policy_version=?4 AND scanned_at BETWEEN ?5 AND ?6",
        )?;
        let row: Option<(String, Option<String>)> = stmt
            .query_row(
                params![
                    sha256.as_slice(),
                    engine_version,
                    sig_generation,
                    policy_version,
                    cutoff,
                    now
                ],
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

    #[allow(clippy::too_many_arguments)]
    pub fn put_sync(
        &self,
        sha256: [u8; 32],
        engine_version: &str,
        sig_generation: &str,
        policy_version: &str,
        status: ScanStatus,
        threat_name: Option<&str>,
        scanned_at: i64,
    ) -> Result<()> {
        let clean_threat_name = if status == ScanStatus::Infected {
            let name = threat_name.map(str::trim).filter(|s| !s.is_empty());
            if name.is_none() {
                bail!("infected verdict requires a threat name to be cached");
            }
            name.map(ToString::to_string)
        } else {
            None
        };

        let mut inner = self
            .inner
            .lock()
            .map_err(|e| anyhow::anyhow!("cache lock poisoned: {e}"))?;

        inner.pending.push(PendingCachePut {
            sha256,
            engine_version: engine_version.to_owned(),
            sig_generation: sig_generation.to_owned(),
            policy_version: policy_version.to_owned(),
            status,
            threat_name: clean_threat_name,
            scanned_at,
        });

        if inner.pending.len() >= BATCH_MAX_ROWS || inner.last_flush.elapsed() >= BATCH_MAX_DURATION
        {
            inner.flush()?;
        }
        Ok(())
    }

    pub fn flush_sync(&self) -> Result<()> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|e| anyhow::anyhow!("cache lock poisoned: {e}"))?;
        inner.flush()
    }

    pub async fn get(
        &self,
        sha256: [u8; 32],
        engine_version: String,
        sig_generation: String,
        policy_version: String,
        now: i64,
        ttl_seconds: i64,
    ) -> Result<Option<CachedVerdict>> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || {
            this.get_sync(
                &sha256,
                &engine_version,
                &sig_generation,
                &policy_version,
                now,
                ttl_seconds,
            )
        })
        .await
        .map_err(|e| anyhow::anyhow!("spawn_blocking join error: {e}"))?
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn put(
        &self,
        sha256: [u8; 32],
        engine_version: String,
        sig_generation: String,
        policy_version: String,
        status: ScanStatus,
        threat_name: Option<String>,
        scanned_at: i64,
    ) -> Result<()> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || {
            this.put_sync(
                sha256,
                &engine_version,
                &sig_generation,
                &policy_version,
                status,
                threat_name.as_deref(),
                scanned_at,
            )
        })
        .await
        .map_err(|e| anyhow::anyhow!("spawn_blocking join error: {e}"))?
    }

    pub async fn flush(&self) -> Result<()> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.flush_sync())
            .await
            .map_err(|e| anyhow::anyhow!("spawn_blocking join error: {e}"))?
    }
}

pub fn get(
    path: &Path,
    sha256: &[u8; 32],
    engine_version: &str,
    sig_generation: &str,
    policy_version: &str,
    now: i64,
    ttl_seconds: i64,
) -> Result<Option<CachedVerdict>> {
    let session = CacheSession::open(path)?;
    session.get_sync(
        sha256,
        engine_version,
        sig_generation,
        policy_version,
        now,
        ttl_seconds,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn put(
    path: &Path,
    sha256: &[u8; 32],
    engine_version: &str,
    sig_generation: &str,
    policy_version: &str,
    status: ScanStatus,
    threat_name: Option<&str>,
    scanned_at: i64,
) -> Result<()> {
    let session = CacheSession::open(path)?;
    session.put_sync(
        *sha256,
        engine_version,
        sig_generation,
        policy_version,
        status,
        threat_name,
        scanned_at,
    )?;
    session.flush_sync()
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
    const TEST_SIG_GEN: &str = "sig:8616464:2026-11-14T11:41:49+07:00";

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
            TEST_SIG_GEN,
            "policy-1",
            ScanStatus::Clean,
            None,
            100,
        )
        .unwrap();
        assert_eq!(
            get(
                &path,
                &hash1,
                "clamd-1/sig-10",
                TEST_SIG_GEN,
                "policy-1",
                110,
                20
            )
            .unwrap()
            .unwrap()
            .status,
            ScanStatus::Clean
        );
        assert!(
            get(
                &path,
                &hash2,
                "clamd-1/sig-10",
                TEST_SIG_GEN,
                "policy-1",
                110,
                20
            )
            .unwrap()
            .is_none()
        );
        assert!(
            get(
                &path,
                &hash1,
                "clamd-2/sig-10",
                TEST_SIG_GEN,
                "policy-1",
                110,
                20
            )
            .unwrap()
            .is_none()
        );
        assert!(
            get(
                &path,
                &hash1,
                "clamd-1/sig-10",
                TEST_SIG_GEN,
                "policy-2",
                110,
                20
            )
            .unwrap()
            .is_none()
        );
        assert!(
            get(
                &path,
                &hash1,
                "clamd-1/sig-10",
                "sig:9999999:2026-11-14T11:41:49+07:00",
                "policy-1",
                110,
                20
            )
            .unwrap()
            .is_none()
        );
        assert!(
            get(
                &path,
                &hash1,
                "clamd-1/sig-10",
                TEST_SIG_GEN,
                "policy-1",
                121,
                20
            )
            .unwrap()
            .is_none()
        );
        assert!(
            get(
                &path,
                &hash1,
                "clamd-1/sig-10",
                TEST_SIG_GEN,
                "policy-1",
                90,
                20
            )
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
        assert_eq!(version, 3);
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
                "INSERT INTO scan_cache VALUES(?1, 'engine', 'sig', 'policy', 'CLEAN', NULL, 1)",
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
            TEST_SIG_GEN,
            "policy-v1",
            ScanStatus::Infected,
            Some(threat),
            1000,
        )
        .unwrap();

        let verdict = get(
            &path,
            &hash,
            "ClamAV 1.5.4/28176",
            TEST_SIG_GEN,
            "policy-v1",
            1050,
            3600,
        )
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
            TEST_SIG_GEN,
            "policy-v1",
            ScanStatus::Clean,
            None,
            1000,
        )
        .unwrap();

        let verdict = get(
            &path,
            &hash,
            "ClamAV 1.5.4/28176",
            TEST_SIG_GEN,
            "policy-v1",
            1050,
            3600,
        )
        .unwrap()
        .expect("cache hit expected");

        assert_eq!(verdict.status, ScanStatus::Clean);
        assert_eq!(verdict.threat_name, None);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn legacy_v2_database_migrates_to_v3_discarding_old_rows() {
        let path = test_path("legacy-v2-migration");

        // Manually construct a valid schema v2 database (without sig_generation)
        let db = Connection::open(&path).unwrap();
        db.execute_batch(
            "CREATE TABLE scan_cache (
               sha256 BLOB NOT NULL CHECK(length(sha256) = 32),
               engine_version TEXT NOT NULL,
               policy_version TEXT NOT NULL,
               status TEXT NOT NULL CHECK(status IN ('CLEAN', 'INFECTED')),
               threat_name TEXT,
               scanned_at INTEGER NOT NULL,
               PRIMARY KEY(sha256, engine_version, policy_version)
             );
             PRAGMA user_version = 2;",
        )
        .unwrap();

        let hash_clean = [0x11; 32];
        db.execute(
            "INSERT INTO scan_cache(sha256, engine_version, policy_version, status, scanned_at)
             VALUES (?1, 'ClamAV 1.5.4/28176', 'policy-v1', 'CLEAN', 1000)",
            params![hash_clean.as_slice()],
        )
        .unwrap();
        drop(db);

        // Open via open(&path) -> triggers migrate() to v3
        let db = open(&path).unwrap();
        let version: i64 = db
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 3, "user_version must be bumped to 3");

        // Legacy rows without sig_generation are discarded
        let count: i64 = db
            .query_row("SELECT COUNT(*) FROM scan_cache", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0, "Old rows without sig_generation are discarded");
        drop(db);

        // Put a new entry with sig_generation into the migrated v3 schema
        put(
            &path,
            &hash_clean,
            "ClamAV 1.5.4/28176",
            TEST_SIG_GEN,
            "policy-v1",
            ScanStatus::Clean,
            None,
            1100,
        )
        .unwrap();

        let rescanned = get(
            &path,
            &hash_clean,
            "ClamAV 1.5.4/28176",
            TEST_SIG_GEN,
            "policy-v1",
            1120,
            3600,
        )
        .unwrap()
        .expect("Entry with sig_generation hits cache");
        assert_eq!(rescanned.status, ScanStatus::Clean);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn signature_generation_change_invalidates_cache_entry() {
        let path = test_path("sig-gen-invalidation");
        let hash = [0x33; 32];
        let sig_gen_1 = "sig:8616464:2026-11-14T11:41:49+07:00";
        let sig_gen_count_change = "sig:8616497:2026-11-14T11:41:49+07:00";
        let sig_gen_time_change = "sig:8616464:2026-11-14T12:00:00+07:00";

        put(
            &path,
            &hash,
            "ClamAV 1.5.4",
            sig_gen_1,
            "policy-v1",
            ScanStatus::Clean,
            None,
            1000,
        )
        .unwrap();

        // Exact match hits
        assert!(
            get(
                &path,
                &hash,
                "ClamAV 1.5.4",
                sig_gen_1,
                "policy-v1",
                1050,
                3600
            )
            .unwrap()
            .is_some()
        );

        // Signatures count bump misses
        assert!(
            get(
                &path,
                &hash,
                "ClamAV 1.5.4",
                sig_gen_count_change,
                "policy-v1",
                1050,
                3600
            )
            .unwrap()
            .is_none()
        );

        // Loaded_at reload time bump misses
        assert!(
            get(
                &path,
                &hash,
                "ClamAV 1.5.4",
                sig_gen_time_change,
                "policy-v1",
                1050,
                3600
            )
            .unwrap()
            .is_none()
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
                TEST_SIG_GEN,
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
                TEST_SIG_GEN,
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
                TEST_SIG_GEN,
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
                TEST_SIG_GEN,
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
                "INSERT INTO scan_cache(sha256, engine_version, sig_generation, policy_version, status, threat_name, scanned_at)
                 VALUES (?1, 'engine', 'sig', 'policy', 'ERROR', NULL, 100)",
                params![hash.as_slice()],
            )
            .is_err()
        );
        assert!(
            db.execute(
                "INSERT INTO scan_cache(sha256, engine_version, sig_generation, policy_version, status, threat_name, scanned_at)
                 VALUES (?1, 'engine', 'sig', 'policy', 'UNKNOWN', NULL, 100)",
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
                TEST_SIG_GEN,
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
                TEST_SIG_GEN,
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
            TEST_SIG_GEN,
            "policy-1",
            ScanStatus::Clean,
            None,
            100,
        );
        assert!(
            result.is_ok(),
            "corrupt db must automatically recover: {result:?}"
        );

        // Cache must work properly after recovery
        let cached = get(&path, &hash, "engine-v1", TEST_SIG_GEN, "policy-1", 110, 60).unwrap();
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
            TEST_SIG_GEN,
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
            TEST_SIG_GEN,
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
            TEST_SIG_GEN,
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
            get(
                &path,
                &hash1,
                "engine-v1",
                TEST_SIG_GEN,
                "policy-1",
                600,
                600
            )
            .unwrap()
            .is_none()
        );
        assert!(
            get(
                &path,
                &hash2,
                "engine-v1",
                TEST_SIG_GEN,
                "policy-1",
                600,
                600
            )
            .unwrap()
            .is_some()
        );

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn test_cache_session_batching_flush_on_cancel_preserves_verdicts() {
        let db_path = test_path("batch-cancel");

        let session = CacheSession::open(&db_path).unwrap();
        let engine = "ClamAV 1.5.4/28183";
        let sig_gen = "8616901:2026-11-14T20:59:49+07:00";
        let policy = "zonendpoint-0.3.1-v1";
        let now = 1794665700;

        // Put 10 verdicts (well below 200 batch threshold)
        for i in 0..10u8 {
            let mut hash = [0u8; 32];
            hash[0] = i;
            session
                .put(
                    hash,
                    engine.to_string(),
                    sig_gen.to_string(),
                    policy.to_string(),
                    ScanStatus::Clean,
                    None,
                    now,
                )
                .await
                .unwrap();
        }

        // Verify they are visible via get in the same session even before flush
        let mut hash5 = [0u8; 32];
        hash5[0] = 5;
        let cached = session
            .get(
                hash5,
                engine.to_string(),
                sig_gen.to_string(),
                policy.to_string(),
                now,
                3600,
            )
            .await
            .unwrap();
        assert_eq!(
            cached,
            Some(CachedVerdict {
                status: ScanStatus::Clean,
                threat_name: None,
            })
        );

        // Before flush, check that raw DB on disk has 0 entries (still in memory batch)
        {
            let raw_conn = Connection::open(&db_path).unwrap();
            let count: i64 = raw_conn
                .query_row("SELECT COUNT(*) FROM scan_cache", [], |r| r.get(0))
                .unwrap();
            assert_eq!(
                count, 0,
                "verdicts should be buffered in memory before batch threshold or flush"
            );
        }

        // Simulate scan cancellation triggering flush: no verdicts should be lost
        session.flush().await.unwrap();

        // Now verify that a completely separate connection from disk sees all 10 entries
        {
            let raw_conn = Connection::open(&db_path).unwrap();
            let count: i64 = raw_conn
                .query_row("SELECT COUNT(*) FROM scan_cache", [], |r| r.get(0))
                .unwrap();
            assert_eq!(
                count, 10,
                "all verdicts must be safely persisted on cancel flush"
            );
        }

        let _ = std::fs::remove_file(&db_path);
    }

    #[tokio::test]
    async fn test_cache_session_auto_flushes_at_200_items() {
        let db_path = test_path("batch-200");

        let session = CacheSession::open(&db_path).unwrap();
        let engine = "ClamAV 1.5.4/28183";
        let sig_gen = "8616901:2026-11-14T20:59:49+07:00";
        let policy = "zonendpoint-0.3.1-v1";
        let now = 1794665700;

        for i in 0..199u16 {
            let mut hash = [0u8; 32];
            hash[0..2].copy_from_slice(&i.to_be_bytes());
            session
                .put(
                    hash,
                    engine.to_string(),
                    sig_gen.to_string(),
                    policy.to_string(),
                    ScanStatus::Clean,
                    None,
                    now,
                )
                .await
                .unwrap();
        }
        {
            let raw_conn = Connection::open(&db_path).unwrap();
            let count: i64 = raw_conn
                .query_row("SELECT COUNT(*) FROM scan_cache", [], |r| r.get(0))
                .unwrap();
            assert_eq!(count, 0, "199 items should still be in memory batch");
        }

        // 200th item triggers auto-flush
        let mut hash200 = [0u8; 32];
        hash200[0..2].copy_from_slice(&200u16.to_be_bytes());
        session
            .put(
                hash200,
                engine.to_string(),
                sig_gen.to_string(),
                policy.to_string(),
                ScanStatus::Clean,
                None,
                now,
            )
            .await
            .unwrap();

        {
            let raw_conn = Connection::open(&db_path).unwrap();
            let count: i64 = raw_conn
                .query_row("SELECT COUNT(*) FROM scan_cache", [], |r| r.get(0))
                .unwrap();
            assert_eq!(
                count, 200,
                "200 items must be automatically flushed to sqlite in a single transaction"
            );
        }

        let _ = std::fs::remove_file(&db_path);
    }
}
