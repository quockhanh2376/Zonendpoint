use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::watch,
};
use zonendpoint_lib::{
    ScanManager, cache, directory,
    scanner::{ScanLimiter, ScanStatus, clamd_version, scan_file, scan_file_with_timeout},
};

const STANDARD_VERSION: &str = "ClamAV 1.5.4/28175/Sat Nov  7 08:34:12 2026\0";

#[derive(Clone)]
enum FaultMode {
    Clean,
    ResetInstream,
    StallInstream,
    CorruptReply(Vec<u8>),
    Infected(String),
    RawReply(Vec<u8>),
    DelayedExtraReply {
        first: Vec<u8>,
        delay: Duration,
        second: Vec<u8>,
    },
    NeverClose(Vec<u8>),
    DelayedExtraVersion {
        first: Vec<u8>,
        delay: Duration,
        second: Vec<u8>,
    },
    NeverCloseVersion(Vec<u8>),
}

async fn drain_instream(socket: &mut tokio::net::TcpStream) {
    loop {
        let size = match socket.read_u32().await {
            Ok(s) => s,
            Err(_) => return,
        };
        if size == 0 {
            break;
        }
        let mut buf = vec![0u8; size as usize];
        if socket.read_exact(&mut buf).await.is_err() {
            return;
        }
    }
}

async fn spawn_mock_clamd(
    version: &'static str,
    mode: FaultMode,
) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let mode = mode.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 16];
                let mut cmd = Vec::new();
                while let Ok(n) = socket.read(&mut buf).await {
                    if n == 0 {
                        return;
                    }
                    cmd.extend_from_slice(&buf[..n]);
                    if cmd.contains(&0) {
                        break;
                    }
                }
                if cmd.starts_with(b"zVERSION\0") {
                    match &mode {
                        FaultMode::DelayedExtraVersion {
                            first,
                            delay,
                            second,
                        } => {
                            let _ = socket.write_all(first).await;
                            tokio::time::sleep(*delay).await;
                            let _ = socket.write_all(second).await;
                        }
                        FaultMode::NeverCloseVersion(reply) => {
                            let _ = socket.write_all(reply).await;
                            tokio::time::sleep(Duration::from_secs(10)).await;
                        }
                        _ => {
                            let _ = socket.write_all(version.as_bytes()).await;
                        }
                    }
                    return;
                }
                if cmd.starts_with(b"zINSTREAM\0") {
                    match mode {
                        FaultMode::ResetInstream => {
                            let mut chunk_buf = vec![0u8; 4096];
                            let _ = socket.read(&mut chunk_buf).await;
                            drop(socket);
                        }
                        FaultMode::StallInstream => {
                            tokio::time::sleep(Duration::from_secs(15)).await;
                        }
                        FaultMode::CorruptReply(reply) => {
                            drain_instream(&mut socket).await;
                            let _ = socket.write_all(&reply).await;
                        }
                        FaultMode::Clean => {
                            drain_instream(&mut socket).await;
                            let _ = socket.write_all(b"stream: OK\0").await;
                        }
                        FaultMode::Infected(name) => {
                            drain_instream(&mut socket).await;
                            let msg = format!("stream: {name} FOUND\0");
                            let _ = socket.write_all(msg.as_bytes()).await;
                        }
                        FaultMode::RawReply(reply) => {
                            drain_instream(&mut socket).await;
                            let _ = socket.write_all(&reply).await;
                        }
                        FaultMode::DelayedExtraReply {
                            first,
                            delay,
                            second,
                        } => {
                            drain_instream(&mut socket).await;
                            let _ = socket.write_all(&first).await;
                            tokio::time::sleep(delay).await;
                            let _ = socket.write_all(&second).await;
                        }
                        FaultMode::NeverClose(reply) => {
                            drain_instream(&mut socket).await;
                            let _ = socket.write_all(&reply).await;
                            tokio::time::sleep(Duration::from_secs(10)).await;
                        }
                        FaultMode::DelayedExtraVersion { .. } | FaultMode::NeverCloseVersion(_) => {
                            unreachable!("Version fault modes are handled during zVERSION");
                        }
                    }
                }
            });
        }
    });
    (addr, task)
}

use std::sync::atomic::{AtomicU64, Ordering};

static TEST_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

