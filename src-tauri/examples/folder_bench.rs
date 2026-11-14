//! Development-only scan benchmark. Example binaries are not bundled in releases.
//! Set LOCALAPPDATA to an isolated directory for every cold-cache run.

use std::{
    env,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::watch;
use zonendpoint_lib::{cache, config, directory, quick, scanner};

#[derive(serde::Serialize)]
struct FileMetric {
    path: String,
    bytes: u64,
    elapsed_ms: u64,
}

#[derive(serde::Serialize, Clone, Debug, Default)]
struct StageSummary {
    p50_ms: f64,
    p95_ms: f64,
    max_ms: f64,
    total_ms: f64,
}

#[derive(serde::Serialize, Clone, Debug, Default)]
struct StagesReport {
    wait_limiter: StageSummary,
    open_and_hash: StageSummary,
    cache_lookup: StageSummary,
    tcp_connect: StageSummary,
    send_instream: StageSummary,
    clamd_wait: StageSummary,
    cache_write: StageSummary,
}

#[derive(serde::Serialize)]
struct Report {
    started_utc_unix_ms: u128,
    elapsed_ms: u128,
    process_cpu_seconds: f64,
    peak_working_set_bytes: usize,
    scanned: usize,
    clean: usize,
    infected: usize,
    unknown: usize,
    errors: usize,
    skipped: usize,
    cancelled: usize,
    complete: bool,
    cache_entries: i64,
    concurrency_requested: usize,
    concurrency_final: usize,
    backoffs: usize,
    recoveries: usize,
    cancel_latency_ms: Option<u128>,
    file_time_p50_ms: u64,
    file_time_p95_ms: u64,
    file_time_p99_ms: u64,
    file_time_max_ms: u64,
    slowest_files: Vec<FileMetric>,
    stages: StagesReport,
}

#[repr(C)]
#[derive(Default)]
struct FileTime {
    low: u32,
    high: u32,
}

#[repr(C)]
#[derive(Default)]
struct ProcessMemoryCounters {
    cb: u32,
    page_fault_count: u32,
    peak_working_set_size: usize,
    working_set_size: usize,
    quota_peak_paged_pool_usage: usize,
    quota_paged_pool_usage: usize,
    quota_peak_non_paged_pool_usage: usize,
    quota_non_paged_pool_usage: usize,
    pagefile_usage: usize,
    peak_pagefile_usage: usize,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> *mut std::ffi::c_void;
    fn GetProcessTimes(
        process: *mut std::ffi::c_void,
        creation: *mut FileTime,
        exit: *mut FileTime,
        kernel: *mut FileTime,
        user: *mut FileTime,
    ) -> i32;
}
#[link(name = "psapi")]
unsafe extern "system" {
    fn GetProcessMemoryInfo(
        process: *mut std::ffi::c_void,
        counters: *mut ProcessMemoryCounters,
        size: u32,
    ) -> i32;
}

fn filetime_seconds(time: &FileTime) -> f64 {
    (((u64::from(time.high)) << 32) | u64::from(time.low)) as f64 / 10_000_000.0
}

fn process_metrics() -> (f64, usize) {
    let process = unsafe { GetCurrentProcess() };
    let mut creation = FileTime::default();
    let mut exit = FileTime::default();
    let mut kernel = FileTime::default();
    let mut user = FileTime::default();
    let cpu =
        if unsafe { GetProcessTimes(process, &mut creation, &mut exit, &mut kernel, &mut user) }
            != 0
        {
            filetime_seconds(&kernel) + filetime_seconds(&user)
        } else {
            0.0
        };
    let mut memory = ProcessMemoryCounters {
        cb: std::mem::size_of::<ProcessMemoryCounters>() as u32,
        ..Default::default()
    };
    let peak = if unsafe { GetProcessMemoryInfo(process, &mut memory, memory.cb) } != 0 {
        memory.peak_working_set_size
    } else {
        0
    };
    (cpu, peak)
}

fn summarize_stage(
    records: &[scanner::StageTimings],
    getter: impl Fn(&scanner::StageTimings) -> u64,
) -> StageSummary {
    if records.is_empty() {
        return StageSummary::default();
    }
    let mut values_us: Vec<u64> = records.iter().map(getter).collect();
    let total_us: u64 = values_us.iter().sum();
    values_us.sort_unstable();
    let n = values_us.len();
    let p50_us = values_us[(n * 50).div_ceil(100).saturating_sub(1)];
    let p95_us = values_us[(n * 95).div_ceil(100).saturating_sub(1)];
    let max_us = *values_us.last().unwrap_or(&0);
    StageSummary {
        p50_ms: (p50_us as f64) / 1000.0,
        p95_ms: (p95_us as f64) / 1000.0,
        max_ms: (max_us as f64) / 1000.0,
        total_ms: (total_us as f64) / 1000.0,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let mode = args
        .next()
        .ok_or("usage: folder_bench <folder|--quick> [--concurrency N] [--cancel-after SECONDS]")?;
    let quick_mode = mode == "--quick";
    let root = if quick_mode {
        quick::system_root()?
    } else {
        PathBuf::from(mode)
    };
    let mut concurrency = config::scan_concurrency();
    let mut cancel_after = None;
    while let Some(arg) = args.next() {
        match arg.to_string_lossy().as_ref() {
            "--concurrency" => {
                concurrency = args
                    .next()
                    .ok_or("missing concurrency")?
                    .to_string_lossy()
                    .parse()?
            }
            "--cancel-after" => {
                cancel_after = Some(
                    args.next()
                        .ok_or("missing cancellation seconds")?
                        .to_string_lossy()
                        .parse::<u64>()?,
                )
            }
            _ => return Err("unknown argument".into()),
        }
    }
    if !(1..=6).contains(&concurrency) {
        return Err("concurrency must be between 1 and 6".into());
    }
    let cache_path = cache::default_path()?;
    let addr = scanner::configured_clamd_addr()?;
    let (_cancel_tx, cancel) = watch::channel(false);
    let engine = scanner::clamd_preflight(addr, cancel.clone()).await?;
    let limiter = Arc::new(scanner::ScanLimiter::new(concurrency));
    let file_metrics = Arc::new(Mutex::new(Vec::new()));
    let stage_metrics = Arc::new(Mutex::new(Vec::new()));
    let cache_session = Arc::new(cache::CacheSession::open(&cache_path)?);
    let scan_session = cache_session.clone();
    let scan_limiter = limiter.clone();
    let scan_engine = engine;
    let scan_addr = addr;
    let scan_times = file_metrics.clone();
    let scan_stages = stage_metrics.clone();
    let scan_tracker = Arc::new(scanner::EngineTracker::new(scan_engine.clone()));
    let scan: directory::ScanFn = Box::new(move |path, cancel| {
        let session = scan_session.clone();
        let limiter = scan_limiter.clone();
        let tracker = scan_tracker.clone();
        let times = scan_times.clone();
        let stages = scan_stages.clone();
        Box::pin(async move {
            let started = Instant::now();
            let result = scanner::scan_file_with_tracker(
                path,
                Some(session),
                scan_addr,
                tracker,
                cancel,
                limiter,
            )
            .await;
            if let Some(st) = result.stages {
                stages.lock().unwrap().push(st);
            }
            times.lock().unwrap().push(FileMetric {
                path: result.path.clone(),
                bytes: result.bytes_scanned,
                elapsed_ms: started.elapsed().as_millis().min(u64::MAX as u128) as u64,
            });
            result
        })
    });
    let targets = if quick_mode {
        quick::targets_for_root(&root)
    } else {
        vec![root.clone()]
    };
    let exclusions = if quick_mode {
        quick::exclusions_for_root(&root)
    } else {
        Vec::new()
    };
    let (cancel_tx, cancel_rx) = watch::channel(false);
    let started_utc_unix_ms = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let started = Instant::now();
    let trigger = cancel_after.map(|seconds| {
        let cancel_tx = cancel_tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(seconds)).await;
            let cancelled_at = Instant::now();
            let _ = cancel_tx.send(true);
            cancelled_at
        })
    });
    let summary = directory::scan_roots_with_exclusions(
        targets,
        exclusions,
        cancel_rx,
        scan,
        Box::new(|_, _| {}),
    )
    .await;
    let _ = cache_session.flush().await;
    let elapsed_ms = started.elapsed().as_millis();
    let cancel_latency_ms = if let Some(trigger) = trigger {
        if trigger.is_finished() {
            Some(trigger.await?.elapsed().as_millis())
        } else {
            trigger.abort();
            None
        }
    } else {
        None
    };
    let (process_cpu_seconds, peak_working_set_bytes) = process_metrics();
    let (concurrency_final, backoffs, recoveries) = limiter.telemetry();
    let stats = cache::stats(&cache_path)?;
    let mut file_metrics = file_metrics.lock().unwrap().drain(..).collect::<Vec<_>>();
    file_metrics.sort_by_key(|item| item.elapsed_ms);
    let percentile = |p: usize| -> u64 {
        if file_metrics.is_empty() {
            return 0;
        }
        file_metrics[(file_metrics.len() * p).div_ceil(100).saturating_sub(1)].elapsed_ms
    };
    let file_time_p50_ms = percentile(50);
    let file_time_p95_ms = percentile(95);
    let file_time_p99_ms = percentile(99);
    let file_time_max_ms = file_metrics.last().map_or(0, |item| item.elapsed_ms);
    file_metrics.sort_by_key(|item| std::cmp::Reverse(item.elapsed_ms));
    file_metrics.truncate(10);
    let stage_records = stage_metrics.lock().unwrap().drain(..).collect::<Vec<_>>();
    let stages = StagesReport {
        wait_limiter: summarize_stage(&stage_records, |s| s.wait_limiter_us),
        open_and_hash: summarize_stage(&stage_records, |s| s.open_and_hash_us),
        cache_lookup: summarize_stage(&stage_records, |s| s.cache_lookup_us),
        tcp_connect: summarize_stage(&stage_records, |s| s.tcp_connect_us),
        send_instream: summarize_stage(&stage_records, |s| s.send_instream_us),
        clamd_wait: summarize_stage(&stage_records, |s| s.clamd_wait_us),
        cache_write: summarize_stage(&stage_records, |s| s.cache_write_us),
    };
    eprintln!("--- STAGE TIMINGS BREAKDOWN (ms) ---");
    eprintln!(
        "{:<16} | {:>10} | {:>10} | {:>10} | {:>10}",
        "Stage", "Total (ms)", "p50 (ms)", "p95 (ms)", "Max (ms)"
    );
    eprintln!(
        "{:<16} | {:>10.2} | {:>10.2} | {:>10.2} | {:>10.2}",
        "wait_limiter",
        stages.wait_limiter.total_ms,
        stages.wait_limiter.p50_ms,
        stages.wait_limiter.p95_ms,
        stages.wait_limiter.max_ms
    );
    eprintln!(
        "{:<16} | {:>10.2} | {:>10.2} | {:>10.2} | {:>10.2}",
        "open_and_hash",
        stages.open_and_hash.total_ms,
        stages.open_and_hash.p50_ms,
        stages.open_and_hash.p95_ms,
        stages.open_and_hash.max_ms
    );
    eprintln!(
        "{:<16} | {:>10.2} | {:>10.2} | {:>10.2} | {:>10.2}",
        "cache_lookup",
        stages.cache_lookup.total_ms,
        stages.cache_lookup.p50_ms,
        stages.cache_lookup.p95_ms,
        stages.cache_lookup.max_ms
    );
    eprintln!(
        "{:<16} | {:>10.2} | {:>10.2} | {:>10.2} | {:>10.2}",
        "tcp_connect",
        stages.tcp_connect.total_ms,
        stages.tcp_connect.p50_ms,
        stages.tcp_connect.p95_ms,
        stages.tcp_connect.max_ms
    );
    eprintln!(
        "{:<16} | {:>10.2} | {:>10.2} | {:>10.2} | {:>10.2}",
        "send_instream",
        stages.send_instream.total_ms,
        stages.send_instream.p50_ms,
        stages.send_instream.p95_ms,
        stages.send_instream.max_ms
    );
    eprintln!(
        "{:<16} | {:>10.2} | {:>10.2} | {:>10.2} | {:>10.2}",
        "clamd_wait",
        stages.clamd_wait.total_ms,
        stages.clamd_wait.p50_ms,
        stages.clamd_wait.p95_ms,
        stages.clamd_wait.max_ms
    );
    eprintln!(
        "{:<16} | {:>10.2} | {:>10.2} | {:>10.2} | {:>10.2}",
        "cache_write",
        stages.cache_write.total_ms,
        stages.cache_write.p50_ms,
        stages.cache_write.p95_ms,
        stages.cache_write.max_ms
    );
    println!(
        "{}",
        serde_json::to_string(&Report {
            started_utc_unix_ms,
            elapsed_ms,
            process_cpu_seconds,
            peak_working_set_bytes,
            scanned: summary.scanned,
            clean: summary.clean,
            infected: summary.infected,
            unknown: summary.unknown,
            errors: summary.errors,
            skipped: summary.skipped,
            cancelled: summary.cancelled_files,
            complete: summary.complete,
            cache_entries: stats.entries,
            concurrency_requested: concurrency,
            concurrency_final,
            backoffs,
            recoveries,
            cancel_latency_ms,
            file_time_p50_ms,
            file_time_p95_ms,
            file_time_p99_ms,
            file_time_max_ms,
            slowest_files: file_metrics,
            stages,
        })?
    );
    Ok(())
}
