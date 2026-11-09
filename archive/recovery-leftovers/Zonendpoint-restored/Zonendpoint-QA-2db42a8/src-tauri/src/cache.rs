use anyhow::Result;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
pub struct CacheStats { pub entries: i64 }

fn open(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
    let db = Connection::open(path)?;
    db.pragma_update(None, "journal_mode", "WAL")?;
    db.execute_batch("CREATE TABLE IF NOT EXISTS scan_cache(path TEXT NOT NULL, size INTEGER NOT NULL, mtime INTEGER NOT NULL, sha256 TEXT NOT NULL, status TEXT NOT NULL, PRIMARY KEY(path, size, mtime)); CREATE INDEX IF NOT EXISTS idx_scan_cache_sha256 ON scan_cache(sha256);")?;
    Ok(db)
}

pub fn stats(path: &Path) -> Result<CacheStats> {
    let db = open(path)?;
    Ok(CacheStats { entries: db.query_row("SELECT COUNT(*) FROM scan_cache", [], |r| r.get(0))? })
}

pub fn get(path: &Path, file: &str, size: u64, mtime: i64) -> Result<Option<(String, String)>> {
    let db = open(path)?;
    let mut stmt = db.prepare("SELECT sha256,status FROM scan_cache WHERE path=?1 AND size=?2 AND mtime=?3 LIMIT 1")?;
    let mut rows = stmt.query(params![file, size as i64, mtime])?;
    if let Some(row) = rows.next()? { Ok(Some((row.get(0)?, row.get(1)?))) } else { Ok(None) }
}

#[allow(dead_code)]
pub fn put(path: &Path, file: &str, size: u64, mtime: i64, sha256: &str, status: &str) -> Result<()> {
    open(path)?.execute("INSERT OR REPLACE INTO scan_cache(path,size,mtime,sha256,status) VALUES(?1,?2,?3,?4,?5)", params![file, size as i64, mtime, sha256, status])?;
    Ok(())
}
