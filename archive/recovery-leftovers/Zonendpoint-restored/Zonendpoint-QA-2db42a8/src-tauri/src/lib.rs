mod cache;
pub mod scanner;
mod watcher;

use std::sync::{Arc, Mutex};
use tauri::Manager;
use watcher::Guard;

#[tauri::command]
async fn quick_scan(path: String) -> Result<scanner::ScanResult, String> {
    scanner::scan_file(&path).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn deep_scan(path: String) -> Result<Vec<scanner::ScanResult>, String> {
    let mut out = Vec::new();
    let mut pending = vec![std::path::PathBuf::from(path)];
    while let Some(p) = pending.pop() {
        if p.is_dir() {
            for entry in std::fs::read_dir(p).map_err(|e| e.to_string())? {
                pending.push(entry.map_err(|e| e.to_string())?.path());
            }
        } else if p.is_file() {
            if let Ok(result) = scanner::scan_file(p.to_string_lossy().to_string()).await { out.push(result); }
        }
    }
    Ok(out)
}

#[tauri::command]
fn cache_stats(app: tauri::AppHandle) -> Result<cache::CacheStats, String> {
    let path = app.path().app_data_dir().map_err(|e| e.to_string())?.join("cache.db");
    cache::stats(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_guard(app: tauri::AppHandle, enabled: bool) -> Result<bool, String> {
    let state = app.state::<Arc<Mutex<Guard>>>();
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    if enabled { guard.start().map_err(|e| e.to_string())?; } else { guard.stop(); }
    Ok(guard.is_running())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let guard = Arc::new(Mutex::new(Guard::default()));
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(guard)
        .invoke_handler(tauri::generate_handler![quick_scan, deep_scan, cache_stats, set_guard])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
