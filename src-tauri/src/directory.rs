use crate::scanner::{ScanResult, ScanStatus};
use serde::Serialize;
use std::{
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::{mpsc, watch};

pub const MAX_RETAINED_RESULTS: usize = 1000;
const QUEUE_CAPACITY: usize = 32;

#[derive(Clone, Serialize)]
pub struct ScanSummary {
    pub complete: bool,
    pub cancelled: bool,
    pub scanned: usize,
    pub clean: usize,
    pub infected: usize,
    pub unknown: usize,
    pub errors: usize,
    pub skipped: usize,
    pub cancelled_files: usize,
    pub details_truncated: bool,
    pub results: Vec<ScanResult>,
}

impl Default for ScanSummary {
    fn default() -> Self {
        Self {
            complete: true,
            cancelled: false,
            scanned: 0,
            clean: 0,
            infected: 0,
            unknown: 0,
            errors: 0,
            skipped: 0,
            cancelled_files: 0,
            details_truncated: false,
            results: Vec::new(),
        }
    }
}

impl ScanSummary {
    pub fn record(&mut self, result: ScanResult) {
        self.scanned = self.scanned.saturating_add(1);
        match result.status {
            ScanStatus::Clean => self.clean = self.clean.saturating_add(1),
            ScanStatus::Infected => self.infected = self.infected.saturating_add(1),
            ScanStatus::Unknown => {
                self.unknown = self.unknown.saturating_add(1);
                self.complete = false;
            }
            ScanStatus::Error => {
                self.errors = self.errors.saturating_add(1);
                self.complete = false;
            }
            ScanStatus::Skipped => {
                self.skipped = self.skipped.saturating_add(1);
                self.complete = false;
            }
            ScanStatus::Cancelled => {
                self.cancelled_files = self.cancelled_files.saturating_add(1);
                self.cancelled = true;
                self.complete = false;
            }
        }
        if self.results.len() < MAX_RETAINED_RESULTS {
            self.results.push(result);
        } else {
            self.details_truncated = true;
        }
    }
}

pub type ScanFuture = Pin<Box<dyn Future<Output = ScanResult> + Send>>;
pub type ScanFn = Box<dyn Fn(PathBuf, watch::Receiver<bool>) -> ScanFuture + Send + Sync>;
pub type ProgressFn = Box<dyn Fn(&ScanSummary, &ScanResult) + Send + Sync>;

enum WalkItem {
    File(PathBuf),
    Skip(PathBuf),
    Error(PathBuf, String),
}

async fn wait_cancelled(mut cancel: watch::Receiver<bool>) -> bool {
    loop {
        if *cancel.borrow() {
            return true;
        }
        if cancel.changed().await.is_err() {
            return false;
        }
    }
}

pub async fn scan_directory(
    root: PathBuf,
    cancel: watch::Receiver<bool>,
    scan: ScanFn,
    progress: ProgressFn,
) -> ScanSummary {
    scan_roots(vec![root], cancel, scan, progress).await
}

pub async fn scan_roots(
    roots: Vec<PathBuf>,
    cancel: watch::Receiver<bool>,
    scan: ScanFn,
    progress: ProgressFn,
) -> ScanSummary {
    let root = roots.first().cloned().unwrap_or_default();
    let (tx, mut rx) = mpsc::channel(QUEUE_CAPACITY);
    let cancelled = Arc::new(AtomicBool::new(false));
    let scan_cancel = cancel.clone();
    let producer_cancelled = cancelled.clone();
    let producer = tokio::task::spawn_blocking(move || {
        for producer_root in roots {
            for entry in walkdir::WalkDir::new(&producer_root)
                .follow_links(false)
                .follow_root_links(false)
                .max_open(16)
            {
                if producer_cancelled.load(Ordering::Relaxed) {
                    break;
                }
                let item = match entry {
                    Ok(entry) if entry.file_type().is_dir() => continue,
                    Ok(entry) if entry.file_type().is_symlink() => {
                        WalkItem::Skip(entry.into_path())
                    }
                    Ok(entry) if entry.file_type().is_file() => WalkItem::File(entry.into_path()),
                    Ok(entry) => WalkItem::Skip(entry.into_path()),
                    Err(error) => WalkItem::Error(
                        error.path().unwrap_or(&producer_root).to_path_buf(),
                        error.to_string(),
                    ),
                };
                if tx.blocking_send(item).is_err() {
                    break;
                }
            }
        }
    });
    let monitor_flag = cancelled.clone();
    let monitor_cancel = cancel.clone();
    let monitor = tokio::spawn(async move {
        if wait_cancelled(monitor_cancel).await {
            monitor_flag.store(true, Ordering::Relaxed);
        }
    });
    let mut summary = ScanSummary::default();
    let mut cancellation_observed = false;
    while let Some(item) = rx.recv().await {
        let result = if cancellation_observed || cancelled.load(Ordering::Relaxed) {
            cancellation_observed = true;
            match item {
                WalkItem::File(path) | WalkItem::Skip(path) => ScanResult::new(
                    path,
                    ScanStatus::Cancelled,
                    None,
                    Some("cancelled before scan".into()),
                    0,
                ),
                WalkItem::Error(path, reason) => {
                    ScanResult::new(path, ScanStatus::Error, None, Some(reason), 0)
                }
            }
        } else {
            match item {
                WalkItem::File(path) => scan(path, scan_cancel.clone()).await,
                WalkItem::Skip(path) => ScanResult::new(
                    path,
                    ScanStatus::Skipped,
                    None,
                    Some("symbolic link or unsupported filesystem entry was not followed".into()),
                    0,
                ),
                WalkItem::Error(path, reason) => ScanResult::error(path, reason),
            }
        };
        // Cancellation may arrive while a scan finishes, before the monitor runs.
        // Do not publish that racing CLEAN as a completed scan.
        let result = if *cancel.borrow() && result.status == ScanStatus::Clean {
            ScanResult::new(
                result.path,
                ScanStatus::Cancelled,
                None,
                Some("cancelled before completion".into()),
                0,
            )
        } else {
            result
        };
        summary.record(result.clone());
        progress(&summary, &result);
        if *cancel.borrow() {
            cancelled.store(true, Ordering::Relaxed);
            cancellation_observed = true;
        }
    }
    if producer.await.is_err() {
        let result = ScanResult::error(&root, "folder traversal worker failed");
        summary.record(result.clone());
        progress(&summary, &result);
    }
    monitor.abort();
    if cancellation_observed || cancelled.load(Ordering::Relaxed) {
        summary.cancelled = true;
        summary.complete = false;
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::{ScanResult, ScanStatus};
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };
    use tokio::sync::watch;

    fn test_dir() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("zonendpoint-folder-{}-{nonce}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn root_junction_is_skipped_not_followed() {
        use std::os::windows::process::CommandExt;
        let root = test_dir();
        let target = root.join("target");
        let link = root.join("junction");
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("private.txt"), b"do not scan").unwrap();
        let output = std::process::Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(&link)
            .arg(&target)
            .creation_flags(0x08000000)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let (_tx, cancel) = watch::channel(false);
        let scan: ScanFn = Box::new(|_, _| panic!("junction target must not be scanned"));
        let summary = scan_directory(link.clone(), cancel, scan, Box::new(|_, _| {})).await;
        assert_eq!(summary.skipped, 1);
        assert_eq!(summary.clean, 0);
        assert!(!summary.complete);
        std::fs::remove_dir(link).unwrap();
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn multi_root_cancel_preserves_already_detected_threat() {
        let root = test_dir();
        let threat = root.join("threat.bin");
        let later = root.join("later.bin");
        std::fs::write(&threat, b"harmless mock").unwrap();
        std::fs::write(&later, b"later").unwrap();
        let (tx, cancel) = watch::channel(false);
        let scan: ScanFn = Box::new(move |path, _| {
            let tx = tx.clone();
            Box::pin(async move {
                let _ = tx.send(true);
                ScanResult::new(
                    path,
                    ScanStatus::Infected,
                    None,
                    Some("mock positive evidence".into()),
                    1,
                )
            })
        });
        let summary = scan_roots(vec![threat, later], cancel, scan, Box::new(|_, _| {})).await;
        assert!(summary.cancelled);
        assert!(!summary.complete);
        assert_eq!(summary.infected, 1);
        assert_eq!(summary.results[0].status, ScanStatus::Infected);
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn multiple_roots_keep_missing_file_errors_and_driver_results() {
        let root = test_dir();
        std::fs::create_dir(root.join("drivers")).unwrap();
        std::fs::write(root.join("core.exe"), b"core").unwrap();
        std::fs::write(root.join("drivers").join("fixture.sys"), b"driver").unwrap();
        let (_tx, cancel) = watch::channel(false);
        let scan: ScanFn = Box::new(|path, _| {
            Box::pin(async move { ScanResult::new(path, ScanStatus::Clean, None, None, 1) })
        });
        let summary = scan_roots(
            vec![
                root.join("core.exe"),
                root.join("missing.dll"),
                root.join("drivers"),
            ],
            cancel,
            scan,
            Box::new(|_, _| {}),
        )
        .await;
        assert_eq!(summary.scanned, 3);
        assert_eq!(summary.clean, 2);
        assert_eq!(summary.errors, 1);
        assert!(!summary.complete);
        assert!(
            summary
                .results
                .iter()
                .any(|r| r.path.ends_with("missing.dll") && r.status == ScanStatus::Error)
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn folder_scan_retains_per_file_error_and_unicode_path() {
        let root = test_dir();
        std::fs::write(root.join("lỗi.txt"), b"one").unwrap();
        std::fs::write(root.join("broken.bin"), b"two").unwrap();
        let (_tx, cancel) = watch::channel(false);
        let scan: ScanFn = Box::new(|path, _| {
            Box::pin(async move {
                if path.file_name().unwrap() == "broken.bin" {
                    ScanResult::error(path, "mock file read failure")
                } else {
                    ScanResult::new(
                        path,
                        ScanStatus::Clean,
                        Some("fixture clean".into()),
                        None,
                        3,
                    )
                }
            })
        });
        let summary = scan_directory(root.clone(), cancel, scan, Box::new(|_, _| {})).await;
        assert_eq!(summary.errors, 1);
        assert_eq!(summary.clean, 1);
        assert!(
            summary
                .results
                .iter()
                .any(|result| result.path.contains("lỗi.txt"))
        );
        assert!(
            summary
                .results
                .iter()
                .any(|result| result.reason.as_deref() == Some("mock file read failure"))
        );
        assert!(!summary.complete);
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn folder_scan_bounds_retained_results() {
        let root = test_dir();
        for index in 0..MAX_RETAINED_RESULTS + 2 {
            std::fs::write(root.join(format!("{index}.bin")), b"x").unwrap();
        }
        let (_tx, cancel) = watch::channel(false);
        let scan: ScanFn = Box::new(|path, _| {
            Box::pin(async move { ScanResult::new(path, ScanStatus::Clean, None, None, 1) })
        });
        let summary = scan_directory(root.clone(), cancel, scan, Box::new(|_, _| {})).await;
        assert_eq!(summary.clean, MAX_RETAINED_RESULTS + 2);
        assert_eq!(summary.results.len(), MAX_RETAINED_RESULTS);
        assert!(summary.details_truncated);
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn folder_scan_marks_queued_items_cancelled() {
        let root = test_dir();
        for index in 0..64 {
            std::fs::write(root.join(format!("{index}.bin")), b"x").unwrap();
        }
        let (tx, cancel) = watch::channel(false);
        let scan: ScanFn = Box::new(move |path, _| {
            let _ = tx.send(true);
            Box::pin(async move { ScanResult::new(path, ScanStatus::Clean, None, None, 1) })
        });
        let summary = scan_directory(root.clone(), cancel, scan, Box::new(|_, _| {})).await;
        assert!(summary.cancelled);
        assert!(!summary.complete);
        assert!(summary.cancelled_files > 0);
        let _ = std::fs::remove_dir_all(root);
    }
}
