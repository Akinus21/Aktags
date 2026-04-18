use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub watch_dirs: Vec<PathBuf>,
    pub db_path: PathBuf,
    pub ollama_base_url: String,
    pub ollama_model: String,
    pub max_content_chars: usize,
    pub retag_on_modify: bool,
    pub ocr_enabled: bool,
    pub supported_extensions: SupportedExtensions,
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportedExtensions {
    pub documents: Vec<String>,
    pub images: Vec<String>,
    pub code: Vec<String>,
    pub audio: Vec<String>,
    pub video: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub window_width: u32,
    pub window_height: u32,
    pub thumbnail_size: u32,
    pub sidebar_width: u32,
    pub detail_panel_width: u32,
    pub theme: String,
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}

impl Default for Config {
    fn default() -> Self {
        Self {
            watch_dirs: vec![home_dir().join("Documents")],
            db_path: config_dir().join("aktags.db"),
            ollama_base_url: String::new(),
            ollama_model: String::new(),
            max_content_chars: 4000,
            retag_on_modify: true,
            ocr_enabled: true,
            supported_extensions: SupportedExtensions::default(),
            ui: UiConfig::default(),
        }
    }
}

impl Default for SupportedExtensions {
    fn default() -> Self {
        Self {
            documents: vec![
                ".pdf", ".doc", ".docx", ".odt", ".txt", ".md",
                ".rtf", ".csv", ".xlsx", ".xls", ".pptx", ".ppt",
            ].into_iter().map(String::from).collect(),
            images: vec![
                ".jpg", ".jpeg", ".png", ".gif", ".bmp",
                ".tiff", ".webp", ".svg",
            ].into_iter().map(String::from).collect(),
            code: vec![
                ".py", ".js", ".ts", ".sh", ".bash", ".c", ".cpp",
                ".h", ".java", ".go", ".rs", ".rb", ".php",
                ".yaml", ".yml", ".json", ".toml", ".ini", ".conf",
            ].into_iter().map(String::from).collect(),
            audio: vec![".mp3", ".wav", ".flac", ".ogg", ".m4a", ".aac"]
                .into_iter().map(String::from).collect(),
            video: vec![".mp4", ".mkv", ".avi", ".mov", ".webm", ".flv"]
                .into_iter().map(String::from).collect(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            window_width: 1400,
            window_height: 900,
            thumbnail_size: 120,
            sidebar_width: 240,
            detail_panel_width: 300,
            theme: "Dark".to_string(),
        }
    }
}

impl SupportedExtensions {
    pub fn all(&self) -> Vec<&str> {
        self.documents.iter()
            .chain(self.images.iter())
            .chain(self.code.iter())
            .chain(self.audio.iter())
            .chain(self.video.iter())
            .map(|s| s.as_str())
            .collect()
    }

    pub fn category(&self, ext: &str) -> &'static str {
        let ext = ext.to_lowercase();
        if self.documents.iter().any(|e| e == &ext) { return "documents"; }
        if self.images.iter().any(|e| e == &ext)    { return "images"; }
        if self.code.iter().any(|e| e == &ext)      { return "code"; }
        if self.audio.iter().any(|e| e == &ext)     { return "audio"; }
        if self.video.iter().any(|e| e == &ext)     { return "video"; }
        "other"
    }
}

pub fn config_dir() -> PathBuf {
    home_dir().join(".aktags")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn taxonomy_path() -> PathBuf {
    config_dir().join("tags.json")
}

pub fn pending_path() -> PathBuf {
    config_dir().join("pending.json")
}

pub fn load() -> Result<Config> {
    let path = config_path();

    // Apply env var overrides after loading
    let mut config = if path.exists() {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Reading config from {}", path.display()))?;
        toml::from_str(&content)
            .with_context(|| "Parsing config.toml")?
    } else {
        Config::default()
    };

    // Migrate: if watch_dirs is empty, seed from Documents
    if config.watch_dirs.is_empty() {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        config.watch_dirs.push(home.join("Documents"));
    }

    // Env var overrides
    if let Ok(url) = std::env::var("FILETAGGER_OLLAMA_URL") {
        if !url.is_empty() { config.ollama_base_url = url; }
    }
    if let Ok(model) = std::env::var("FILETAGGER_OLLAMA_MODEL") {
        if !model.is_empty() { config.ollama_model = model; }
    }
    if let Ok(dirs_str) = std::env::var("FILETAGGER_WATCH_DIRS") {
        let dirs: Vec<PathBuf> = dirs_str.split(':')
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .collect();
        if !dirs.is_empty() { config.watch_dirs = dirs; }
    }

    Ok(config)
}

pub fn save(config: &Config) -> Result<()> {
    let path = config_path();
    std::fs::create_dir_all(path.parent().unwrap())?;
    let content = toml::to_string_pretty(config)?;
    std::fs::write(&path, content)?;
    Ok(())
}

pub fn needs_first_run(config: &Config) -> bool {
    config.ollama_base_url.is_empty() || config.ollama_model.is_empty()
}
