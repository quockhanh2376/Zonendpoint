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
    active: Mutex<Option<ActiveScan>>,
    selected: Mutex<HashMap<String, (Instant, PathBuf)>>,
    semaphore: Arc<Semaphore>,
}

struct ActiveScan {
    scan_id: String,
    cancel: watch::Sender<bool>,
    state: ScanLifecycle,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum ScanLifecycle {
    Scanning,
    Cancelling,
    Idle,
}

#[derive(Clone, Serialize)]
struct ScanStateEvent {
    scan_id: String,
    state: ScanLifecycle,
}

impl ScanManager {
    fn new() -> Self {
        Self {
            active: Mutex::new(None),
            selected: Mutex::new(HashMap::new()),
            semaphore: Arc::new(Semaphore::new(2)),
        }
    }

    fn begin_scan(&self, scan_id: String) -> Result<watch::Receiver<bool>, String> {
        let mut active = self
            .active
            .lock()
            .map_err(|_| "scan state is unavailable")?;
        if active.is_some() {
            return Err("another scan is still active".into());
        }
        let (cancel, receiver) = watch::channel(false);
        *active = Some(ActiveScan {
            scan_id,
            cancel,
            state: ScanLifecycle::Scanning,
        });
        Ok(receiver)
    }

    fn selected_path(&self, target_id: &str) -> Result<PathBuf, String> {
        let mut selected = self
            .selected
            .lock()
            .map_err(|_| "selection state is unavailable")?;
        let Some((created, path)) = selected.get(target_id) else {
            return Err("target must be selected in the native picker".into());
        };
        if created.elapsed() >= Duration::from_secs(300) {
            selected.remove(target_id);
            return Err("file selection expired; choose it again".into());
        }
        Ok(path.clone())
    }

    fn request_cancel(&self, scan_id: &str) -> bool {
        let Ok(mut active) = self.active.lock() else {
            return false;
        };
        let Some(active) = active.as_mut() else {
            return false;
        };
        if active.scan_id != scan_id || active.state != ScanLifecycle::Scanning {
            return false;
        }
        if active.cancel.send(true).is_err() {
            return false;
        }
        active.state = ScanLifecycle::Cancelling;
        true
    }

    fn finish_scan(&self, scan_id: &str) -> Result<bool, String> {
        let mut active = self
            .active
            .lock()
            .map_err(|_| "scan state is unavailable")?;
        if active.as_ref().is_some_and(|scan| scan.scan_id == scan_id) {
            Ok(active
                .take()
                .is_some_and(|scan| scan.state == ScanLifecycle::Cancelling))
        } else {
            Ok(false)
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
    let target = manager.selected_path(&target_id)?;
    let cancel_rx = manager.begin_scan(scan_id.clone())?;
    let _ = app.emit(
        "scan-state",
        ScanStateEvent {
            scan_id: scan_id.clone(),
            state: ScanLifecycle::Scanning,
        },
    );
    let cancellation_path = target.clone();
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
    let cancelled = manager.finish_scan(&scan_id)?;
    let result = match (result, cancelled) {
        (Ok(mut summary), true) => {
            mark_user_cancelled(&mut summary, &cancellation_path);
            Ok(summary)
        }
        (Err(_), true) => {
            let mut summary = ScanSummary::default();
            summary.record(scanner::ScanResult::new(
                &cancellation_path,
                scanner::ScanStatus::Cancelled,
                None,
                Some("cancelled by user".into()),
                0,
            ));
            Ok(summary)
        }
        (result, false) => result,
    };
    let _ = app.emit(
        "scan-state",
        ScanStateEvent {
            scan_id,
            state: ScanLifecycle::Idle,
        },
    );
    result
}

fn mark_user_cancelled(summary: &mut ScanSummary, path: &std::path::Path) {
    summary.cancelled = true;
    summary.complete = false;
    if summary.scanned != 1 || summary.results.len() != 1 {
        return;
    }

    let result = &mut summary.results[0];
    if result.status == scanner::ScanStatus::Cancelled {
        return;
    }
    match result.status {
        scanner::ScanStatus::Clean => summary.clean = summary.clean.saturating_sub(1),
        scanner::ScanStatus::Infected => summary.infected = summary.infected.saturating_sub(1),
        scanner::ScanStatus::Unknown => summary.unknown = summary.unknown.saturating_sub(1),
        scanner::ScanStatus::Error => summary.errors = summary.errors.saturating_sub(1),
        scanner::ScanStatus::Skipped => summary.skipped = summary.skipped.saturating_sub(1),
        scanner::ScanStatus::Cancelled => unreachable!(),
    }
    result.path = path.to_string_lossy().into_owned();
    result.sha256 = None;
    result.status = scanner::ScanStatus::Cancelled;
    result.reason = Some("cancelled by user".into());
    result.bytes_scanned = 0;
    summary.cancelled_files = summary.cancelled_files.saturating_add(1);
}

#[tauri::command]
fn cancel_scan(
    app: tauri::AppHandle,
    manager: tauri::State<'_, ScanManager>,
    scan_id: String,
) -> bool {
    let accepted = manager.request_cancel(&scan_id);
    if accepted {
        let _ = app.emit(
            "scan-state",
            ScanStateEvent {
                scan_id,
                state: ScanLifecycle::Cancelling,
            },
        );
    }
    accepted
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

#[cfg(test)]
mod tests {
    use super::*;
    use scanner::ScanStatus;

    #[test]
    fn cancellation_keeps_the_backend_busy_until_the_worker_finishes() {
        let manager = ScanManager::new();
        let cancel = manager.begin_scan("first".into()).unwrap();

        assert!(manager.begin_scan("second".into()).is_err());
        assert!(manager.request_cancel("first"));
        assert!(*cancel.borrow());
        assert!(!manager.request_cancel("first"));
        assert!(manager.finish_scan("first").unwrap());

        assert!(manager.begin_scan("second".into()).is_ok());
    }

    #[test]
    fn accepted_cancel_does_not_return_a_single_file_as_clean() {
        let mut summary = ScanSummary::default();
        summary.record(scanner::ScanResult::new(
            "sample.txt",
            ScanStatus::Clean,
            Some("abc".into()),
            None,
            6,
        ));

        mark_user_cancelled(&mut summary, std::path::Path::new("sample.txt"));

        assert!(summary.cancelled);
        assert!(!summary.complete);
        assert_eq!(summary.clean, 0);
        assert_eq!(summary.cancelled_files, 1);
        assert_eq!(summary.results[0].status, ScanStatus::Cancelled);
    }

    #[test]
    fn selected_target_can_be_scanned_again_until_its_expiry() {
        let manager = ScanManager::new();
        let path = PathBuf::from("fixture.txt");
        manager
            .selected
            .lock()
            .unwrap()
            .insert("target".into(), (Instant::now(), path.clone()));

        assert_eq!(manager.selected_path("target").unwrap(), path);
        assert_eq!(manager.selected_path("target").unwrap(), path);
    }
}