fn create_temp_test_dir(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let count = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "zonendpoint-resilience-{name}-{}-{count}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let target = root.join("test-file.bin");
    let cache_db = root.join("cache.db");
    (root, target, cache_db)
}

#[tokio::test]
async fn test_midstream_reset_yields_error_not_clean() {
    let (root, target, cache_db) = create_temp_test_dir("midstream-reset");
    let payload = vec![0x42u8; 1024 * 1024]; // 1 MB
    std::fs::write(&target, &payload).unwrap();

    let (addr, handle) = spawn_mock_clamd(STANDARD_VERSION, FaultMode::ResetInstream).await;
    let (_tx, cancel) = watch::channel(false);

    let result = scan_file_with_timeout(
        target,
        cache_db.clone(),
        addr,
        cancel,
        Arc::new(ScanLimiter::new(1)),
        Duration::from_secs(5),
    )
    .await;

    assert_eq!(result.status, ScanStatus::Error);
    assert_ne!(result.status, ScanStatus::Clean);
    assert!(result.reason.is_some());

    let stats = cache::stats(&cache_db).unwrap();
    assert_eq!(stats.entries, 0, "Failed scan must never be cached");

    handle.abort();
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn test_stall_timeout_yields_error_not_clean() {
    let (root, target, cache_db) = create_temp_test_dir("stall-timeout");
    std::fs::write(&target, b"stall test payload").unwrap();

    let (addr, handle) = spawn_mock_clamd(STANDARD_VERSION, FaultMode::StallInstream).await;
    let (_tx, cancel) = watch::channel(false);

    let result = scan_file_with_timeout(
        target,
        cache_db.clone(),
        addr,
        cancel,
        Arc::new(ScanLimiter::new(1)),
        Duration::from_millis(500),
    )
    .await;

    assert_eq!(result.status, ScanStatus::Error);
    assert_ne!(result.status, ScanStatus::Clean);
    let reason = result.reason.expect("Stall must report reason");
    assert!(
        reason.contains("timed out"),
        "Expected 'timed out', got: {reason}"
    );

    let stats = cache::stats(&cache_db).unwrap();
    assert_eq!(stats.entries, 0, "Timed out scan must never be cached");

    handle.abort();
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn test_malformed_truncated_responses_rejected() {
    let frames: Vec<(&'static str, Vec<u8>)> = vec![
        ("truncated", b"stream: OK".to_vec()), // No trailing NUL
        ("suffix_garbage", b"stream: OK trailing garbage\0".to_vec()),
        ("non_terminal", b"stream: OK \n FOUND\0".to_vec()),
        ("empty", b"".to_vec()), // Closes socket immediately without NUL
    ];

    for (name, frame) in frames {
        let (root, target, cache_db) = create_temp_test_dir(&format!("malformed-{name}"));
        std::fs::write(&target, b"sample file content").unwrap();

        let (addr, handle) =
            spawn_mock_clamd(STANDARD_VERSION, FaultMode::CorruptReply(frame)).await;
        let (_tx, cancel) = watch::channel(false);

        let result = scan_file_with_timeout(
            target,
            cache_db.clone(),
            addr,
            cancel,
            Arc::new(ScanLimiter::new(1)),
            Duration::from_secs(2),
        )
        .await;

        assert_eq!(
            result.status,
            ScanStatus::Error,
            "Malformed frame '{name}' must produce ScanStatus::Error"
        );
        assert_ne!(result.status, ScanStatus::Clean);

        let stats = cache::stats(&cache_db).unwrap();
        assert_eq!(
            stats.entries, 0,
            "Malformed response for '{name}' must never be cached"
        );

        handle.abort();
        let _ = std::fs::remove_dir_all(root);
    }
}

#[tokio::test]
async fn test_daemon_connection_refused_never_cached() {
    let (root, target, cache_db) = create_temp_test_dir("conn-refused");
    std::fs::write(&target, b"refused payload").unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let closed_addr = listener.local_addr().unwrap();
    drop(listener); // Immediately closed

    let (_tx, cancel) = watch::channel(false);
    let result = scan_file(
        target,
        cache_db.clone(),
        closed_addr,
        cancel,
        Arc::new(ScanLimiter::new(1)),
    )
    .await;

    assert_eq!(result.status, ScanStatus::Error);
    assert_ne!(result.status, ScanStatus::Clean);

    let stats = cache::stats(&cache_db).unwrap();
    assert_eq!(stats.entries, 0, "Refused connection must never be cached");

    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn test_cancel_during_stall_returns_cancelled() {
    let (root, target, cache_db) = create_temp_test_dir("cancel-stall");
    std::fs::write(&target, b"cancellation payload").unwrap();

    let (addr, handle) = spawn_mock_clamd(STANDARD_VERSION, FaultMode::StallInstream).await;
    let (tx, rx) = watch::channel(false);

    let cancel_tx = tx.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let _ = cancel_tx.send(true);
    });

    let semaphore = Arc::new(ScanLimiter::new(1));
    let scan_cache = cache_db.clone();
    let scan_semaphore = semaphore.clone();
    let scan_addr = addr;

    let scan: directory::ScanFn = Box::new(move |file, cancel| {
        let cache_path = scan_cache.clone();
        let sem = scan_semaphore.clone();
        Box::pin(async move {
            scan_file_with_timeout(
                file,
                cache_path,
                scan_addr,
                cancel,
                sem,
                Duration::from_secs(10),
            )
            .await
        })
    });

    let progress: directory::ProgressFn = Box::new(|_, _| {});
    let summary = directory::scan_directory(root.clone(), rx, scan, progress).await;

    assert!(summary.cancelled, "Summary must be marked cancelled");
    assert!(!summary.complete, "Summary complete must be false");
    assert_eq!(summary.clean, 0, "Clean files must be 0");
    assert!(
        summary.cancelled_files >= 1,
        "Cancelled files must be at least 1"
    );

    let stats = cache::stats(&cache_db).unwrap();
    assert_eq!(stats.entries, 0, "Cancelled scan must never be cached");

    handle.abort();
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn test_engine_signature_change_invalidates_cache_reuse() {
    let (root, target, cache_db) = create_temp_test_dir("identity-change");
    std::fs::write(&target, b"identity test content").unwrap();

    // 1. Scan 1 under version 28175 -> CLEAN
    let (addr1, handle1) = spawn_mock_clamd(
        "ClamAV 1.5.4/28175/Sat Nov  7 08:34:12 2026\0",
        FaultMode::Clean,
    )
    .await;
    let (_tx, cancel1) = watch::channel(false);
    let result1 = scan_file(
        target.clone(),
        cache_db.clone(),
        addr1,
        cancel1,
        Arc::new(ScanLimiter::new(1)),
    )
    .await;
    assert_eq!(result1.status, ScanStatus::Clean);
    let stats1 = cache::stats(&cache_db).unwrap();
    assert_eq!(stats1.entries, 1, "Initial clean scan should be cached");
    handle1.abort();

    // 2. Scan 2 with bumped version 28176, but engine encounters fault/reset
    let (addr2, handle2) = spawn_mock_clamd(
        "ClamAV 1.5.4/28176/Sat Nov  7 08:34:12 2026\0",
        FaultMode::ResetInstream,
    )
    .await;
    let (_tx, cancel2) = watch::channel(false);
    let result2 = scan_file_with_timeout(
        target.clone(),
        cache_db.clone(),
        addr2,
        cancel2,
        Arc::new(ScanLimiter::new(1)),
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(
        result2.status,
        ScanStatus::Error,
        "Signature change must bypass old cache and fail live rescan"
    );
    assert_ne!(result2.status, ScanStatus::Clean);
    handle2.abort();

    // 3. Scan 3 when engine is completely offline
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let offline_addr = listener.local_addr().unwrap();
    drop(listener);
    let (_tx, cancel3) = watch::channel(false);
    let result3 = scan_file(
        target,
        cache_db,
        offline_addr,
        cancel3,
        Arc::new(ScanLimiter::new(1)),
    )
    .await;
    assert_eq!(
        result3.status,
        ScanStatus::Error,
        "Offline engine must produce ERROR, never cached CLEAN"
    );

    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn test_pipelined_extra_response_rejected() {
    // Qeen review finding P0-01: multiple responses / pipelined frames
    let (root, target, cache_db) = create_temp_test_dir("pipelined-frames");
    std::fs::write(&target, b"pipelined frames payload").unwrap();

    let pipelined_reply = b"stream: OK\0stream: Threat.Dropper FOUND\0".to_vec();
    let (addr, handle) =
        spawn_mock_clamd(STANDARD_VERSION, FaultMode::RawReply(pipelined_reply)).await;
    let (_tx, cancel) = watch::channel(false);

    let result = scan_file_with_timeout(
        target,
        cache_db.clone(),
        addr,
        cancel,
        Arc::new(ScanLimiter::new(1)),
        Duration::from_secs(2),
    )
    .await;

    assert_eq!(
        result.status,
        ScanStatus::Error,
        "Pipelined extra data must be rejected as Error"
    );
    assert_ne!(result.status, ScanStatus::Clean);

    let stats = cache::stats(&cache_db).unwrap();
    assert_eq!(stats.entries, 0, "Pipelined response must never be cached");

    handle.abort();
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn test_stale_scan_race_dropped() {
    // Qeen review finding P0-02: Fault 7 ScanManager isolation & stale events
    let manager = ScanManager::new();

    // 1. Begin Scan A
    let _cancel_a = manager
        .begin_scan("scan-A".into())
        .expect("Scan A should begin");

    // 2. Scan B cannot begin while Scan A is active
    let err_b = manager.begin_scan("scan-B".into());
    assert!(
        err_b.is_err(),
        "Concurrent scan initiation must be rejected"
    );

    // 3. Request cancel for Scan A
    assert!(
        manager.request_cancel("scan-A"),
        "Cancel request should succeed"
    );

    // 4. Stale cancel for non-existent scan returns false
    assert!(!manager.request_cancel("scan-UNKNOWN"));

    // 5. Finish Scan A
    let cancelled = manager.finish_scan("scan-A").expect("Finish scan-A");
    assert!(cancelled, "Scan A must finish in cancelled state");

    // 6. Now Scan B can begin
    let _cancel_b = manager
        .begin_scan("scan-B".into())
        .expect("Scan B should begin");

    // 7. Stale finish from Scan A after Scan B started must not disrupt Scan B
    let stale_finish = manager.finish_scan("scan-A").expect("Stale finish call");
    assert!(!stale_finish, "Stale scan-A finish must return false");

    // Verify Scan B is still the active scan
    assert_eq!(manager.active_scan_id(), Some("scan-B".into()));
}

#[tokio::test]
async fn test_incomplete_version_response_rejected() {
    // Qeen review finding P1-01: version without signature info
    let (root, target, cache_db) = create_temp_test_dir("incomplete-version");
    std::fs::write(&target, b"version check payload").unwrap();

    let incomplete_version = "ClamAV 1.5.4\0";
    let (addr, handle) = spawn_mock_clamd(incomplete_version, FaultMode::Clean).await;
    let (_tx, cancel) = watch::channel(false);

    // Direct version check fails
    let v_result = clamd_version(addr, cancel.clone()).await;
    assert!(
        v_result.is_err(),
        "Version without signature component must be rejected"
    );

    // File scan using this daemon also fails
    let result = scan_file(
        target,
        cache_db.clone(),
        addr,
        cancel,
        Arc::new(ScanLimiter::new(1)),
    )
    .await;
    assert_eq!(result.status, ScanStatus::Error);
    assert_ne!(result.status, ScanStatus::Clean);

    let stats = cache::stats(&cache_db).unwrap();
    assert_eq!(
        stats.entries, 0,
        "Scan with incomplete engine identity must never cache"
    );

    handle.abort();
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn test_clamd_stream_limit_error_handled() {
    // Qeen review finding P1-03: INSTREAM size limit exceeded. ERROR
    let (root, target, cache_db) = create_temp_test_dir("stream-limit-exceeded");
    std::fs::write(&target, b"large stream payload").unwrap();

    let limit_reply = b"INSTREAM size limit exceeded. ERROR\0".to_vec();
    let (addr, handle) = spawn_mock_clamd(STANDARD_VERSION, FaultMode::RawReply(limit_reply)).await;
    let (_tx, cancel) = watch::channel(false);

    let result = scan_file_with_timeout(
        target,
        cache_db.clone(),
        addr,
        cancel,
        Arc::new(ScanLimiter::new(1)),
        Duration::from_secs(2),
    )
    .await;

    assert_eq!(result.status, ScanStatus::Error);
    assert_ne!(result.status, ScanStatus::Clean);
    let reason = result.reason.expect("Expected reason for limit error");
    assert!(
        reason.contains("INSTREAM size limit exceeded"),
        "Expected stream limit reason, got: {reason}"
    );

    let stats = cache::stats(&cache_db).unwrap();
    assert_eq!(stats.entries, 0);

    handle.abort();
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn test_infected_preserves_threat_name() {
    // Qeen review finding P2-02: Threat name preserved in reason
    let (root, target, cache_db) = create_temp_test_dir("infected-name");
    std::fs::write(&target, b"threat test payload").unwrap();

    let (addr, handle) = spawn_mock_clamd(
        STANDARD_VERSION,
        FaultMode::Infected("Win.Test.CustomThreat-42".into()),
    )
    .await;
    let (_tx, cancel) = watch::channel(false);

    let result = scan_file_with_timeout(
        target,
        cache_db.clone(),
        addr,
        cancel,
        Arc::new(ScanLimiter::new(1)),
        Duration::from_secs(2),
    )
    .await;

    assert_eq!(result.status, ScanStatus::Infected);
    assert_eq!(result.reason, Some("Win.Test.CustomThreat-42".into()));

    handle.abort();
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn test_delayed_second_frame_50ms_rejected() {
    let (root, target, cache_db) = create_temp_test_dir("delayed-50ms");
    std::fs::write(&target, b"delayed 50ms payload").unwrap();

    let (addr, handle) = spawn_mock_clamd(
        STANDARD_VERSION,
        FaultMode::DelayedExtraReply {
            first: b"stream: OK\0".to_vec(),
            delay: Duration::from_millis(50),
            second: b"stream: Threat.Delayed50ms FOUND\0".to_vec(),
        },
    )
    .await;
    let (_tx, cancel) = watch::channel(false);

    let result = scan_file_with_timeout(
        target,
        cache_db.clone(),
        addr,
        cancel,
        Arc::new(ScanLimiter::new(1)),
        Duration::from_secs(5),
    )
    .await;

    assert_eq!(
        result.status,
        ScanStatus::Error,
        "50ms delayed second frame must be rejected as Error"
    );
    assert_ne!(result.status, ScanStatus::Clean);

    let stats = cache::stats(&cache_db).unwrap();
    assert_eq!(
        stats.entries, 0,
        "50ms delayed response must never be cached"
    );

    handle.abort();
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn test_delayed_second_frame_500ms_rejected() {
    let (root, target, cache_db) = create_temp_test_dir("delayed-500ms");
    std::fs::write(&target, b"delayed 500ms payload").unwrap();

    let (addr, handle) = spawn_mock_clamd(
        STANDARD_VERSION,
        FaultMode::DelayedExtraReply {
            first: b"stream: OK\0".to_vec(),
            delay: Duration::from_millis(500),
            second: b"stream: Threat.Delayed500ms FOUND\0".to_vec(),
        },
    )
    .await;
    let (_tx, cancel) = watch::channel(false);

    let result = scan_file_with_timeout(
        target,
        cache_db.clone(),
        addr,
        cancel,
        Arc::new(ScanLimiter::new(1)),
        Duration::from_secs(5),
    )
    .await;

    assert_eq!(
        result.status,
        ScanStatus::Error,
        "500ms delayed second frame must be rejected as Error"
    );
    assert_ne!(result.status, ScanStatus::Clean);

    let stats = cache::stats(&cache_db).unwrap();
    assert_eq!(
        stats.entries, 0,
        "500ms delayed response must never be cached"
    );

    handle.abort();
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn test_response_without_eof_never_closed_rejected() {
    let (root, target, cache_db) = create_temp_test_dir("never-closed");
    std::fs::write(&target, b"never closed payload").unwrap();

    let (addr, handle) = spawn_mock_clamd(
        STANDARD_VERSION,
        FaultMode::NeverClose(b"stream: OK\0".to_vec()),
    )
    .await;
    let (_tx, cancel) = watch::channel(false);

    let result = scan_file_with_timeout(
        target,
        cache_db.clone(),
        addr,
        cancel,
        Arc::new(ScanLimiter::new(1)),
        Duration::from_secs(5),
    )
    .await;

    assert_eq!(
        result.status,
        ScanStatus::Error,
        "Response without EOF must time out and be rejected as Error"
    );
    assert_ne!(result.status, ScanStatus::Clean);

    let stats = cache::stats(&cache_db).unwrap();
    assert_eq!(
        stats.entries, 0,
        "Never-closed response must never be cached"
    );

    handle.abort();
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn test_preflight_delayed_extra_frame_rejected() {
    let (addr, handle) = spawn_mock_clamd(
        STANDARD_VERSION,
        FaultMode::DelayedExtraVersion {
            first: b"ClamAV 1.5.4/28175/Sat Nov  7 08:34:12 2026\0".to_vec(),
            delay: Duration::from_millis(50),
            second: b"EXTRA_FRAME\0".to_vec(),
        },
    )
    .await;
    let (_tx, cancel) = watch::channel(false);

    let result = clamd_version(addr, cancel).await;
    assert!(
        result.is_err(),
        "Preflight with delayed extra frame must be rejected"
    );

    handle.abort();
}

#[tokio::test]
async fn test_preflight_without_eof_rejected() {
    let (addr, handle) = spawn_mock_clamd(
        STANDARD_VERSION,
        FaultMode::NeverCloseVersion(b"ClamAV 1.5.4/28175/Sat Nov  7 08:34:12 2026\0".to_vec()),
    )
    .await;
    let (_tx, cancel) = watch::channel(false);

    let result = clamd_version(addr, cancel).await;
    assert!(
        result.is_err(),
        "Preflight without EOF close must be rejected"
    );

    handle.abort();
}

#[tokio::test]
async fn test_infected_cache_hit_preserves_exact_threat_name() {
    let (root, target, cache_db) = create_temp_test_dir("infected-cache-hit-name");
    std::fs::write(&target, b"payload with threat").unwrap();

    let threat_name = "YARA.ZonQA_Benign_Marker.UNOFFICIAL";
    let (addr, handle) =
        spawn_mock_clamd(STANDARD_VERSION, FaultMode::Infected(threat_name.into())).await;
    let (_tx, cancel) = watch::channel(false);

    // Scan 1: Live scan from clamd daemon
    let result1 = scan_file_with_timeout(
        target.clone(),
        cache_db.clone(),
        addr,
        cancel,
        Arc::new(ScanLimiter::new(1)),
        Duration::from_secs(2),
    )
    .await;

    assert_eq!(result1.status, ScanStatus::Infected);
    assert_eq!(result1.reason, Some(threat_name.into()));

    // Verify cache recorded 1 infected entry
    let stats = cache::stats(&cache_db).unwrap();
    assert_eq!(stats.entries, 1, "Infected scan must be recorded in cache");

    // Scan 2: Cache hit on the same file -> must preserve the exact threat name!
    let (_tx2, cancel2) = watch::channel(false);
    let result2 = scan_file_with_timeout(
        target.clone(),
        cache_db.clone(),
        addr,
        cancel2,
        Arc::new(ScanLimiter::new(1)),
        Duration::from_secs(2),
    )
    .await;

    assert_eq!(result2.status, ScanStatus::Infected);
    assert_eq!(
        result2.reason,
        Some(threat_name.into()),
        "Cache hit on INFECTED file must return the exact threat name, not generic cache hit message"
    );

    handle.abort();
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn test_clean_cache_hit_preserves_verified_content_cache_hit_message() {
    let (root, target, cache_db) = create_temp_test_dir("clean-cache-hit-msg");
    std::fs::write(&target, b"clean payload bytes").unwrap();

    let (addr, handle) = spawn_mock_clamd(STANDARD_VERSION, FaultMode::Clean).await;
    let (_tx, cancel) = watch::channel(false);

    // Scan 1: Live scan
    let result1 = scan_file_with_timeout(
        target.clone(),
        cache_db.clone(),
        addr,
        cancel,
        Arc::new(ScanLimiter::new(1)),
        Duration::from_secs(2),
    )
    .await;

    assert_eq!(result1.status, ScanStatus::Clean);

    // Scan 2: Cache hit -> must preserve "verified content cache hit"
    let (_tx2, cancel2) = watch::channel(false);
    let result2 = scan_file_with_timeout(
        target.clone(),
        cache_db.clone(),
        addr,
        cancel2,
        Arc::new(ScanLimiter::new(1)),
        Duration::from_secs(2),
    )
    .await;

    assert_eq!(result2.status, ScanStatus::Clean);
    assert_eq!(
        result2.reason,
        Some("verified content cache hit".into()),
        "Cache hit on CLEAN file must preserve verified content cache hit message"
    );

    handle.abort();
    let _ = std::fs::remove_dir_all(root);
}
