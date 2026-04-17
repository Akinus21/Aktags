use anyhow::Result;
use crossbeam_channel::{bounded, Sender};
use notify::{Config as NotifyConfig, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::db::{self, DbPool};
use crate::extractor;
use crate::tagger;
use crate::taxonomy;

#[derive(Debug, Clone)]
pub struct DaemonStats {
    pub processed: u64,
    pub errors: u64,
    pub queue_size: usize,
    pub current_file: Option<String>,
    pub running: bool,
}

impl Default for DaemonStats {
    fn default() -> Self {
        Self { processed: 0, errors: 0, queue_size: 0, current_file: None, running: false }
    }
}

#[derive(Debug)]
enum FileEvent {
    Process(PathBuf),
    Delete(PathBuf),
    RetagAll,
    Stop,
}

pub struct Daemon {
    config: Arc<Mutex<Config>>,
    pool: DbPool,
    stats: Arc<Mutex<DaemonStats>>,
    event_tx: Option<Sender<FileEvent>>,
}

impl Daemon {
    pub fn new(config: Config, pool: DbPool) -> Self {
        Self {
            config: Arc::new(Mutex::new(config)),
            pool,
            stats: Arc::new(Mutex::new(DaemonStats::default())),
            event_tx: None,
        }
    }

    pub fn stats(&self) -> DaemonStats {
        self.stats.lock().unwrap().clone()
    }

    pub fn retag_all(&self) {
        if let Some(tx) = &self.event_tx {
            let _ = tx.send(FileEvent::RetagAll);
        }
    }

    pub fn retag_file(&self, path: PathBuf) {
        if let Some(tx) = &self.event_tx {
            let _ = tx.send(FileEvent::Process(path));
        }
    }

    pub fn update_config(&self, new_config: Config) {
        *self.config.lock().unwrap() = new_config;
    }

    /// Start the daemon. Returns a shutdown sender.
    pub fn start(&mut self) -> Result<mpsc::Sender<()>> {
        let (event_tx, event_rx) = bounded::<FileEvent>(1024);
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        self.event_tx = Some(event_tx.clone());

        let config = Arc::clone(&self.config);
        let pool = self.pool.clone();
        let stats = Arc::clone(&self.stats);
        let event_tx_clone = event_tx.clone();

        // Spawn the main daemon tokio task
        tokio::spawn(async move {
            stats.lock().unwrap().running = true;
            info!("Daemon started");

            // Initial taxonomy seed
            if let Err(e) = taxonomy::init_taxonomy() {
                warn!("Taxonomy init failed: {e}");
            }

            // Start filesystem watcher
            let watch_dirs = config.lock().unwrap().watch_dirs.clone();
            let supported = config.lock().unwrap().supported_extensions.all()
                .into_iter().map(String::from).collect::<Vec<_>>();

            let tx_for_watcher = event_tx_clone.clone();
            let supported_clone = supported.clone();

            let mut watcher = RecommendedWatcher::new(
                move |result: notify::Result<Event>| {
                    let Ok(event) = result else { return };
                    match event.kind {
                        EventKind::Create(_) | EventKind::Modify(_) => {
                            for path in &event.paths {
                                if should_process(path, &supported_clone) {
                                    std::thread::sleep(Duration::from_millis(200));
                                    let _ = tx_for_watcher.send(FileEvent::Process(path.clone()));
                                }
                            }
                        }
                        EventKind::Remove(_) => {
                            for path in &event.paths {
                                let _ = tx_for_watcher.send(FileEvent::Delete(path.clone()));
                            }
                        }
                        _ => {}
                    }
                },
                NotifyConfig::default().with_poll_interval(Duration::from_secs(2)),
            ).expect("Failed to create filesystem watcher");

            for dir in &watch_dirs {
                if dir.exists() {
                    if let Err(e) = watcher.watch(dir, RecursiveMode::Recursive) {
                        warn!("Could not watch {}: {e}", dir.display());
                    } else {
                        info!("Watching: {}", dir.display());
                    }
                }
            }

            // Initial scan
            let scan_dirs = watch_dirs.clone();
            let scan_supported = supported.clone();
            let scan_tx = event_tx_clone.clone();
            tokio::spawn(async move {
                let mut count = 0;
                for dir in &scan_dirs {
                    for entry in walkdir::WalkDir::new(dir)
                        .follow_links(true)
                        .into_iter()
                        .filter_map(|e| e.ok())
                    {
                        let path = entry.path().to_owned();
                        if should_process(&path, &scan_supported) {
                            let _hash = db::file_hash(&path);
                            // will be checked again in process loop, this is just queuing
                            let _ = scan_tx.send(FileEvent::Process(path));
                            count += 1;
                        }
                    }
                }
                info!("Initial scan queued {count} files");
            });

            // Build HTTP client for Ollama
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .unwrap();

            // Process loop — poll channel in a blocking thread, check shutdown via flag
            let shutdown_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let flag_clone = Arc::clone(&shutdown_flag);

            tokio::spawn(async move {
                shutdown_rx.recv().await;
                flag_clone.store(true, std::sync::atomic::Ordering::Relaxed);
            });

            loop {
                if shutdown_flag.load(std::sync::atomic::Ordering::Relaxed) {
                    info!("Daemon shutting down");
                    break;
                }

                match event_rx.recv_timeout(Duration::from_millis(200)) {
                    Ok(event) => {
                        let cfg = config.lock().unwrap().clone();
                        match event {
                            FileEvent::Process(path) => {
                                process_file(&path, &cfg, &pool, &client, &stats).await;
                            }
                            FileEvent::Delete(path) => {
                                let _ = db::remove_file(&pool, path.to_str().unwrap_or(""));
                                info!("Removed: {}", path.display());
                            }
                            FileEvent::RetagAll => {
                                let dirs = cfg.watch_dirs.clone();
                                let sup: Vec<String> = cfg.supported_extensions.all()
                                    .into_iter().map(String::from).collect();
                                let tx = event_tx_clone.clone();
                                for dir in &dirs {
                                    for entry in walkdir::WalkDir::new(dir)
                                        .follow_links(true)
                                        .into_iter()
                                        .filter_map(|e| e.ok())
                                    {
                                        let path = entry.path().to_owned();
                                        if should_process(&path, &sup) {
                                            let _ = tx.send(FileEvent::Process(path));
                                        }
                                    }
                                }
                            }
                            FileEvent::Stop => break,
                        }
                    }
                    Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                        // yield back to tokio runtime briefly
                        tokio::task::yield_now().await;
                    }
                    Err(_) => break, // channel disconnected
                }
            }

            stats.lock().unwrap().running = false;
            info!("Daemon stopped");
        });

        Ok(shutdown_tx)
    }
}

