use anyhow::{Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{path::Path, time::UNIX_EPOCH};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream};

#[derive(Serialize)]
pub struct ScanResult { pub path: String, pub sha256: String, pub verdict: String }

pub async fn scan_file(path: &str) -> Result<ScanResult> {
    let file_path = path.to_owned();
    let mut file = tokio::fs::File::open(path).await.context("open scan target")?;
    let mut stream = TcpStream::connect(std::env::var("ZONENDPOINT_CLAMD").unwrap_or_else(|_| "127.0.0.1:3310".into())).await.context("connect to clamd (set ZONENDPOINT_CLAMD)")?;
    stream.write_all(b"zINSTREAM\0").await?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
        stream.write_all(&(n as u32).to_be_bytes()).await?;
        stream.write_all(&buf[..n]).await?;
    }
    stream.write_all(&0u32.to_be_bytes()).await?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response).await?;
    let text = String::from_utf8_lossy(&response).trim().to_owned();
    let verdict = if text.contains("FOUND") { "infected" } else if text.contains("OK") { "clean" } else { "unknown" };
    let _metadata = std::fs::metadata(Path::new(path))?;
    Ok(ScanResult { path: file_path, sha256: hex::encode(hasher.finalize()), verdict: format!("{verdict}: {text}") })
}
