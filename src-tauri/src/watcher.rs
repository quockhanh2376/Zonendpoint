use anyhow::Result;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, mpsc},
    thread,
    time::Duration,
};
use tokio::sync::{Semaphore, watch};

#[derive(Default)]
pub struct Guard {
    watcher: Option<RecommendedWatcher>,
}
impl Guard {
    pub fn is_running(&self) -> bool {
        self.watcher.is_some()
    }
    pub fn stop(&mut self) {
        self.watcher.take();
    }
    pub fn start(
        &mut self,
        cache_path: PathBuf,
        addr: SocketAddr,
        semaphore: Arc<Semaphore>,
    ) -> Result<()> {
        if self.watcher.is_some() {
            return Ok(());
        }
        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx)?;
        let home = std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        for dir in [
            home.join("Downloads"),
            home.join("Desktop"),
            std::env::temp_dir(),
        ] {
            if dir.exists() {
                watcher.watch(&dir, RecursiveMode::NonRecursive)?;
            }
        }
        thread::spawn(move || {
            while let Ok(Ok(event)) = rx.recv() {
                if matches!(event.kind, EventKind::Create(_)) {
                    for path in event.paths {
                        let name = path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_lowercase();
                        if [".crdownload", ".tmp", ".part", ".lock"]
                            .iter()
                            .any(|s| name.ends_with(s))
                        {
                            continue;
                        }
                        // Wait for writers to close; scanning is wired to the same scanner used by commands.
                        let cache_path = cache_path.clone();
                        let semaphore = semaphore.clone();
                        thread::spawn(move || {
                            if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                                .enable_all()
                                .build()
                            {
                                rt.block_on(async move {
                                    tokio::time::sleep(Duration::from_millis(300)).await;
                                    let (_cancel, receiver) = watch::channel(false);
                                    let _ = crate::scanner::scan_file(
                                        path, cache_path, addr, receiver, semaphore,
                                    )
                                    .await;
                                });
                            }
                        });
                    }
                }
            }
        });
        self.watcher = Some(watcher);
        Ok(())
    }
}
