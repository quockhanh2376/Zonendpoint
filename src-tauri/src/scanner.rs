#[cfg(all(feature = "native-ui-tests", not(debug_assertions)))]
compile_error!("native-ui-tests must never be enabled in release builds");

use anyhow::Result;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    fs::File,
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::{Semaphore, watch},
    time::timeout,
};

const STREAM_CHUNK_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 1024;
pub const MAX_FILE_BYTES: u64 = 25 * 1024 * 1024;
pub const CACHE_TTL_SECONDS: i64 = 12 * 60 * 60;
pub const POLICY_VERSION: &str = "zonendpoint-mvp-1";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
pub const PREFLIGHT_ERROR_MSG: &str = "Cannot connect to ZonEndpoint server";
const PREFLIGHT_TIMEOUT: Duration = Duration::from_secs(3);
const TOTAL_SCAN_TIMEOUT: Duration = Duration::from_secs(60);

pub fn default_scan_timeout() -> Duration {
    #[cfg(all(feature = "native-ui-tests", debug_assertions))]
    if let Ok(ms) = std::env::var("ZONENDPOINT_TEST_SCAN_TIMEOUT_MS") {
        if let Ok(ms) = ms.parse::<u64>() {
            return Duration::from_millis(ms);
        }
    }
    TOTAL_SCAN_TIMEOUT
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScanStatus {
    Clean,
    Infected,
    Unknown,
    Error,
    Skipped,
    Cancelled,
}

pub const DEFAULT_CLAMD_ADDR: &str = "127.0.0.1:13310";

pub fn validate_clamd_addr(raw: &str) -> Result<SocketAddr, String> {
    let addr: SocketAddr = raw.trim().parse().map_err(|_| {
        "Server address must be an IP address and port (e.g. 127.0.0.1:13310)".to_owned()
    })?;
    let allowed = match addr.ip() {
        std::net::IpAddr::V4(ip) => {
            ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || (ip.octets()[0] == 100 && (64..=127).contains(&ip.octets()[1]))
        }
        std::net::IpAddr::V6(ip) => {
            ip.is_loopback() || ip.is_unique_local() || ip.is_unicast_link_local()
        }
    };
    if !allowed {
        return Err(
            "Server address must use loopback, private, link-local, or VPN addressing".into(),
        );
    }
    Ok(addr)
}

pub fn configured_clamd_addr() -> Result<SocketAddr, String> {
    // 1. ZONENDPOINT_CLAMD takes priority
    if let Ok(value) = std::env::var("ZONENDPOINT_CLAMD") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return validate_clamd_addr(trimmed);
        }
    }
    // 2. Saved configuration in app config dir
    if let Ok(Some(saved)) = crate::config::load_saved_server_address() {
        let trimmed = saved.trim();
        if !trimmed.is_empty() {
            return validate_clamd_addr(trimmed);
        }
    }
    // 3. Default fallback: 127.0.0.1:13310
    validate_clamd_addr(DEFAULT_CLAMD_ADDR)
}

#[derive(Clone, Debug, Serialize)]
pub struct ScanResult {
    pub path: String,
    pub sha256: Option<String>,
    pub status: ScanStatus,
    pub reason: Option<String>,
    pub engine_version: Option<String>,
    pub bytes_scanned: u64,
}

impl ScanResult {
    pub fn new(
        path: impl AsRef<Path>,
        status: ScanStatus,
        sha256: Option<String>,
        reason: Option<String>,
        bytes_scanned: u64,
    ) -> Self {
        Self {
            path: path.as_ref().to_string_lossy().into_owned(),
            sha256,
            status,
            reason,
            engine_version: None,
            bytes_scanned,
        }
    }

    pub fn error(path: impl AsRef<Path>, reason: impl Into<String>) -> Self {
        Self::new(path, ScanStatus::Error, None, Some(reason.into()), 0)
    }
}

#[derive(Debug)]
struct StreamOutcome {
    status: ScanStatus,
    sha256: Option<String>,
    reason: Option<String>,
    bytes_scanned: u64,
}

