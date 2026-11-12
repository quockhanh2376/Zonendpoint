//! Development-only folder benchmark. Run from a PowerShell session with
//! LOCALAPPDATA set to an isolated directory for each cache-cold run.

use std::{env, path::PathBuf, sync::Arc, time::Instant};
use tokio::sync::watch;
use zonendpoint_lib::{cache, directory, scanner};

#[derive(serde::Serialize)]
struct Report {
    elapsed_seconds: f64,
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
    cache: String,
    cache_entries: i64,
    concurrency_final: usize,
    backoffs: usize,
    recoveries: usize,
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
    let ticks = (u64::from(time.high) << 32) | u64::from(time.low);
    ticks as f64 / 10_000_000.0
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let root = args
        .next()
        .map(PathBuf::from)
        .ok_or("usage: cargo run --release --example folder_bench -- <folder> <concurrency>")?;
    let concurrency: usize = args
        .next()
        .ok_or("missing concurrency level (1-6)")?
        .to_string_lossy()
        .parse()?;
    if !(1..=6).contains(&concurrency) {
        return Err("concurrency must be between 1 and 6".into());
    }
    let cache_path = cache::default_path()?;
    let addr = scanner::configured_clamd_addr()?;
    let (_cancel_tx, cancel) = watch::channel(false);
    let engine = scanner::clamd_preflight(addr, cancel.clone()).await?;
    let limiter = Arc::new(scanner::ScanLimiter::new(concurrency));
    let scan_cache = cache_path.clone();
    let scan_limiter = limiter.clone();
    let scan_engine = engine.clone();
    let scan_addr = addr;
    let scan: directory::ScanFn = Box::new(move |path, cancel| {
        let cache_path = scan_cache.clone();
        let limiter = scan_limiter.clone();
        let engine = scan_engine.clone();
        Box::pin(async move {
            scanner::scan_file_with_engine_override(
                path,
                cache_path,
                scan_addr,
                Some(engine),
                cancel,
                limiter,
            )
            .await
        })
    });
    let started = Instant::now();
    let summary = directory::scan_roots(vec![root], cancel, scan, Box::new(|_, _| {})).await;
    let elapsed_seconds = started.elapsed().as_secs_f64();
    let (process_cpu_seconds, peak_working_set_bytes) = process_metrics();
    let (concurrency_final, backoffs, recoveries) = limiter.telemetry();
    let cache = cache::stats(&cache_path)?;
    println!(
        "{}",
        serde_json::to_string(&Report {
            elapsed_seconds,
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
            cache: cache_path.to_string_lossy().into_owned(),
            cache_entries: cache.entries,
            concurrency_final,
            backoffs,
            recoveries,
        })?
    );
    Ok(())
}
