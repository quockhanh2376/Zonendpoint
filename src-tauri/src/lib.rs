pub mod cache;
pub mod config;
pub mod directory;
pub mod quick;
pub mod scanner;
pub mod server_status;
pub mod watcher;

#[cfg(all(target_os = "windows", not(target_arch = "x86_64")))]
compile_error!("Zonendpoint supports Windows x64 builds only");

#[cfg(all(feature = "native-ui-tests", not(debug_assertions)))]
compile_error!("native-ui-tests must never be enabled in release builds");

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
use tokio::sync::watch;
use watcher::Guard;

struct ThrottleState {
    last_emitted: Option<Instant>,
    last_pending: Option<(ScanSummary, scanner::ScanResult)>,
}

pub(crate) struct ProgressThrottler<F> {
    throttle_interval: Duration,
    emit: F,
    state: Mutex<ThrottleState>,
}

impl<F: Fn(&ScanSummary, &scanner::ScanResult) + Send + Sync + 'static> ProgressThrottler<F> {
    pub(crate) fn new(throttle_interval: Duration, emit: F) -> Self {
        Self {
            throttle_interval,
            emit,
            state: Mutex::new(ThrottleState {
                last_emitted: None,
                last_pending: None,
            }),
        }
    }

    pub(crate) fn handle(&self, summary: &ScanSummary, latest: &scanner::ScanResult) {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let is_non_clean = latest.status != scanner::ScanStatus::Clean;
        let elapsed_ok = match guard.last_emitted {
            Some(instant) => instant.elapsed() >= self.throttle_interval,
            None => true,
        };

        if is_non_clean || elapsed_ok {
            (self.emit)(summary, latest);
            guard.last_emitted = Some(Instant::now());
            guard.last_pending = None;
        } else {
            guard.last_pending = Some((summary.clone(), latest.clone()));
        }
    }

    pub(crate) fn flush(&self) {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((summary, latest)) = guard.last_pending.take() {
            (self.emit)(&summary, &latest);
            guard.last_emitted = Some(Instant::now());
        }
    }
}

pub struct ScanManager {
    active: Mutex<Option<ActiveScan>>,
    selected: Mutex<HashMap<String, (Instant, PathBuf)>>,
    semaphore: Arc<scanner::ScanLimiter>,
}