fn parse_response(response: &[u8]) -> Result<(ScanStatus, Option<String>), String> {
    let Some(response) = response.strip_suffix(b"\0") else {
        return Err("server response is not NUL terminated".into());
    };
    let response = std::str::from_utf8(response).map_err(|_| "server response is not UTF-8")?;
    if response == "stream: OK" {
        return Ok((ScanStatus::Clean, None));
    }
    if let Some(err_msg) = response.strip_suffix(" ERROR") {
        let msg = err_msg.strip_prefix("stream: ").unwrap_or(err_msg);
        if !msg.trim().is_empty() && !msg.bytes().any(|byte| byte.is_ascii_control()) {
            return Ok((ScanStatus::Error, Some(msg.trim().to_owned())));
        }
    }
    if let Some(threat) = response
        .strip_prefix("stream: ")
        .and_then(|text| text.strip_suffix(" FOUND"))
        && !threat.trim().is_empty()
        && !threat.bytes().any(|byte| byte.is_ascii_control())
    {
        return Ok((ScanStatus::Infected, Some(threat.trim().to_owned())));
    }
    Err("unrecognized server response".into())
}

async fn read_nul_frame<R: AsyncRead + Unpin>(
    reader: &mut R,
    limit: usize,
) -> Result<Vec<u8>, String> {
    let mut response = Vec::with_capacity(64);
    loop {
        if response.len() == limit {
            return Err("server response exceeded limit".into());
        }
        let mut byte = [0];
        let read_result = timeout(Duration::from_secs(5), reader.read(&mut byte)).await;
        let count = match read_result {
            Ok(Ok(n)) => n,
            Ok(Err(error)) => return Err(format!("read server response: {error}")),
            Err(_) => return Err("read server response timed out".into()),
        };
        match count {
            0 => return Err("server closed before response terminator".into()),
            _ if byte[0] == 0 => {
                response.push(0);
                return Ok(response);
            }
            _ => response.push(byte[0]),
        }
    }
}

const EOF_DRAIN_TIMEOUT: Duration = Duration::from_secs(1);

async fn read_until_eof<R: AsyncRead + Unpin>(
    reader: &mut R,
    timeout_duration: Duration,
) -> Result<(), String> {
    let mut extra = [0u8; 64];
    match timeout(timeout_duration, reader.read(&mut extra)).await {
        Ok(Ok(0)) => Ok(()),
        Ok(Ok(_)) => Err("server sent unexpected extra data after response terminator".into()),
        Ok(Err(error)) => Err(format!("read server response close: {error}")),
        Err(_) => Err("server did not close connection after response terminator".into()),
    }
}

async fn wait_cancelled(mut cancel: watch::Receiver<bool>) {
    loop {
        if *cancel.borrow() {
            return;
        }
        if cancel.changed().await.is_err() {
            return;
        }
    }
}

pub async fn clamd_version_with_timeout(
    addr: SocketAddr,
    connect_timeout: Duration,
    cancel: watch::Receiver<bool>,
) -> Result<String, String> {
    let query = async {
        let mut socket = timeout(connect_timeout, TcpStream::connect(addr))
            .await
            .map_err(|_| "server connection timed out".to_owned())?
            .map_err(|error| format!("connect to server: {error}"))?;
        socket
            .write_all(b"zVERSION\0")
            .await
            .map_err(|error| format!("write VERSION command: {error}"))?;
        let response = timeout(connect_timeout, read_nul_frame(&mut socket, 256))
            .await
            .map_err(|_| "VERSION command timed out".to_owned())??;
        read_until_eof(&mut socket, EOF_DRAIN_TIMEOUT).await?;
        if response.is_empty() || response[response.len() - 1] != 0 {
            return Err("VERSION response is not NUL terminated".into());
        }
        let text = std::str::from_utf8(&response[..response.len() - 1])
            .map_err(|_| "VERSION response is not UTF-8")?;
        if !text.starts_with("ClamAV ") || text.len() <= 7 {
            return Err("invalid VERSION response".into());
        }
        let parts: Vec<&str> = text.split('/').collect();
        if parts.len() < 2 || parts[0].trim().len() <= 7 || parts[1].trim().is_empty() {
            return Err("invalid VERSION response: missing signature version".into());
        }
        Ok(text.to_owned())
    };
    tokio::select! {
        _ = wait_cancelled(cancel.clone()) => Err("cancelled".into()),
        result = query => result,
    }
}

pub async fn clamd_version(
    addr: SocketAddr,
    cancel: watch::Receiver<bool>,
) -> Result<String, String> {
    clamd_version_with_timeout(addr, CONNECT_TIMEOUT, cancel).await
}

pub async fn clamd_preflight(
    addr: SocketAddr,
    cancel: watch::Receiver<bool>,
) -> Result<String, String> {
    clamd_version_with_timeout(addr, PREFLIGHT_TIMEOUT, cancel)
        .await
        .map_err(|_| PREFLIGHT_ERROR_MSG.to_string())
}

