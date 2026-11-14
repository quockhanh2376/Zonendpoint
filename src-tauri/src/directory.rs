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
use tokio::{
    sync::{mpsc, watch},
    task::JoinSet,
};

pub const MAX_RETAINED_RESULTS: usize = 1000;
const QUEUE_CAPACITY: usize = 32;

fn record_result(
    summary: &mut ScanSummary,
    progress: &ProgressFn,
    cancel: &watch::Receiver<bool>,
    result: ScanResult,
) -> bool {
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
    progress(summary, &result);
    *cancel.borrow()
}

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
            ScanStatus::Clean => {
                self.clean = self.clean.saturating_add(1);
                return;
            }
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
        } else if result.status == ScanStatus::Infected {
            if let Some(pos) = self
                .results
                .iter()
                .rposition(|r| r.status != ScanStatus::Infected)
            {
                self.results.remove(pos);
                self.results.push(result);
                self.details_truncated = true;
            } else {
                self.details_truncated = true;
            }
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
    scan_roots_with_exclusions(vec![root], Vec::new(), cancel, scan, progress).await
}

pub async fn scan_roots(
    roots: Vec<PathBuf>,
    cancel: watch::Receiver<bool>,
    scan: ScanFn,
    progress: ProgressFn,
) -> ScanSummary {
    scan_roots_with_exclusions(roots, Vec::new(), cancel, scan, progress).await
}

