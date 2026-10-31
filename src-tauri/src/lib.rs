mod cache;
mod directory;
pub mod scanner;
mod watcher;

use directory::{ProgressFn, ScanFn, ScanSummary};
use serde::Serialize;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::{Semaphore, watch};
use watcher::Guard;

struct ScanManager {
    active: Mutex<HashMap<String, watch::Sender<bool>>>,
    selected: Mutex<HashMap<String, (Instant, PathBuf)>>,
    semaphore: Arc<Semaphore>,
}

impl ScanManager {
    fn new() -> Self {
        Self {
            active: Mutex::new(HashMap::new()),
            selected: Mutex::new(HashMap::new()),
            semaphore: Arc::new(Semaphore::new(2)),
        }
    }
}

#[derive(Clone, Serialize)]
struct ScanProgress {
    scan_id: String,
    scanned: usize,
    clean: usize,
    infected: usize,
    unknown: usize,
    errors: usize,
    skipped: usize,
    cancelled_files: usize,
    latest: scanner::ScanResult,
}

#[derive(Serialize)]
struct SelectedTarget {
    id: String,
    path: String,
}

impl ScanProgress {
    fn new(scan_id: String, summary: &ScanSummary, latest: scanner::ScanResult) -> Self {
        Self {
            scan_id,
            scanned: summary.scanned,
            clean: summary.clean,
            infected: summary.infected,
            unknown: summary.unknown,
            errors: summary.errors,
            skipped: summary.skipped,
            cancelled_files: summary.cancelled_files,
            latest,
        }
    }
}

#[tauri::command]
fn select_scan_path(
    app: tauri::AppHandle,
    manager: tauri::State<'_, ScanManager>,
    kind: String,
) -> Result<Option<SelectedTarget>, String> {
    let selected = match kind.as_str() {
        "file" => app.dialog().file().blocking_pick_file(),
        "folder" => app.dialog().file().blocking_pick_folder(),
        _ => return Err("selection kind must be file or folder".into()),
    };
    let Some(path) = selected else {
        return Ok(None);
    };
    let path = path.into_path().map_err(|error| error.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();
    let mut selections = manager
        .selected
        .lock()
        .map_err(|_| "selection state is unavailable")?;
    selections.retain(|_, (created, _)| created.elapsed() < Duration::from_secs(300));
    if selections.len() >= 8 {
        selections.clear();
    }
    selections.insert(id.clone(), (Instant::now(), path.clone()));
    Ok(Some(SelectedTarget {
        id,
        path: path.to_string_lossy().into_owned(),
    }))
}

#[tauri::command]
async fn scan_path(
    app: tauri::AppHandle,
    manager: tauri::State<'_, ScanManager>,
    scan_id: String,
    target_id: String,
) -> Result<ScanSummary, String> {
    if scan_id.is_empty() || scan_id.len() > 80 {
        return Err("invalid scan identifier".into());
    }
    let target = {
        let mut selections = manager
            .selected
            .lock()
            .map_err(|_| "selection state is unavailable")?;
        let (created, path) = selections
            .remove(&target_id)
            .ok_or("target must be selected in the native picker")?;
        if created.elapsed() >= Duration::from_secs(300) {
            return Err("file selection expired; choose it again".into());
        }
        path
    };
    let (cancel_tx, cancel_rx) = watch::channel(false);
    {
        let mut active = manager
            .active
            .lock()
            .map_err(|_| "scan state is unavailable")?;
        if active.contains_key(&scan_id) {
            return Err("scan identifier is already active".into());
        }
        active.insert(scan_id.clone(), cancel_tx);
    }
    let result = async {
        let cache_path = cache::default_path().map_err(|error| error.to_string())?;
        let addr = scanner::configured_clamd_addr();
        let semaphore = manager.semaphore.clone();
        let scan_addr = addr.clone();
        let scan_cache = cache_path.clone();
        let scan_semaphore = semaphore.clone();
        let scan: ScanFn = Box::new(move |file, cancel| {
            let cache_path = scan_cache.clone();
            let semaphore = scan_semaphore.clone();
            let addr = scan_addr.clone();
            Box::pin(async move {
                match addr {
                    Ok(addr) => scanner::scan_file(file, cache_path, addr, cancel, semaphore).await,
                    Err(reason) => scanner::ScanResult::error(file, reason),
                }
            })
        });
        let progress_app = app.clone();
        let progress_id = scan_id.clone();
        let progress: ProgressFn = Box::new(move |summary, latest| {
            let _ = progress_app.emit(
                "scan-progress",
                ScanProgress::new(progress_id.clone(), summary, latest.clone()),
            );
        });

        match tokio::fs::symlink_metadata(&target).await {
            Ok(metadata) if metadata.is_dir() => {
                Ok(directory::scan_directory(target, cancel_rx.clone(), scan, progress).await)
            }
            _ => {
                let result = match addr {
                    Ok(addr) => {
                        scanner::scan_file(target, cache_path, addr, cancel_rx.clone(), semaphore)
                            .await
                    }
                    Err(reason) => scanner::ScanResult::error(&target, reason),
                };
                let mut summary = ScanSummary::default();
                summary.record(result.clone());
                let _ = app.emit(
                    "scan-progress",
                    ScanProgress::new(scan_id.clone(), &summary, result),
                );
                Ok(summary)
            }
        }
    }
    .await;
    if let Ok(mut active) = manager.active.lock() {
        active.remove(&scan_id);
    }
    result
}

#[tauri::command]
fn cancel_scan(manager: tauri::State<'_, ScanManager>, scan_id: String) -> bool {
    manager
        .active
        .lock()
        .ok()
        .and_then(|active| active.get(&scan_id).cloned())
        .is_some_and(|sender| sender.send(true).is_ok())
}

#[tauri::command]
fn cache_stats() -> Result<cache::CacheStats, String> {
    let path = cache::default_path().map_err(|error| error.to_string())?;
    cache::stats(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_guard(
    app: tauri::AppHandle,
    manager: tauri::State<'_, ScanManager>,
    enabled: bool,
) -> Result<bool, String> {
    let state = app.state::<Arc<Mutex<Guard>>>();
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    if enabled {
        let cache_path = cache::default_path().map_err(|error| error.to_string())?;
        let addr = scanner::configured_clamd_addr()?;
        guard
            .start(cache_path, addr, manager.semaphore.clone())
            .map_err(|e| e.to_string())?;
    } else {
        guard.stop();
    }
    Ok(guard.is_running())
}

pub async fn scan_cli(path: PathBuf) -> scanner::ScanResult {
    let cache_path = match cache::default_path() {
        Ok(path) => path,
        Err(error) => return scanner::ScanResult::error(path, error.to_string()),
    };
    let (_cancel, receiver) = watch::channel(false);
    match scanner::configured_clamd_addr() {
        Ok(addr) => {
            scanner::scan_file(
                path,
                cache_path,
                addr,
                receiver,
                Arc::new(Semaphore::new(2)),
            )
            .await
        }
        Err(reason) => scanner::ScanResult::error(path, reason),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(ScanManager::new())
        .manage(Arc::new(Mutex::new(Guard::default())))
        .invoke_handler(tauri::generate_handler![
            select_scan_path,
            scan_path,
            cancel_scan,
            cache_stats,
            set_guard
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
