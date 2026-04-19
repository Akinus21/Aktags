mod config;
mod daemon;
mod db;
mod extractor;
mod tagger;
mod taxonomy;
mod ui;
mod updater;

use anyhow::Result;
use std::env;
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber::EnvFilter;

const DAEMON_LOCKFILE: &str = "aktags-daemon.pid";

fn daemon_lockfile_path() -> Option<PathBuf> {
    env::var("XDG_RUNTIME_DIR")
        .ok()
        .map(|p| PathBuf::from(p).join(DAEMON_LOCKFILE))
        .or_else(|| env::var("TMPDIR").ok().map(|p| PathBuf::from(p).join(DAEMON_LOCKFILE)))
}

fn check_lockfile() -> Result<Option<u32>> {
    let path = daemon_lockfile_path().context("Could not determine lockfile path")?;

    if !path.exists() {
        return Ok(None);
    }

    let pid: u32 = std::fs::read_to_string(&path)
        .context("Failed to read lockfile")?
        .trim()
        .parse()
        .context("Failed to parse PID in lockfile")?;

    // Check if process is running
    if pid_exists(pid) {
        return Ok(Some(pid));
    }

    // Stale lockfile - remove it
    drop(path);
    std::fs::remove_file(daemon_lockfile_path().unwrap()).ok();
    Ok(None)
}

fn pid_exists(pid: u32) -> bool {
    // On Linux, sending signal 0 checks if process exists
    unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
}

fn write_lockfile() -> Result<()> {
    let path = daemon_lockfile_path().context("Could not determine lockfile path")?;
    let pid = std::process::id();
    std::fs::write(&path, pid.to_string())?;
    info!("Wrote lockfile: {} (PID={})", path.display(), pid);
    Ok(())
}

fn remove_lockfile() {
    if let Some(path) = daemon_lockfile_path() {
        std::fs::remove_file(&path).ok();
    }
}

fn print_help() {
    eprintln!("AkTags - AI-powered tag-based file browser");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  aktags           Start GUI (default)");
    eprintln!("  aktags --daemon  Start daemon only");
    eprintln!("  aktags --help    Show this help");
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    // Check for --help first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return Ok(());
    }

    // Check for --daemon flag
    let daemon_only = args.iter().any(|a| a == "--daemon");

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("aktags=info,warn"))
        )
        .init();

    info!("AkTags starting...");

    if daemon_only {
        run_daemon()?;
    } else {
        run_gui()?;
    }

    Ok(())
}

fn run_daemon() -> Result<()> {
    use anyhow::Context;

    // Check for existing daemon
    if let Some(pid) = check_lockfile()? {
        anyhow::bail!("Daemon already running (PID {}). Exiting.", pid);
    }

    // Write lockfile
    write_lockfile()?;

    // Ensure clean exit
    struct ExitGuard;
    impl Drop for ExitGuard {
        fn drop(&mut self) {
            remove_lockfile();
        }
    }
    let _guard = ExitGuard;

    info!("Starting daemon mode...");

    let cfg = config::load().unwrap_or_else(|e| {
        tracing::warn!("Config load failed ({e}), using defaults");
        config::Config::default()
    });

    std::fs::create_dir_all(config::config_dir())?;
    let pool = db::create_pool(&cfg.db_path)?;

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let mut daemon = daemon::Daemon::new(cfg, pool);
        let shutdown_tx = daemon.start()?;
        info!("Daemon running. Press Ctrl+C to stop.");

        // Wait for SIGINT/SIGTERM
        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        info!("Shutdown signal received");
        let _ = shutdown_tx.send(()).await;
    });

    info!("Daemon stopped");
    Ok(())
}

fn run_gui() -> Result<()> {
    let cfg = config::load().unwrap_or_else(|e| {
        tracing::warn!("Config load failed ({e}), using defaults");
        config::Config::default()
    });

    std::fs::create_dir_all(config::config_dir())?;
    let pool = db::create_pool(&cfg.db_path)?;

    ui::app::run(cfg, pool)?;

    Ok(())
}