fn should_process(path: &Path, supported: &[String]) -> bool {
    if !path.is_file() { return false; }
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    if name.starts_with('.') { return false; }
    // Skip .TagStudio internals
    if path.components().any(|c| c.as_os_str() == ".TagStudio") { return false; }
    let ext = path.extension()
        .map(|e| format!(".{}", e.to_string_lossy().to_lowercase()))
        .unwrap_or_default();
    supported.iter().any(|s| s == &ext)
}

async fn process_file(
    path: &Path,
    config: &Config,
    pool: &DbPool,
    client: &reqwest::Client,
    stats: &Arc<Mutex<DaemonStats>>,
) {
    let path_str = path.to_str().unwrap_or("");
    let hash = db::file_hash(path);

    // Check if reindex needed
    match db::needs_reindex(pool, path_str, &hash) {
        Ok(false) if !config.retag_on_modify => {
            debug!("Skipping unchanged: {}", path.display());
            return;
        }
        Err(e) => { error!("DB check failed for {}: {e}", path.display()); return; }
        _ => {}
    }

    let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
    stats.lock().unwrap().current_file = Some(filename.clone());
    info!("Processing: {filename}");

    let ext = path.extension()
        .map(|e| format!(".{}", e.to_string_lossy().to_lowercase()))
        .unwrap_or_default();
    let category = config.supported_extensions.category(&ext);
    let size_bytes = path.metadata().map(|m| m.len() as i64).unwrap_or(0);

    // Extract content
    let content = tokio::task::spawn_blocking({
        let path = path.to_owned();
        let cat = category.to_string();
        let max = config.max_content_chars;
        let ocr = config.ocr_enabled;
        move || extractor::extract(&path, &cat, max, ocr)
    }).await.unwrap_or_default();

    // Load taxonomy
    let taxonomy = taxonomy::load_taxonomy();
    let approved = taxonomy::approved_tags(&taxonomy);

    // Tag with Ollama
    match tagger::tag_file(
        client,
        &config.ollama_base_url,
        &config.ollama_model,
        &filename,
        category,
        &ext,
        &content,
        size_bytes,
        &approved,
    ).await {
        Ok((summary, ai_tags)) => {
            // Resolve tags against taxonomy
            let mut pending = taxonomy::load_pending();
            let (good_tags, new_tags) = taxonomy::resolve_tags(&ai_tags, &filename, &taxonomy, &mut pending);

            if !new_tags.is_empty() {
                info!("Pending approval: {filename} → new tags [{}]", new_tags.join(", "));
                let _ = taxonomy::save_pending(&pending);
            }

            match db::upsert_file(pool, path_str, category, &summary, &good_tags, size_bytes, &hash, None) {
                Ok(_) => {
                    let mut s = stats.lock().unwrap();
                    s.processed += 1;
                    s.current_file = None;
                    info!("Tagged: {filename} → [{}]", good_tags.join(", "));
                }
                Err(e) => {
                    error!("DB write failed for {filename}: {e}");
                    let _ = db::upsert_file(pool, path_str, category, "", &[], size_bytes, &hash, Some(&e.to_string()));
                    let mut s = stats.lock().unwrap();
                    s.errors += 1;
                    s.current_file = None;
                }
            }
        }
        Err(e) => {
            error!("Failed to tag {filename}: {e}");
            let _ = db::upsert_file(pool, path_str, category, "", &[], size_bytes, &hash, Some(&e.to_string()));
            let mut s = stats.lock().unwrap();
            s.errors += 1;
            s.current_file = None;
        }
    }

    // Update queue size
    stats.lock().unwrap().queue_size = 0; // approximate — crossbeam doesn't expose len easily
}
