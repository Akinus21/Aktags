mod config;
mod daemon;
mod db;
mod extractor;
mod tagger;
mod taxonomy;
mod ui;

use anyhow::Result;
use tracing::info;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("aktags=info,warn"))
        )
        .init();

    info!("AkTags starting...");

    let cfg = config::load().unwrap_or_else(|e| {
        tracing::warn!("Config load failed ({e}), using defaults");
        config::Config::default()
    });

    std::fs::create_dir_all(config::config_dir())?;
    let pool = db::create_pool(&cfg.db_path)?;

    ui::app::run(cfg, pool)
}