async fn stream_scan<R: AsyncRead + Unpin>(
    reader: &mut R,
    addr: SocketAddr,
    max_bytes: u64,
    cancel: watch::Receiver<bool>,
    total_timeout: Duration,
) -> StreamOutcome {
    let work = async {
        let mut socket = timeout(CONNECT_TIMEOUT, TcpStream::connect(addr))
            .await
            .map_err(|_| "server connection timed out".to_owned())?
            .map_err(|error| format!("connect to server: {error}"))?;
        socket
            .write_all(b"zINSTREAM\0")
            .await
            .map_err(|error| format!("write server command: {error}"))?;
        let mut buffer = vec![0; STREAM_CHUNK_BYTES];
        let mut total = 0u64;
        let mut hasher = Sha256::new();
        loop {
            let count = reader
                .read(&mut buffer)
                .await
                .map_err(|error| format!("read scan target: {error}"))?;
            if count == 0 {
                break;
            }
            if total.saturating_add(count as u64) > max_bytes {
                return Ok(StreamOutcome {
                    status: ScanStatus::Skipped,
                    sha256: None,
                    reason: Some("file exceeds scan size limit".into()),
                    bytes_scanned: total,
                });
            }
            total += count as u64;
            hasher.update(&buffer[..count]);
            socket
                .write_all(&(count as u32).to_be_bytes())
                .await
                .map_err(|error| format!("write stream frame: {error}"))?;
            socket
                .write_all(&buffer[..count])
                .await
                .map_err(|error| format!("write stream payload: {error}"))?;
        }
        socket
            .write_all(&0u32.to_be_bytes())
            .await
            .map_err(|error| format!("finish stream: {error}"))?;
        let response = read_nul_frame(&mut socket, MAX_RESPONSE_BYTES).await?;
        read_until_eof(&mut socket, EOF_DRAIN_TIMEOUT).await?;
        let (status, reason) = parse_response(&response)?;
        Ok(StreamOutcome {
            status,
            sha256: Some(hex::encode(hasher.finalize())),
            reason,
            bytes_scanned: total,
        })
    };
    tokio::select! {
        _ = wait_cancelled(cancel) => StreamOutcome { status: ScanStatus::Cancelled, sha256: None, reason: Some("cancelled".into()), bytes_scanned: 0 },
        result = timeout(total_timeout, work) => match result {
            Ok(Ok(outcome)) => outcome,
            Ok(Err(reason)) => StreamOutcome { status: ScanStatus::Error, sha256: None, reason: Some(reason), bytes_scanned: 0 },
            Err(_) => StreamOutcome { status: ScanStatus::Error, sha256: None, reason: Some("scan timed out".into()), bytes_scanned: 0 },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FileSnapshot {
    size: u64,
    modified: SystemTime,
}

fn snapshot(metadata: &std::fs::Metadata) -> Result<FileSnapshot, String> {
    Ok(FileSnapshot {
        size: metadata.len(),
        modified: metadata
            .modified()
            .map_err(|error| format!("read file modified time: {error}"))?,
    })
}

async fn hash_file(path: &Path) -> Result<([u8; 32], u64, FileSnapshot), String> {
    let mut file = File::open(path)
        .await
        .map_err(|error| format!("open scan target: {error}"))?;
    let before = snapshot(
        &file
            .metadata()
            .await
            .map_err(|error| format!("read scan target metadata: {error}"))?,
    )?;
    if before.size > MAX_FILE_BYTES {
        return Err("file exceeds scan size limit".into());
    }
    let mut hasher = Sha256::new();
    let mut total = 0u64;
    let mut buffer = vec![0; STREAM_CHUNK_BYTES];
    loop {
        let count = file
            .read(&mut buffer)
            .await
            .map_err(|error| format!("read scan target: {error}"))?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > MAX_FILE_BYTES {
            return Err("file exceeds scan size limit".into());
        }
        hasher.update(&buffer[..count]);
    }
    let after_handle = snapshot(
        &file
            .metadata()
            .await
            .map_err(|error| format!("recheck scan target: {error}"))?,
    )?;
    let after_path = snapshot(
        &tokio::fs::metadata(path)
            .await
            .map_err(|error| format!("recheck scan path: {error}"))?,
    )?;
    if before != after_handle || before != after_path || total != before.size {
        return Err("file changed while hashing".into());
    }
    Ok((hasher.finalize().into(), total, before))
}

pub async fn scan_file(
    path: PathBuf,
    cache_path: PathBuf,
    addr: SocketAddr,
    cancel: watch::Receiver<bool>,
    semaphore: Arc<Semaphore>,
) -> ScanResult {
    scan_file_with_engine_override(path, cache_path, addr, None, cancel, semaphore).await
}

pub async fn scan_file_with_engine_override(
    path: PathBuf,
    cache_path: PathBuf,
    addr: SocketAddr,
    engine_override: Option<String>,
    cancel: watch::Receiver<bool>,
    semaphore: Arc<Semaphore>,
) -> ScanResult {
    scan_file_with_engine_timeout_checked(
        path,
        cache_path,
        addr,
        engine_override,
        cancel,
        semaphore,
        default_scan_timeout(),
    )
    .await
}

pub async fn scan_file_with_timeout(
    path: PathBuf,
    cache_path: PathBuf,
    addr: SocketAddr,
    cancel: watch::Receiver<bool>,
    semaphore: Arc<Semaphore>,
    total_timeout: Duration,
) -> ScanResult {
    scan_file_with_engine_timeout_checked(
        path,
        cache_path,
        addr,
        None,
        cancel,
        semaphore,
        total_timeout,
    )
    .await
}

async fn scan_file_with_engine_timeout_checked(
    path: PathBuf,
    cache_path: PathBuf,
    addr: SocketAddr,
    engine_override: Option<String>,
    cancel: watch::Receiver<bool>,
    semaphore: Arc<Semaphore>,
    total_timeout: Duration,
) -> ScanResult {
    if *cancel.borrow() {
        return ScanResult::new(
            path,
            ScanStatus::Cancelled,
            None,
            Some("cancelled".into()),
            0,
        );
    }
    match tokio::fs::symlink_metadata(&path).await {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return ScanResult::new(
                path,
                ScanStatus::Skipped,
                None,
                Some("symbolic links are not followed".into()),
                0,
            );
        }
        Ok(metadata) if metadata.is_file() && metadata.len() > MAX_FILE_BYTES => {
            return ScanResult::new(
                path,
                ScanStatus::Skipped,
                None,
                Some("file exceeds scan size limit".into()),
                0,
            );
        }
        Ok(metadata) if !metadata.is_file() => {
            return ScanResult::new(
                path,
                ScanStatus::Skipped,
                None,
                Some("path is not a regular file".into()),
                0,
            );
        }
        Ok(_) => {}
        Err(error) => return ScanResult::error(path, format!("read file metadata: {error}")),
    }
    scan_file_with_engine_timeout(
        path,
        cache_path,
        addr,
        engine_override,
        cancel,
        semaphore,
        total_timeout,
    )
    .await
}

pub async fn scan_file_with_engine_timeout(
    path: PathBuf,
    cache_path: PathBuf,
    addr: SocketAddr,
    engine_override: Option<String>,
    cancel: watch::Receiver<bool>,
    semaphore: Arc<Semaphore>,
    total_timeout: Duration,
) -> ScanResult {
    let permit = tokio::select! {
        _ = wait_cancelled(cancel.clone()) => return ScanResult::new(path, ScanStatus::Cancelled, None, Some("cancelled".into()), 0),
        permit = semaphore.acquire_owned() => match permit {
            Ok(permit) => permit,
            Err(_) => return ScanResult::error(path, "scan worker limit is unavailable"),
        }
    };
    let operation = async {
        let engine_version = match &engine_override {
            Some(version) => version.clone(),
            None => clamd_version(addr, cancel.clone()).await?,
        };
        let (hash, size, file_snapshot) = hash_file(&path).await?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_secs() as i64;
        if let Some(cached) = crate::cache::get(
            &cache_path,
            &hash,
            &engine_version,
            POLICY_VERSION,
            now,
            CACHE_TTL_SECONDS,
        )
        .map_err(|error| format!("read scan cache: {error}"))?
        {
            let (verify_hash, verify_size, verify_snapshot) = hash_file(&path).await?;
            if verify_hash != hash || verify_size != size || verify_snapshot != file_snapshot {
                return Ok::<ScanResult, String>(ScanResult::new(
                    &path,
                    ScanStatus::Unknown,
                    Some(hex::encode(hash)),
                    Some("file changed while validating cached verdict".into()),
                    size,
                ));
            }
            let reason = match cached.status {
                ScanStatus::Infected => cached.threat_name,
                _ => Some("verified content cache hit".into()),
            };
            let mut result =
                ScanResult::new(&path, cached.status, Some(hex::encode(hash)), reason, size);
            result.engine_version = Some(engine_version.clone());
            return Ok(result);
        }
        let mut file = File::open(&path)
            .await
            .map_err(|error| format!("open scan target for streaming: {error}"))?;
        let before = snapshot(
            &file
                .metadata()
                .await
                .map_err(|error| format!("read scan target metadata: {error}"))?,
        )?;
        let outcome = stream_scan(
            &mut file,
            addr,
            MAX_FILE_BYTES,
            cancel.clone(),
            total_timeout,
        )
        .await;
        if outcome.status == ScanStatus::Clean || outcome.status == ScanStatus::Infected {
            let after_handle = snapshot(
                &file
                    .metadata()
                    .await
                    .map_err(|error| format!("recheck scan target: {error}"))?,
            )?;
            let after_path = snapshot(
                &tokio::fs::metadata(&path)
                    .await
                    .map_err(|error| format!("recheck scan path: {error}"))?,
            )?;
            let stream_hash = outcome
                .sha256
                .as_deref()
                .and_then(|value| hex::decode(value).ok())
                .and_then(|value| <[u8; 32]>::try_from(value).ok());
            if before != file_snapshot
                || before != after_handle
                || before != after_path
                || outcome.bytes_scanned != size
                || stream_hash != Some(hash)
            {
                return Ok(ScanResult::new(
                    &path,
                    ScanStatus::Unknown,
                    Some(hex::encode(hash)),
                    Some("file changed during scan; verdict not accepted".into()),
                    outcome.bytes_scanned,
                ));
            }
            let mut result = ScanResult::new(
                &path,
                outcome.status,
                outcome.sha256,
                outcome.reason,
                outcome.bytes_scanned,
            );
            result.engine_version = Some(engine_version.clone());
            if let Err(error) = crate::cache::put(
                &cache_path,
                &hash,
                &engine_version,
                POLICY_VERSION,
                outcome.status,
                result.reason.as_deref(),
                now,
            ) {
                result.reason = Some(format!(
                    "{}; cache write failed: {error}",
                    result
                        .reason
                        .take()
                        .unwrap_or_else(|| "scan completed".into())
                ));
            }
            return Ok(result);
        }
        let mut result = ScanResult::new(
            &path,
            outcome.status,
            outcome.sha256,
            outcome.reason,
            outcome.bytes_scanned,
        );
        result.engine_version = Some(engine_version.clone());
        Ok(result)
    };
    let result_cancel = cancel.clone();
    let result = tokio::select! {
        _ = wait_cancelled(result_cancel) => ScanResult::new(&path, ScanStatus::Cancelled, None, Some("cancelled".into()), 0),
        result = timeout(total_timeout, operation) => match result {
            Ok(Ok(result)) => result,
            Ok(Err(reason)) => ScanResult::error(&path, reason),
            Err(_) => ScanResult::new(&path, ScanStatus::Error, None, Some("scan timed out".into()), 0),
        }
    };
    drop(permit);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::{net::TcpListener, sync::watch};

    #[test]
    fn malformed_response_containing_ok_is_not_clean() {
        assert!(parse_response(b"garbage OK\0").is_err());
        assert!(parse_response(b"stream: ERROR: failed OK\0").is_err());
    }

    #[test]
    fn only_terminal_clamd_replies_map_to_clean_or_infected() {
        assert_eq!(
            parse_response(b"stream: OK\0").unwrap(),
            (ScanStatus::Clean, None)
        );
        assert_eq!(
            parse_response(b"stream: Eicar-Test-Signature FOUND\0").unwrap(),
            (
                ScanStatus::Infected,
                Some("Eicar-Test-Signature".to_string())
            )
        );
        assert_eq!(
            parse_response(b"stream: failed ERROR\0").unwrap(),
            (ScanStatus::Error, Some("failed".to_string()))
        );
        assert_eq!(
            parse_response(b"INSTREAM size limit exceeded. ERROR\0").unwrap(),
            (
                ScanStatus::Error,
                Some("INSTREAM size limit exceeded.".to_string())
            )
        );
        assert!(parse_response(b"\0").is_err());
        assert!(parse_response(b"stream: OK").is_err());
        assert!(parse_response(b"stream: OK trailing\0").is_err());
        assert!(parse_response(b"stream: \n FOUND\0").is_err());
    }

    async fn mock_clamd(
        parts: Vec<Vec<u8>>,
        hold_open: bool,
    ) -> (std::net::SocketAddr, tokio::task::JoinHandle<Vec<u8>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut command = [0; 10];
            socket.read_exact(&mut command).await.unwrap();
            assert_eq!(&command, b"zINSTREAM\0");
            let mut payload = Vec::new();
            loop {
                let size = match socket.read_u32().await {
                    Ok(size) => size,
                    Err(_) => return payload,
                };
                if size == 0 {
                    break;
                }
                let start = payload.len();
                payload.resize(start + size as usize, 0);
                socket.read_exact(&mut payload[start..]).await.unwrap();
            }
            for part in parts {
                socket.write_all(&part).await.unwrap();
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            if hold_open {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            payload
        });
        (addr, task)
    }

    async fn bytes_reader(bytes: &[u8]) -> tokio::io::DuplexStream {
        let (mut writer, reader) = tokio::io::duplex(bytes.len().max(1));
        writer.write_all(bytes).await.unwrap();
        drop(writer);
        reader
    }

    #[tokio::test]
    async fn mock_daemon_checks_instream_framing_and_fragmented_found() {
        let payload = b"fixture-bytes";
        let (addr, daemon) = mock_clamd(
            vec![
                b"stream: Eicar-Test-".to_vec(),
                b"Signature FOUND\0".to_vec(),
            ],
            false,
        )
        .await;
        let mut reader = bytes_reader(payload).await;
        let (_tx, cancel) = watch::channel(false);
        let result = stream_scan(&mut reader, addr, 1024, cancel, Duration::from_secs(1)).await;
        assert_eq!(result.status, ScanStatus::Infected);
        let expected_hash = hex::encode(Sha256::digest(payload));
        assert_eq!(result.sha256.as_deref(), Some(expected_hash.as_str()));
        assert_eq!(daemon.await.unwrap(), payload);
    }

    #[tokio::test]
    async fn mock_daemon_never_maps_malformed_or_empty_reply_to_clean() {
        for reply in [
            b"garbage OK\0".as_slice(),
            b"".as_slice(),
            b"stream: OK".as_slice(),
        ] {
            let parts = if reply.is_empty() {
                vec![]
            } else {
                vec![reply.to_vec()]
            };
            let (addr, daemon) = mock_clamd(parts, false).await;
            let mut reader = bytes_reader(b"fixture").await;
            let (_tx, cancel) = watch::channel(false);
            let result =
                stream_scan(&mut reader, addr, 1024, cancel, Duration::from_millis(300)).await;
            assert_eq!(result.status, ScanStatus::Error);
            let _ = daemon.await.unwrap();
        }
    }

    #[tokio::test]
    async fn mock_daemon_response_limit_is_enforced() {
        let (addr, daemon) = mock_clamd(vec![vec![b'x'; MAX_RESPONSE_BYTES + 1]], false).await;
        let mut reader = bytes_reader(b"fixture").await;
        let (_tx, cancel) = watch::channel(false);
        let result = stream_scan(&mut reader, addr, 1024, cancel, Duration::from_secs(1)).await;
        assert_eq!(result.status, ScanStatus::Error);
        let _ = daemon.await.unwrap();
    }

    #[tokio::test]
    async fn mock_daemon_error_is_never_clean() {
        let (addr, daemon) = mock_clamd(vec![b"stream: size limit ERROR\0".to_vec()], false).await;
        let mut reader = bytes_reader(b"fixture").await;
        let (_tx, cancel) = watch::channel(false);
        let result = stream_scan(&mut reader, addr, 1024, cancel, Duration::from_secs(1)).await;
        assert_eq!(result.status, ScanStatus::Error);
        let _ = daemon.await.unwrap();
    }

    #[tokio::test]
    async fn stream_limit_and_cancellation_are_explicit() {
        let (addr, daemon) = mock_clamd(vec![], false).await;
        let mut reader = bytes_reader(b"too large").await;
        let (_tx, cancel) = watch::channel(false);
        let result = stream_scan(&mut reader, addr, 2, cancel, Duration::from_secs(1)).await;
        assert_eq!(result.status, ScanStatus::Skipped);
        let _ = daemon.await.unwrap();

        let (addr, daemon) = mock_clamd(vec![], true).await;
        let mut reader = bytes_reader(b"fixture").await;
        let (tx, cancel) = watch::channel(false);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(30)).await;
            let _ = tx.send(true);
        });
        let result = stream_scan(&mut reader, addr, 1024, cancel, Duration::from_secs(1)).await;
        assert_eq!(result.status, ScanStatus::Cancelled);
        let _ = daemon.await.unwrap();

        let (addr, daemon) = mock_clamd(vec![], true).await;
        let mut reader = bytes_reader(b"fixture").await;
        let (_tx, cancel) = watch::channel(false);
        let result = stream_scan(&mut reader, addr, 1024, cancel, Duration::from_millis(30)).await;
        assert_eq!(result.status, ScanStatus::Error);
        assert_eq!(result.reason.as_deref(), Some("scan timed out"));
        let _ = daemon.await.unwrap();
    }

    #[tokio::test]
    async fn same_size_same_mtime_content_change_cannot_reuse_clean_cache() {
        let root =
            std::env::temp_dir().join(format!("zonendpoint-cache-race-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let file_path = root.join("fixture.bin");
        let cache_path = root.join("cache.db");
        std::fs::write(&file_path, b"good").unwrap();
        let original = std::fs::metadata(&file_path).unwrap();
        let original_mtime = original.modified().unwrap();
        let old_hash: [u8; 32] = Sha256::digest(b"good").into();
        crate::cache::put(
            &cache_path,
            &old_hash,
            "ClamAV test/signature-1",
            POLICY_VERSION,
            ScanStatus::Clean,
            None,
            100,
        )
        .unwrap();
        std::fs::write(&file_path, b"evil").unwrap();
        std::fs::OpenOptions::new()
            .write(true)
            .open(&file_path)
            .unwrap()
            .set_modified(original_mtime)
            .unwrap();
        let changed = std::fs::metadata(&file_path).unwrap();
        assert_eq!(original.len(), changed.len());
        assert_eq!(original.modified().unwrap(), changed.modified().unwrap());

        let (addr, daemon) = mock_clamd(vec![b"stream: Mock-Threat FOUND\0".to_vec()], false).await;
        let (_tx, cancel) = watch::channel(false);
        let result = scan_file_with_engine_timeout(
            file_path.clone(),
            cache_path.clone(),
            addr,
            Some("ClamAV 1.5.4/28175/Sat Nov 7 2026".into()),
            cancel,
            Arc::new(Semaphore::new(1)),
            Duration::from_secs(60),
        )
        .await;
        assert_eq!(result.status, ScanStatus::Infected);
        assert_eq!(daemon.await.unwrap(), b"evil");
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn offline_engine_cannot_reuse_cached_clean_verdict() {
        let root = std::env::temp_dir().join(format!("zonendpoint-offline-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let file_path = root.join("fixture.bin");
        let cache_path = root.join("cache.db");
        std::fs::write(&file_path, b"known clean bytes").unwrap();
        let hash: [u8; 32] = Sha256::digest(b"known clean bytes").into();
        crate::cache::put(
            &cache_path,
            &hash,
            "ClamAV 1.5.4/28175/Sat Nov 7 2026",
            POLICY_VERSION,
            ScanStatus::Clean,
            None,
            9999999999,
        )
        .unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let (_tx, cancel) = watch::channel(false);
        let result = scan_file(
            file_path.clone(),
            cache_path.clone(),
            addr,
            cancel,
            Arc::new(Semaphore::new(1)),
        )
        .await;
        assert_eq!(result.status, ScanStatus::Error);
        let _ = std::fs::remove_dir_all(root);
    }

    async fn mock_version_daemon(
        reply: Vec<u8>,
    ) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut cmd = [0u8; 9];
            let n = socket.read(&mut cmd).await.unwrap();
            assert_eq!(&cmd[..n], b"zVERSION\0");
            socket.write_all(&reply).await.unwrap();
        });
        (addr, task)
    }

    #[tokio::test]
    async fn preflight_ok_returns_validated_engine_version() {
        let banner = b"ClamAV 1.5.4/28175/Sat Nov 7 2026\0".to_vec();
        let (addr, daemon) = mock_version_daemon(banner).await;
        let (_tx, cancel) = watch::channel(false);
        let result = clamd_preflight(addr, cancel).await;
        assert_eq!(result.unwrap(), "ClamAV 1.5.4/28175/Sat Nov 7 2026");
        daemon.await.unwrap();
    }

    #[tokio::test]
    async fn preflight_refused_returns_generic_server_error() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let (_tx, cancel) = watch::channel(false);
        let result = clamd_preflight(addr, cancel).await;
        assert_eq!(result.unwrap_err(), PREFLIGHT_ERROR_MSG);
    }

    #[tokio::test]
    async fn preflight_invalid_banner_rejected() {
        for invalid in [
            b"SSH-2.0-OpenSSH_8.9p1\r\n\0".to_vec(),
            b"HTTP/1.1 200 OK\r\n\r\n\0".to_vec(),
            b"ClamAV 1.5.4\0".to_vec(),
            b"ClamAV \0".to_vec(),
            b"random banner\0".to_vec(),
            b"\0".to_vec(),
        ] {
            let (addr, daemon) = mock_version_daemon(invalid).await;
            let (_tx, cancel) = watch::channel(false);
            let result = clamd_preflight(addr, cancel).await;
            assert_eq!(result.unwrap_err(), PREFLIGHT_ERROR_MSG);
            daemon.await.unwrap();
        }
    }

    #[tokio::test]
    async fn preflight_offline_cache_not_reused() {
        let root = std::env::temp_dir().join(format!(
            "zonendpoint-preflight-cache-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let file_path = root.join("fixture.bin");
        let cache_path = root.join("cache.db");
        std::fs::write(&file_path, b"known clean data").unwrap();
        let hash: [u8; 32] = Sha256::digest(b"known clean data").into();
        crate::cache::put(
            &cache_path,
            &hash,
            "ClamAV 1.5.4/28175/Sat Nov 7 2026",
            POLICY_VERSION,
            ScanStatus::Clean,
            None,
            9999999999,
        )
        .unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let (_tx, cancel) = watch::channel(false);
        let preflight = clamd_preflight(addr, cancel).await;
        assert_eq!(preflight.unwrap_err(), PREFLIGHT_ERROR_MSG);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn validate_clamd_addr_allows_private_and_vpn_ips() {
        assert!(validate_clamd_addr("127.0.0.1:13310").is_ok());
        assert!(validate_clamd_addr("100.96.250.245:3310").is_ok());
        assert!(validate_clamd_addr("192.168.1.100:3310").is_ok());
        assert!(validate_clamd_addr("10.0.0.5:3310").is_ok());
        assert!(validate_clamd_addr("172.16.5.10:3310").is_ok());
        assert!(validate_clamd_addr("[::1]:3310").is_ok());
    }

    #[test]
    fn validate_clamd_addr_rejects_public_and_invalid_inputs() {
        assert!(validate_clamd_addr("8.8.8.8:3310").is_err());
        assert!(validate_clamd_addr("1.1.1.1:3310").is_err());
        assert!(validate_clamd_addr("example.com:3310").is_err());
        assert!(validate_clamd_addr("127.0.0.1").is_err());
        assert!(validate_clamd_addr("not-an-ip:3310").is_err());
    }

    #[test]
    fn priority_env_overrides_saved_and_default() {
        let temp_dir =
            std::env::temp_dir().join(format!("zonendpoint-cfg-prio-{}", uuid::Uuid::new_v4()));
        let cfg_file = temp_dir.join("config.json");
        unsafe {
            std::env::set_var("ZONENDPOINT_CONFIG_FILE", &cfg_file);
        }

        // 1. Neither env nor saved -> default 127.0.0.1:13310
        unsafe {
            std::env::remove_var("ZONENDPOINT_CLAMD");
        }
        let _ = std::fs::remove_file(&cfg_file);
        assert_eq!(
            configured_clamd_addr().unwrap().to_string(),
            "127.0.0.1:13310"
        );

        // 2. Saved set -> uses saved
        crate::config::save_server_address("192.168.1.200:3310").unwrap();
        assert_eq!(
            configured_clamd_addr().unwrap().to_string(),
            "192.168.1.200:3310"
        );

        // 3. Env var set -> env overrides saved
        unsafe {
            std::env::set_var("ZONENDPOINT_CLAMD", "100.96.250.245:3310");
        }
        assert_eq!(
            configured_clamd_addr().unwrap().to_string(),
            "100.96.250.245:3310"
        );

        // Cleanup
        unsafe {
            std::env::remove_var("ZONENDPOINT_CLAMD");
            std::env::remove_var("ZONENDPOINT_CONFIG_FILE");
        }
        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[tokio::test]
    async fn max_file_bytes_is_25_mib_and_oversize_file_is_skipped() {
        assert_eq!(MAX_FILE_BYTES, 25 * 1024 * 1024);

        let temp_dir =
            std::env::temp_dir().join(format!("zonendpoint-oversize-{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("oversize.bin");

        let file = std::fs::File::create(&file_path).unwrap();
        file.set_len(MAX_FILE_BYTES + 1).unwrap();
        drop(file);

        let cache_path = temp_dir.join("cache.db");
        let addr = "127.0.0.1:13310".parse().unwrap();
        let semaphore = Arc::new(Semaphore::new(1));
        let (_tx, cancel) = watch::channel(false);
        let result = scan_file(file_path.clone(), cache_path, addr, cancel, semaphore).await;
        assert_eq!(result.status, ScanStatus::Skipped);
        assert_eq!(
            result.reason.as_deref(),
            Some("file exceeds scan size limit")
        );

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