pub struct ActiveScan {
    scan_id: String,
    cancel: watch::Sender<bool>,
    state: ScanLifecycle,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ScanLifecycle {
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
    pub fn new() -> Self {
        Self {
            active: Mutex::new(None),
            selected: Mutex::new(HashMap::new()),
            semaphore: Arc::new(scanner::ScanLimiter::new(config::scan_concurrency())),
        }
    }

    pub fn begin_scan(&self, scan_id: String) -> Result<watch::Receiver<bool>, String> {
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

    pub fn request_cancel(&self, scan_id: &str) -> bool {
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

    pub fn finish_scan(&self, scan_id: &str) -> Result<bool, String> {
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

    pub fn active_scan_id(&self) -> Option<String> {
        self.active
            .lock()
            .ok()
            .and_then(|active| active.as_ref().map(|scan| scan.scan_id.clone()))
    }
}

impl Default for ScanManager {
    fn default() -> Self {
        Self::new()
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
    #[cfg(all(feature = "native-ui-tests", debug_assertions))]
    let fixture = std::env::var_os("ZONENDPOINT_TEST_FIXTURE").map(PathBuf::from);
    #[cfg(not(all(feature = "native-ui-tests", debug_assertions)))]
    let fixture: Option<PathBuf> = None;
    let path = if let Some(root) = fixture {
        match kind.as_str() {
            "file" => root.join("benign.txt"),
            "folder" => root,
            _ => return Err("selection kind must be file or folder".into()),
        }
    } else {
        let selected = match kind.as_str() {
            "file" => app.dialog().file().blocking_pick_file(),
            "folder" => app.dialog().file().blocking_pick_folder(),
            _ => return Err("selection kind must be file or folder".into()),
        };
        let Some(path) = selected else {
            return Ok(None);
        };
        path.into_path().map_err(|error| error.to_string())?
    };
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
    let target = manager.selected_path(&target_id)?;
    run_scan(app, manager, scan_id, vec![target], Vec::new()).await
}

#[tauri::command]
async fn quick_scan(
    app: tauri::AppHandle,
    manager: tauri::State<'_, ScanManager>,
    scan_id: String,
) -> Result<ScanSummary, String> {
    let root = quick::system_root()?;
    let exclusions = quick::exclusions_for_root(&root);
    run_scan(
        app,
        manager,
        scan_id,
        quick::targets_for_root(&root),
        exclusions,
    )
    .await
}

#[tauri::command]
async fn check_server_status() -> Result<String, String> {
    let addr =
        scanner::configured_clamd_addr().map_err(|_| scanner::PREFLIGHT_ERROR_MSG.to_string())?;
    let (_tx, cancel) = watch::channel(false);
    scanner::clamd_preflight(addr, cancel).await
}

#[tauri::command]
fn get_server_config() -> Result<config::ServerConfigDto, String> {
    config::get_server_config()
}

#[tauri::command]
async fn save_server_config(addr: String) -> Result<config::ServerConfigDto, String> {
    config::save_server_address(&addr)?;
    config::get_server_config()
}

#[tauri::command]
fn get_server_status() -> server_status::ServerStatusDto {
    server_status::load_server_status()
}

async fn run_scan(
    app: tauri::AppHandle,
    manager: tauri::State<'_, ScanManager>,
    scan_id: String,
    targets: Vec<PathBuf>,
    exclusions: Vec<PathBuf>,
) -> Result<ScanSummary, String> {
    if scan_id.is_empty() || scan_id.len() > 80 {
        return Err("invalid scan identifier".into());
    }
    let target = targets
        .first()
        .cloned()
        .ok_or("scan target list is empty")?;
    let multiple = targets.len() > 1;

    let addr =
        scanner::configured_clamd_addr().map_err(|_| scanner::PREFLIGHT_ERROR_MSG.to_string())?;

    let cancel_rx = manager.begin_scan(scan_id.clone())?;
    let engine_version = match scanner::clamd_preflight(addr, cancel_rx.clone()).await {
        Ok(version) => version,
        Err(err) => {
            let _ = manager.finish_scan(&scan_id);
            return Err(err);
        }
    };

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
        let semaphore = manager.semaphore.clone();
        let scan_addr = addr;
        let scan_cache = cache_path.clone();
        let scan_semaphore = semaphore.clone();
        let scan_version = engine_version.clone();
        let scan: ScanFn = Box::new(move |file, cancel| {
            let cache_path = scan_cache.clone();
            let semaphore = scan_semaphore.clone();
            let addr = scan_addr;
            let version = scan_version.clone();
            Box::pin(async move {
                scanner::scan_file_with_engine_override(
                    file,
                    cache_path,
                    addr,
                    Some(version),
                    cancel,
                    semaphore,
                )
                .await
            })
        });
        let progress_app = app.clone();
        let progress_id = scan_id.clone();
        let throttler = Arc::new(ProgressThrottler::new(
            Duration::from_millis(100),
            move |summary, latest| {
                let _ = progress_app.emit(
                    "scan-progress",
                    ScanProgress::new(progress_id.clone(), summary, latest.clone()),
                );
            },
        ));
        let throttler_for_progress = throttler.clone();
        let progress: ProgressFn = Box::new(move |summary, latest| {
            throttler_for_progress.handle(summary, latest);
        });

        if multiple {
            let summary = directory::scan_roots_with_exclusions(
                targets,
                exclusions,
                cancel_rx.clone(),
                scan,
                progress,
            )
            .await;
            throttler.flush();
            return Ok(summary);
        }
        match tokio::fs::symlink_metadata(&target).await {
            Ok(metadata) if metadata.is_dir() => {
                let summary = directory::scan_roots_with_exclusions(
                    vec![target],
                    exclusions,
                    cancel_rx.clone(),
                    scan,
                    progress,
                )
                .await;
                throttler.flush();
                Ok(summary)
            }
            _ => {
                let result = scanner::scan_file_with_engine_override(
                    target,
                    cache_path,
                    addr,
                    Some(engine_version),
                    cancel_rx.clone(),
                    semaphore,
                )
                .await;
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
    if summary.scanned != 1 {
        return;
    }

    if summary.clean == 1 {
        summary.clean = 0;
        summary.cancelled_files = summary.cancelled_files.saturating_add(1);
        let cancelled_result = scanner::ScanResult::new(
            path,
            scanner::ScanStatus::Cancelled,
            None,
            Some("cancelled by user".into()),
            0,
        );
        if summary.results.is_empty() {
            summary.results.push(cancelled_result);
        } else {
            summary.results[0] = cancelled_result;
        }
        return;
    }

    if summary.results.len() == 1 && summary.results[0].status == scanner::ScanStatus::Clean {
        summary.clean = summary.clean.saturating_sub(1);
        let result = &mut summary.results[0];
        result.path = path.to_string_lossy().into_owned();
        result.sha256 = None;
        result.status = scanner::ScanStatus::Cancelled;
        result.reason = Some("cancelled by user".into());
        result.bytes_scanned = 0;
        summary.cancelled_files = summary.cancelled_files.saturating_add(1);
    }
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
fn set_window_theme_icon(app: tauri::AppHandle, theme: String) -> Result<(), String> {
    let bytes: &[u8] = match theme.as_str() {
        "light" => include_bytes!("../icons/icon.png"),
        "dark" => include_bytes!("../icons/dark/icon.png"),
        _ => return Err("unsupported theme".into()),
    };
    let icon = tauri::image::Image::from_bytes(bytes).map_err(|error| error.to_string())?;
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    window.set_icon(icon).map_err(|error| error.to_string())
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
                Arc::new(scanner::ScanLimiter::new(config::scan_concurrency())),
            )
            .await
        }
        Err(reason) => scanner::ScanResult::error(path, reason),
    }
}

fn stop_guard(app: &tauri::AppHandle) {
    if let Some(guard) = app.try_state::<Arc<Mutex<Guard>>>() {
        let _ = guard.lock().map(|mut g| g.stop());
    }
}

fn cancel_and_exit(app: tauri::AppHandle, exit_code: i32) {
    let manager = app.state::<ScanManager>();
    if let Some(active_id) = manager.active_scan_id() {
        manager.request_cancel(&active_id);
        let app_clone = app.clone();
        tauri::async_runtime::spawn(async move {
            let start = std::time::Instant::now();
            while app_clone.state::<ScanManager>().active_scan_id().is_some() {
                if start.elapsed() > std::time::Duration::from_secs(5) {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
            stop_guard(&app_clone);
            app_clone.cleanup_before_exit();
            std::process::exit(exit_code);
        });
    } else {
        stop_guard(&app);
        app.cleanup_before_exit();
        std::process::exit(exit_code);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(ScanManager::new())
        .manage(Arc::new(Mutex::new(Guard::default())))
        .setup(|_app| {
            if let Ok(path) = crate::cache::default_path() {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                let _ =
                    crate::cache::cleanup_expired(&path, now, crate::scanner::CACHE_TTL_SECONDS);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            select_scan_path,
            scan_path,
            quick_scan,
            cancel_scan,
            set_window_theme_icon,
            check_server_status,
            get_server_status,
            get_server_config,
            save_server_config
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle().clone();
                let manager = app.state::<ScanManager>();
                if manager.active_scan_id().is_some() {
                    api.prevent_close();
                    let _ = window.hide();
                    cancel_and_exit(app, 0);
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| match event {
        tauri::RunEvent::ExitRequested { code, api, .. } => {
            let manager = app_handle.state::<ScanManager>();
            if manager.active_scan_id().is_some() {
                api.prevent_exit();
            }
            cancel_and_exit(app_handle.clone(), code.unwrap_or(0));
        }
        tauri::RunEvent::Exit => {
            std::process::exit(0);
        }
        _ => {}
    });
}

#[cfg(test)]
pub(crate) static TEST_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
pub(crate) fn lock_test_env() -> std::sync::MutexGuard<'static, ()> {
    match TEST_ENV_LOCK.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
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
    fn accepted_cancel_preserves_nonclean_evidence() {
        for status in [
            ScanStatus::Infected,
            ScanStatus::Error,
            ScanStatus::Unknown,
            ScanStatus::Skipped,
        ] {
            let mut summary = ScanSummary::default();
            summary.record(scanner::ScanResult::new(
                "sample.txt",
                status,
                None,
                Some("original evidence".into()),
                6,
            ));
            mark_user_cancelled(&mut summary, std::path::Path::new("sample.txt"));
            assert!(summary.cancelled);
            assert!(!summary.complete);
            assert_eq!(summary.results[0].status, status);
            assert_eq!(
                summary.results[0].reason.as_deref(),
                Some("original evidence")
            );
            assert_eq!(summary.cancelled_files, 0);
            if status == ScanStatus::Infected {
                assert_eq!(summary.infected, 1);
            }
        }
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

    #[test]
    fn throttles_rapid_clean_progress_and_flushes_tail() {
        let emitted = Arc::new(Mutex::new(Vec::new()));
        let emitted_clone = emitted.clone();
        let throttler =
            ProgressThrottler::new(Duration::from_millis(100), move |_summary, latest| {
                emitted_clone.lock().unwrap().push(latest.path.clone());
            });

        let mut summary = ScanSummary::default();
        for i in 0..50 {
            let res = scanner::ScanResult::new(
                format!("clean_{}.txt", i),
                ScanStatus::Clean,
                Some("h".into()),
                None,
                10,
            );
            summary.record(res.clone());
            throttler.handle(&summary, &res);
        }

        // Trong thời gian cực ngắn (<100ms), chỉ sự kiện clean đầu tiên được emit
        let count_before_flush = emitted.lock().unwrap().len();
        assert_eq!(count_before_flush, 1);
        assert_eq!(emitted.lock().unwrap()[0], "clean_0.txt");

        // Gọi flush để emit kết quả tail
        throttler.flush();
        let count_after_flush = emitted.lock().unwrap().len();
        assert_eq!(count_after_flush, 2);
        assert_eq!(emitted.lock().unwrap()[1], "clean_49.txt");
    }

    #[test]
    fn emits_non_clean_progress_immediately_even_within_throttle_window() {
        let emitted = Arc::new(Mutex::new(Vec::new()));
        let emitted_clone = emitted.clone();
        let throttler =
            ProgressThrottler::new(Duration::from_millis(100), move |_summary, latest| {
                emitted_clone
                    .lock()
                    .unwrap()
                    .push((latest.path.clone(), latest.status));
            });

        let mut summary = ScanSummary::default();

        // 1. Clean file đầu tiên -> emit ngay
        let clean1 = scanner::ScanResult::new("clean1.txt", ScanStatus::Clean, None, None, 1);
        summary.record(clean1.clone());
        throttler.handle(&summary, &clean1);

        // 2. Infected file ngay lập tức -> phát ngay
        let infected = scanner::ScanResult::new(
            "bad.exe",
            ScanStatus::Infected,
            None,
            Some("Win.Trojan".into()),
            2,
        );
        summary.record(infected.clone());
        throttler.handle(&summary, &infected);

        // 3. Error file ngay lập tức -> phát ngay
        let error_file = scanner::ScanResult::new(
            "locked.bin",
            ScanStatus::Error,
            None,
            Some("permission denied".into()),
            0,
        );
        summary.record(error_file.clone());
        throttler.handle(&summary, &error_file);

        // 4. Clean file ngay sau đó -> bị throttle
        let clean2 = scanner::ScanResult::new("clean2.txt", ScanStatus::Clean, None, None, 1);
        summary.record(clean2.clone());
        throttler.handle(&summary, &clean2);

        {
            let guard = emitted.lock().unwrap();
            assert_eq!(guard.len(), 3);
            assert_eq!(guard[0], ("clean1.txt".into(), ScanStatus::Clean));
            assert_eq!(guard[1], ("bad.exe".into(), ScanStatus::Infected));
            assert_eq!(guard[2], ("locked.bin".into(), ScanStatus::Error));
        }

        // Flush phát tail clean2
        throttler.flush();
        {
            let guard = emitted.lock().unwrap();
            assert_eq!(guard.len(), 4);
            assert_eq!(guard[3], ("clean2.txt".into(), ScanStatus::Clean));
        }
    }
}