pub async fn scan_roots_with_exclusions(
    roots: Vec<PathBuf>,
    exclusions: Vec<PathBuf>,
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
            if !exclusions.is_empty() && crate::quick::is_path_excluded(&producer_root, &exclusions)
            {
                continue;
            }
            let walker = walkdir::WalkDir::new(&producer_root)
                .follow_links(false)
                .follow_root_links(false)
                .max_open(16);
            let mut it = walker.into_iter();
            loop {
                if producer_cancelled.load(Ordering::Relaxed) {
                    break;
                }
                let entry = match it.next() {
                    None => break,
                    Some(res) => res,
                };
                let item = match entry {
                    Ok(entry) => {
                        let path = entry.path();
                        if !exclusions.is_empty()
                            && crate::quick::is_path_excluded(path, &exclusions)
                        {
                            if entry.file_type().is_dir() {
                                it.skip_current_dir();
                            }
                            continue;
                        }
                        if entry.file_type().is_dir() {
                            continue;
                        }
                        if entry.file_type().is_symlink() {
                            WalkItem::Skip(entry.into_path())
                        } else if entry.file_type().is_file() {
                            WalkItem::File(entry.into_path())
                        } else {
                            WalkItem::Skip(entry.into_path())
                        }
                    }
                    Err(error) => {
                        if error.path().is_some_and(|err_path| {
                            !exclusions.is_empty()
                                && crate::quick::is_path_excluded(err_path, &exclusions)
                        }) {
                            continue;
                        }
                        WalkItem::Error(
                            error.path().unwrap_or(&producer_root).to_path_buf(),
                            error.to_string(),
                        )
                    }
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
    let scan = Arc::new(scan);
    let mut scans = JoinSet::new();
    while let Some(item) = rx.recv().await {
        let result = if cancellation_observed || cancelled.load(Ordering::Relaxed) {
            cancellation_observed = true;
            while let Some(joined) = scans.join_next().await {
                let result = match joined {
                    Ok(result) => result,
                    Err(_) => ScanResult::error(&root, "scan worker failed"),
                };
                cancellation_observed |= record_result(&mut summary, &progress, &cancel, result);
            }
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
                WalkItem::File(path) => {
                    let scan = scan.clone();
                    let scan_cancel = scan_cancel.clone();
                    scans.spawn(async move { scan(path, scan_cancel).await });
                    if scans.len() >= QUEUE_CAPACITY {
                        let result = match scans.join_next().await {
                            Some(Ok(result)) => result,
                            Some(Err(_)) => ScanResult::error(&root, "scan worker failed"),
                            None => continue,
                        };
                        cancellation_observed |=
                            record_result(&mut summary, &progress, &cancel, result);
                    }
                    continue;
                }
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
        cancellation_observed |= record_result(&mut summary, &progress, &cancel, result);
        if cancellation_observed {
            cancelled.store(true, Ordering::Relaxed);
        }
    }
    while let Some(joined) = scans.join_next().await {
        let result = match joined {
            Ok(result) => result,
            Err(_) => ScanResult::error(&root, "scan worker failed"),
        };
        cancellation_observed |= record_result(&mut summary, &progress, &cancel, result);
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
    use crate::quick;
    use crate::scanner::{ScanResult, ScanStatus};
    use std::{
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };
    use tokio::sync::watch;

    static TEST_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_dir() -> PathBuf {
        let count = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "zonendpoint-folder-{}-{count}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    struct DenyAccessGuard {
        path: PathBuf,
    }

    impl DenyAccessGuard {
        fn new(path: &Path) -> Self {
            #[cfg(windows)]
            {
                let output = std::process::Command::new("icacls.exe")
                    .arg(path)
                    .args(["/deny", "*S-1-1-0:(RD)"])
                    .output();
                if let Ok(out) = output {
                    assert!(
                        out.status.success(),
                        "icacls deny warning: {}",
                        String::from_utf8_lossy(&out.stderr)
                    );
                }
            }
            #[cfg(not(windows))]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o000));
            }
            Self {
                path: path.to_path_buf(),
            }
        }
    }

    impl Drop for DenyAccessGuard {
        fn drop(&mut self) {
            #[cfg(windows)]
            {
                let _ = std::process::Command::new("icacls.exe")
                    .arg(&self.path)
                    .args(["/remove:d", "*S-1-1-0"])
                    .output();
            }
            #[cfg(not(windows))]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ =
                    std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(0o755));
            }
        }
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
        assert!(summary.infected >= 1);
        let infected_results: Vec<_> = summary
            .results
            .iter()
            .filter(|result| result.status == ScanStatus::Infected)
            .collect();
        assert_eq!(infected_results.len(), summary.infected);
        assert!(
            infected_results
                .iter()
                .any(|result| result.reason.as_deref() == Some("mock positive evidence"))
        );
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
        std::fs::write(root.join("harmless.bin"), b"two").unwrap();
        let (_tx, cancel) = watch::channel(false);
        let scan: ScanFn = Box::new(|path, _| {
            Box::pin(async move {
                if path.file_name().unwrap() == "lỗi.txt" {
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
        let scan: ScanFn =
            Box::new(|path, _| Box::pin(async move { ScanResult::error(path, "simulated error") }));
        let summary = scan_directory(root.clone(), cancel, scan, Box::new(|_, _| {})).await;
        assert_eq!(summary.errors, MAX_RETAINED_RESULTS + 2);
        assert_eq!(summary.results.len(), MAX_RETAINED_RESULTS);
        assert!(summary.details_truncated);
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn clean_files_do_not_crowd_out_subsequent_infected_result() {
        let mut summary = ScanSummary::default();
        for index in 0..MAX_RETAINED_RESULTS + 10 {
            summary.record(ScanResult::new(
                format!("clean_{index}.txt"),
                ScanStatus::Clean,
                None,
                None,
                10,
            ));
        }
        assert_eq!(summary.scanned, MAX_RETAINED_RESULTS + 10);
        assert_eq!(summary.clean, MAX_RETAINED_RESULTS + 10);
        assert!(summary.results.is_empty());
        assert!(!summary.details_truncated);

        // 1 infected file arrives -> path and details must be retained
        summary.record(ScanResult::new(
            "C:\\malware\\threat.exe",
            ScanStatus::Infected,
            None,
            Some("Win.Malware.Found".into()),
            500,
        ));

        assert_eq!(summary.scanned, MAX_RETAINED_RESULTS + 11);
        assert_eq!(summary.clean, MAX_RETAINED_RESULTS + 10);
        assert_eq!(summary.infected, 1);
        assert_eq!(summary.results.len(), 1);
        assert_eq!(summary.results[0].path, "C:\\malware\\threat.exe");
        assert_eq!(summary.results[0].status, ScanStatus::Infected);
        assert!(!summary.details_truncated);
    }

    #[tokio::test]
    async fn infected_result_evicts_non_infected_when_retained_is_full() {
        let mut summary = ScanSummary::default();
        for index in 0..MAX_RETAINED_RESULTS {
            summary.record(ScanResult::error(
                format!("error_{index}.txt"),
                "read error",
            ));
        }
        assert_eq!(summary.results.len(), MAX_RETAINED_RESULTS);
        assert_eq!(summary.errors, MAX_RETAINED_RESULTS);
        assert!(!summary.details_truncated);

        // Record an infected result when full -> evicts non-infected
        summary.record(ScanResult::new(
            "C:\\threats\\virus.exe",
            ScanStatus::Infected,
            None,
            Some("Trojan.Generic".into()),
            100,
        ));

        assert_eq!(summary.results.len(), MAX_RETAINED_RESULTS);
        assert_eq!(summary.infected, 1);
        assert!(summary.details_truncated);
        assert!(
            summary
                .results
                .iter()
                .any(|r| r.path == "C:\\threats\\virus.exe" && r.status == ScanStatus::Infected)
        );
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

    // --- Task V-6 Finding Q-4-01 TDD Tests ---

    /// (a) DriverData under the quick root is excluded, not counted, no row, allows complete scan
    #[tokio::test]
    async fn tdd_driverdata_under_quick_root_excluded() {
        let root = test_dir();
        let drivers = root.join("drivers");
        let driver_data = drivers.join("DriverData");
        std::fs::create_dir_all(&driver_data).unwrap();
        for core in quick::CORE_FILES {
            std::fs::write(root.join(core), b"mock core").unwrap();
        }
        std::fs::write(drivers.join("test.sys"), b"mock driver").unwrap();
        std::fs::write(driver_data.join("hidden.log"), b"should be skipped").unwrap();

        let targets = quick::targets_for_root(&root);
        let exclusions = quick::exclusions_for_root(&root);

        let (_tx, cancel) = watch::channel(false);
        let scan: ScanFn = Box::new(|path, _| {
            Box::pin(async move { ScanResult::new(path, ScanStatus::Clean, None, None, 1) })
        });

        let summary =
            scan_roots_with_exclusions(targets, exclusions, cancel, scan, Box::new(|_, _| {}))
                .await;

        assert!(summary.complete, "Quick scan should be complete");
        assert_eq!(summary.errors, 0);
        assert_eq!(summary.clean, 14); // 13 core files + 1 driver (test.sys)
        assert!(
            !summary
                .results
                .iter()
                .any(|r| { r.path.to_ascii_lowercase().contains("driverdata") })
        );
        let _ = std::fs::remove_dir_all(root);
    }

    /// (b) A different unreadable directory under drivers still yields incomplete
    #[tokio::test]
    async fn tdd_different_unreadable_dir_under_drivers_yields_incomplete() {
        let root = test_dir();
        let drivers = root.join("drivers");
        let driver_data = drivers.join("DriverData");
        let unreadable = drivers.join("unreadable_dir");
        std::fs::create_dir_all(&driver_data).unwrap();
        std::fs::create_dir_all(&unreadable).unwrap();
        std::fs::write(drivers.join("test.sys"), b"mock driver").unwrap();

        let _guard = DenyAccessGuard::new(&unreadable);

        let targets = quick::targets_for_root(&root);
        let exclusions = quick::exclusions_for_root(&root);

        let (_tx, cancel) = watch::channel(false);
        let scan: ScanFn = Box::new(|path, _| {
            Box::pin(async move { ScanResult::new(path, ScanStatus::Clean, None, None, 1) })
        });

        let summary =
            scan_roots_with_exclusions(targets, exclusions, cancel, scan, Box::new(|_, _| {}))
                .await;

        assert!(
            !summary.complete,
            "Scan must be incomplete when another dir is unreadable"
        );
        assert!(summary.errors >= 1);
        assert!(summary.results.iter().any(|r| {
            r.status == ScanStatus::Error && r.path.to_ascii_lowercase().contains("unreadable_dir")
        }));
        // But DriverData must NOT have an error or row
        assert!(
            !summary
                .results
                .iter()
                .any(|r| { r.path.to_ascii_lowercase().contains("driverdata") })
        );
        drop(_guard);
        let _ = std::fs::remove_dir_all(root);
    }

    /// (c) A folder named DriverData elsewhere is NOT excluded
    #[tokio::test]
    async fn tdd_folder_named_driverdata_elsewhere_not_excluded() {
        let quick_root = test_dir();
        let other_root = test_dir();
        let other_driver_data = other_root.join("DriverData");
        std::fs::create_dir_all(&other_driver_data).unwrap();
        std::fs::write(other_driver_data.join("sample.bin"), b"scanned").unwrap();

        // Quick scan exclusion is based on quick_root (%SystemRoot%\System32\drivers\DriverData)
        let exclusions = quick::exclusions_for_root(&quick_root);

        let (_tx, cancel) = watch::channel(false);
        let scan: ScanFn = Box::new(|path, _| {
            Box::pin(async move {
                ScanResult::new(
                    path,
                    ScanStatus::Infected,
                    None,
                    Some("not excluded threat".into()),
                    1,
                )
            })
        });

        // Scanning the other directory must NOT exclude its DriverData folder
        let summary = scan_roots_with_exclusions(
            vec![other_root.clone()],
            exclusions,
            cancel,
            scan,
            Box::new(|_, _| {}),
        )
        .await;

        assert!(summary.complete);
        assert_eq!(summary.infected, 1);
        assert!(
            summary
                .results
                .iter()
                .any(|r| { r.path.to_ascii_lowercase().contains("sample.bin") })
        );
        let _ = std::fs::remove_dir_all(quick_root);
        let _ = std::fs::remove_dir_all(other_root);
    }

    /// (d) Folder scan of drivers still reports the denied subdir
    #[tokio::test]
    #[cfg(windows)]
    async fn tdd_folder_scan_of_drivers_reports_denied_subdir() {
        // Test against the real Windows %SystemRoot%\System32\drivers if available,
        // or a simulated drivers folder with an access-denied DriverData subdir.
        let sys_root = quick::system_root().unwrap();
        let real_drivers = sys_root.join("drivers");
        let real_driver_data = real_drivers.join("DriverData");

        if real_driver_data.exists() {
            let (_tx, cancel) = watch::channel(false);
            let scan: ScanFn = Box::new(|path, _| {
                Box::pin(async move { ScanResult::new(path, ScanStatus::Clean, None, None, 1) })
            });

            // Normal folder scan has NO exclusions
            let summary =
                scan_directory(real_drivers.clone(), cancel, scan, Box::new(|_, _| {})).await;

            // Must be incomplete because DriverData has SYSTEM-only ACL (os error 5)
            assert!(
                !summary.complete,
                "Folder scan of drivers must be incomplete on access-denied DriverData"
            );
            assert!(
                summary.errors >= 1,
                "Should have at least 1 error for DriverData"
            );
            assert!(
                summary.results.iter().any(|r| {
                    r.status == ScanStatus::Error
                        && r.path.to_ascii_lowercase().contains("driverdata")
                }),
                "Folder scan must report DriverData error"
            );
        } else {
            // Fallback for non-standard environments: simulated drivers directory with denied DriverData
            let root = test_dir();
            let drivers = root.join("drivers");
            let driver_data = drivers.join("DriverData");
            std::fs::create_dir_all(&driver_data).unwrap();
            let _guard = DenyAccessGuard::new(&driver_data);

            let (_tx, cancel) = watch::channel(false);
            let scan: ScanFn = Box::new(|path, _| {
                Box::pin(async move { ScanResult::new(path, ScanStatus::Clean, None, None, 1) })
            });

            let summary = scan_directory(drivers.clone(), cancel, scan, Box::new(|_, _| {})).await;
            assert!(!summary.complete);
            assert!(summary.errors >= 1);
            drop(_guard);
            let _ = std::fs::remove_dir_all(root);
        }
    }
}
