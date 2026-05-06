use iced::{
    Element, Subscription, Task, Theme, time,
};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

use crate::config::{self, Config};
use crate::daemon::{Daemon, DaemonStats};
use crate::db::{self, DbPool, FileRecord, SearchFilter};
use crate::icon::IconCache;
use crate::taxonomy;
use crate::ui::theme;
use crate::updater::{UpdateStatus as UpdaterStatus, check_for_update_async};

pub fn run(cfg: Config, pool: DbPool) -> iced::Result {
    let (app, cmd) = AkTags::new((cfg, pool));

    iced::application("AkTags", AkTags::update, AkTags::view)
        .subscription(AkTags::subscription)
        .theme(AkTags::theme)
        .run_with(move || (app, cmd))
}

// ── Panels ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Panel {
    Browser,
    Pending,
    Taxonomy,
    Settings,
    FirstRun,
}

// ── Messages ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    SwitchPanel(Panel),
    SearchChanged(String),
    SearchSubmit,
    TagToggled(String),
    CategorySelected(Option<String>),
    FileSelected(i64),
    FileOpened(i64),
    FileDeselected,
    ClearFilters,
    ViewToggled,
    SortChanged(SortField),
    FilesLoaded(Vec<FileRecord>),
    TagsLoaded(Vec<(String, i64)>),
    StatsLoaded(crate::db::DbStats),
    AddTagToFile(i64, String),
    RemoveTagFromFile(i64, String),
    TagInputChanged(String),
    TagInputSubmit,
    PendingLoaded(Vec<(String, crate::taxonomy::PendingTag)>),
    PendingApprove(String, String),
    PendingReject(String),
    PendingMerge(String, String),
    PendingMergeInputChanged(String, String),
    ApproveAll,
    RejectAll,
    TaxonomyLoaded(Vec<(String, crate::taxonomy::TagMeta)>),
    NewTagNameChanged(String),
    NewTagCategoryChanged(String),
    NewTagAliasesChanged(String),
    AddNewTag,
    RemoveTaxonomyTag(String),
    OllamaUrlChanged(String),
    OllamaModelChanged(String),
    WatchDirAdd(String),
    WatchDirRemove(PathBuf),
    WatchDirInputChanged(String),
    SaveSettings,
    RetagAll,
    ThemeChanged(String),
    StartDaemon,
    FirstRunOllamaUrlChanged(String),
    FirstRunModelChanged(String),
    FirstRunWatchDirChanged(String),
    FirstRunComplete,
    DaemonStatsRefreshed(DaemonStats),
    FileRecordLoaded(Option<FileRecord>),
    Tick,
    CheckForUpdate,
    UpdateCheckResult(crate::updater::UpdateStatus),
    UpdateDownload,
    UpdateInstall,
    SyncNow,
    SyncComplete,
    CloudUrlChanged(String),
    CloudApiKeyChanged(String),
    CloudEnabledToggled(bool),
    AutoUpdateToggled(bool),
    AutoUpdateCheck,
    BrewOutdated(bool),
    BrewUpgradeNow,
    BrewUpgradeResult(Result<(), String>),
    DiagnosticsToggled(bool),
    DiagnosticsWebhookChanged(String),
    SendDiagnosticsReport,
    DiagnosticsReportSent(Result<(), String>),
    FileDeleted(bool),
    DeleteFile(i64),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SortField {
    Name,
    Category,
    Size,
    Date,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode { Grid, List, Card }

// ── App state ─────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AkTags {
    pub config: Config,
    pub pool: DbPool,
    pub daemon: Arc<Mutex<Daemon>>,
    pub shutdown_tx: Option<mpsc::Sender<()>>,
    pub panel: Panel,
    pub view_mode: ViewMode,
    pub files: Vec<FileRecord>,
    pub all_tags: Vec<(String, i64)>,
    pub active_tags: Vec<String>,
    pub active_category: Option<String>,
    pub search_query: String,
    pub selected_file: Option<FileRecord>,
    pub tag_input: String,
    pub stats: Option<crate::db::DbStats>,
    pub pending: Vec<(String, crate::taxonomy::PendingTag)>,
    pub pending_merge_inputs: std::collections::HashMap<String, String>,
    pub taxonomy: Vec<(String, crate::taxonomy::TagMeta)>,
    pub new_tag_name: String,
    pub new_tag_category: String,
    pub new_tag_aliases: String,
    pub settings_ollama_url: String,
    pub settings_ollama_model: String,
    pub settings_watch_dir_input: String,
    pub settings_cloud_url: String,
    pub settings_cloud_api_key: String,
    pub settings_cloud_enabled: bool,
    pub first_run_url: String,
    pub first_run_model: String,
    pub first_run_watch: String,
    pub daemon_stats: DaemonStats,
    pub status_message: Option<String>,
    pub theme_type: theme::ThemeType,
    pub update_status: UpdaterStatus,
    pub sync_status: SyncStatus,
    pub sort_field: SortField,
    pub sort_direction: SortDirection,
    pub icon_cache: IconCache,
    pub settings_auto_update_enabled: bool,
    pub settings_diagnostics_enabled: bool,
    pub settings_diagnostics_webhook_url: String,
}

#[derive(Debug, Clone)]
pub enum SyncStatus {
    Idle,
    Connecting,
    Synced,
    Syncing,
    Error(String),
}

impl AkTags {
    pub fn new((config, pool): (Config, DbPool)) -> (Self, Task<Message>) {
        let is_first_run = config::needs_first_run(&config);
        let first_run_url = config.ollama_base_url.clone();
        let first_run_model = config.ollama_model.clone();
        let first_run_watch = config.watch_dirs.first()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "~/Documents".to_string());
        let settings_ollama_url = config.ollama_base_url.clone();
        let settings_ollama_model = config.ollama_model.clone();
        let saved_theme = config.ui.theme.clone();
        theme::ensure_default_themes();
        let theme_type = theme::ThemeType::from_string(&saved_theme);

        let daemon = Daemon::new(config.clone(), pool.clone());
        let initial_panel = if is_first_run { Panel::FirstRun } else { Panel::Browser };

        let app = Self {
            config: config.clone(),
            pool,
            daemon: Arc::new(Mutex::new(daemon)),
            shutdown_tx: None,
            panel: initial_panel,
            view_mode: ViewMode::List,
            files: vec![],
            all_tags: vec![],
            active_tags: vec![],
            active_category: None,
            search_query: String::new(),
            selected_file: None,
            tag_input: String::new(),
            stats: None,
            pending: vec![],
            pending_merge_inputs: Default::default(),
            taxonomy: vec![],
            new_tag_name: String::new(),
            new_tag_category: String::new(),
            new_tag_aliases: String::new(),
            settings_ollama_url,
            settings_ollama_model,
            settings_watch_dir_input: String::new(),
            settings_cloud_url: config.cloud.url.clone(),
            settings_cloud_api_key: config.cloud.api_key.clone(),
            settings_cloud_enabled: config.cloud.enabled,
            first_run_url,
            first_run_model,
            first_run_watch,
            daemon_stats: DaemonStats::default(),
            status_message: None,
            theme_type,
            update_status: UpdaterStatus::UpToDate,
            sync_status: SyncStatus::Idle,
            sort_field: SortField::Name,
            sort_direction: SortDirection::Ascending,
            icon_cache: IconCache::new(),
            settings_auto_update_enabled: config.auto_update.enabled,
            settings_diagnostics_enabled: config.diagnostics.enabled,
            settings_diagnostics_webhook_url: config.diagnostics.webhook_url.clone(),
        };

        let cmd = if app.panel == Panel::Browser {
            app.refresh_all()
        } else {
            Task::none()
        };

        let cmd = if is_first_run {
            cmd
        } else {
            Task::batch([
                cmd,
                Task::perform(async {}, |()| Message::StartDaemon),
            ])
        };

        (app, cmd)
    }

    pub fn title(&self) -> String {
        "AkTags".to_string()
    }

    pub fn theme(&self) -> Theme {
        theme::iced_theme(self.theme_type)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => return self.refresh_all(),

            Message::SwitchPanel(panel) => {
                self.panel = panel.clone();
                return match panel {
                    Panel::Pending  => self.load_pending(),
                    Panel::Taxonomy => self.load_taxonomy(),
                    Panel::Browser  => self.refresh_all(),
                    _ => Task::none(),
                };
            }

            Message::SearchChanged(q)  => { self.search_query = q; }
            Message::SearchSubmit      => return self.load_files(),

            Message::TagToggled(tag) => {
                if let Some(pos) = self.active_tags.iter().position(|t| t == &tag) {
                    self.active_tags.remove(pos);
                } else {
                    self.active_tags.push(tag);
                }
                return self.load_files();
            }

            Message::CategorySelected(cat) => {
                self.active_category = cat;
                return self.load_files();
            }

            Message::FileSelected(id) => {
                let pool = self.pool.clone();
                return Task::perform(
                    async move { db::get_file_by_id(&pool, id).ok().flatten() },
                    Message::FileRecordLoaded,
                );
            }

            Message::FileRecordLoaded(record) => {
                self.selected_file = record;
            }

            Message::FileOpened(id) => {
                let pool = self.pool.clone();
                return Task::perform(
                    async move {
                        if let Ok(Some(r)) = db::get_file_by_id(&pool, id) {
                            let _ = open::that(&r.path);
                        }
                    },
                    |_| Message::Tick,
                );
            }

            Message::FileDeselected => { self.selected_file = None; }

            Message::ClearFilters => {
                self.active_tags.clear();
                self.active_category = None;
                self.search_query.clear();
                return self.load_files();
            }

            Message::ViewToggled => {
                self.view_mode = match self.view_mode {
                    ViewMode::Grid => ViewMode::List,
                    ViewMode::List => ViewMode::Card,
                    ViewMode::Card => ViewMode::Grid,
                };
            }

            Message::SortChanged(field) => {
                if self.sort_field == field {
                    // Toggle direction if clicking same field
                    self.sort_direction = match self.sort_direction {
                        SortDirection::Ascending => SortDirection::Descending,
                        SortDirection::Descending => SortDirection::Ascending,
                    };
                } else {
                    self.sort_field = field;
                    self.sort_direction = SortDirection::Ascending;
                }
                // Sort files in place
                match self.sort_field {
                    SortField::Name => {
                        let desc = self.sort_direction == SortDirection::Descending;
                        self.files.sort_by(|a, b| {
                            let ord = a.filename.cmp(&b.filename);
                            if desc { ord.reverse() } else { ord }
                        });
                    }
                    SortField::Category => {
                        let desc = self.sort_direction == SortDirection::Descending;
                        self.files.sort_by(|a, b| {
                            let ord = a.category.cmp(&b.category);
                            if desc { ord.reverse() } else { ord }
                        });
                    }
                    SortField::Size => {
                        let desc = self.sort_direction == SortDirection::Descending;
                        self.files.sort_by(|a, b| {
                            let ord = a.size_bytes.cmp(&b.size_bytes);
                            if desc { ord.reverse() } else { ord }
                        });
                    }
                    SortField::Date => {
                        let desc = self.sort_direction == SortDirection::Descending;
                        self.files.sort_by(|a, b| {
                            let ord = a.tagged_at.cmp(&b.tagged_at);
                            if desc { ord.reverse() } else { ord }
                        });
                    }
                }
            }

            Message::FilesLoaded(files)   => { self.files = files; }
            Message::TagsLoaded(tags)     => { self.all_tags = tags; }
            Message::StatsLoaded(stats)   => { self.stats = Some(stats); }
            Message::TagInputChanged(s)   => { self.tag_input = s; }

            Message::TagInputSubmit => {
                if let Some(file) = &self.selected_file {
                    let tag = self.tag_input.trim().to_lowercase().replace(' ', "-");
                    if !tag.is_empty() {
                        let file_id = file.id;
                        let pool = self.pool.clone();
                        let mut tags = file.tags.clone();
                        if !tags.contains(&tag) {
                            tags.push(tag.clone());
                        }
                        self.tag_input.clear();
                        // Update DB
                        return Task::perform(
                            async move {
                                let _ = db::upsert_tags(&pool, file_id, &tags);
                                db::get_file_by_id(&pool, file_id).ok().flatten()
                            },
                            Message::FileRecordLoaded,
                        );
                    }
                }
            }

            Message::RemoveTagFromFile(id, tag) => {
                if let Some(file) = &self.selected_file {
                    let mut tags = file.tags.clone();
                    tags.retain(|t| t != &tag);
                    let pool = self.pool.clone();
                    return Task::perform(
                        async move {
                            let _ = db::upsert_tags(&pool, id, &tags);
                            db::get_file_by_id(&pool, id).ok().flatten()
                        },
                        Message::FileRecordLoaded,
                    );
                }
            }

            Message::PendingLoaded(p)  => { self.pending = p; }

            Message::PendingApprove(tag, cat) => {
                let _ = taxonomy::approve_pending(&tag, &cat);
                return self.load_pending();
            }
            Message::PendingReject(tag) => {
                let _ = taxonomy::reject_pending(&tag);
                return self.load_pending();
            }
            Message::PendingMerge(tag, into) => {
                let _ = taxonomy::merge_pending(&tag, &into);
                self.pending_merge_inputs.remove(&tag);
                return self.load_pending();
            }
            Message::PendingMergeInputChanged(tag, val) => {
                self.pending_merge_inputs.insert(tag, val);
            }
            Message::ApproveAll => {
                let pending = taxonomy::load_pending();
                for tag in pending.keys() { let _ = taxonomy::approve_pending(tag, "misc"); }
                return self.load_pending();
            }
            Message::RejectAll => {
                let pending = taxonomy::load_pending();
                for tag in pending.keys() { let _ = taxonomy::reject_pending(tag); }
                return self.load_pending();
            }

            Message::TaxonomyLoaded(tax) => { self.taxonomy = tax; }
            Message::NewTagNameChanged(s)     => { self.new_tag_name = s; }
            Message::NewTagCategoryChanged(s) => { self.new_tag_category = s; }
            Message::NewTagAliasesChanged(s)  => { self.new_tag_aliases = s; }

            Message::AddNewTag => {
                let tag = self.new_tag_name.trim().to_lowercase();
                if !tag.is_empty() {
                    let aliases: Vec<String> = self.new_tag_aliases
                        .split(',').map(|a| a.trim().to_lowercase())
                        .filter(|a| !a.is_empty()).collect();
                    let _ = taxonomy::add_tag(&tag, &self.new_tag_category, aliases);
                    self.new_tag_name.clear();
                    self.new_tag_aliases.clear();
                    return self.load_taxonomy();
                }
            }
            Message::RemoveTaxonomyTag(tag) => {
                let _ = taxonomy::remove_tag(&tag);
                return self.load_taxonomy();
            }

            Message::OllamaUrlChanged(s)     => { self.settings_ollama_url = s; }
            Message::OllamaModelChanged(s)   => { self.settings_ollama_model = s; }
            Message::WatchDirInputChanged(s) => { self.settings_watch_dir_input = s; }

            Message::CloudUrlChanged(s)      => { self.settings_cloud_url = s; }
            Message::CloudApiKeyChanged(s)   => { self.settings_cloud_api_key = s; }
            Message::CloudEnabledToggled(v)  => { self.settings_cloud_enabled = v; }

            Message::AutoUpdateToggled(v) => {
                self.settings_auto_update_enabled = v;
            }

            Message::DiagnosticsToggled(v) => {
                self.settings_diagnostics_enabled = v;
            }

            Message::DiagnosticsWebhookChanged(s) => {
                self.settings_diagnostics_webhook_url = s;
            }

            Message::SendDiagnosticsReport => {
                let webhook_url = self.settings_diagnostics_webhook_url.clone();
                return Task::perform(
                    async move {
                        let entries = crate::diagnostics::read_recent_log_errors();
                        crate::diagnostics::send_report(&webhook_url, entries).await
                            .map_err(|e| e.to_string())
                    },
                    Message::DiagnosticsReportSent,
                );
            }

            Message::AutoUpdateCheck => {
                return Task::perform(crate::auto_update::check_brew_outdated(), Message::BrewOutdated);
            }

            Message::BrewOutdated(is_outdated) => {
                if is_outdated {
                    self.status_message = Some("Update available via brew".into());
                }
            }

            Message::BrewUpgradeNow => {
                self.status_message = Some("Upgrading via brew...".into());
                return Task::perform(crate::auto_update::brew_upgrade(), Message::BrewUpgradeResult);
            }

            Message::BrewUpgradeResult(result) => {
                match result {
                    Ok(()) => {
                        self.status_message = Some("Upgrade complete. Restarting...".into());
                        crate::auto_update::restart_self();
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Upgrade failed: {}", e));
                    }
                }
            }

            Message::DiagnosticsReportSent(result) => {
                match result {
                    Ok(()) => {
                        self.status_message = Some("Diagnostics report sent".into());
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Diagnostics report failed: {}", e));
                    }
                }
            }

            Message::WatchDirAdd(dir) => {
                let path = PathBuf::from(shellexpand::tilde(&dir).as_ref());
                if !self.config.watch_dirs.contains(&path) {
                    self.config.watch_dirs.push(path);
                    let _ = config::save(&self.config);
                }
                self.settings_watch_dir_input.clear();
            }
            Message::WatchDirRemove(dir) => {
                self.config.watch_dirs.retain(|d| d != &dir);
                if self.config.watch_dirs.is_empty() {
                    self.config.watch_dirs.push(
                        dirs::home_dir().unwrap_or_default().join("Documents")
                    );
                }
                let _ = config::save(&self.config);
            }

            Message::SaveSettings => {
                self.config.ollama_base_url = self.settings_ollama_url.clone();
                self.config.ollama_model    = self.settings_ollama_model.clone();
                self.config.cloud.url       = self.settings_cloud_url.clone();
                self.config.cloud.api_key   = self.settings_cloud_api_key.clone();
                self.config.cloud.enabled   = self.settings_cloud_enabled;
                self.config.auto_update.enabled   = self.settings_auto_update_enabled;
                self.config.diagnostics.enabled  = self.settings_diagnostics_enabled;
                self.config.diagnostics.webhook_url = self.settings_diagnostics_webhook_url.clone();
                let _ = config::save(&self.config);
                self.daemon.lock().unwrap().update_config(self.config.clone());
                self.status_message = Some("Settings saved".into());
            }

            Message::RetagAll => {
                self.daemon.lock().unwrap().retag_all();
                self.status_message = Some("Re-tag queued for all files".into());
            }

            Message::ThemeChanged(theme_name) => {
                let new_theme_type = theme::ThemeType::from_string(&theme_name);
                self.theme_type = new_theme_type;
                self.config.ui.theme = theme_name;
                let _ = config::save(&self.config);
            }

            Message::StartDaemon => {
                let is_first_run = self.panel == Panel::FirstRun;
                if !is_first_run {
                    let mut daemon = self.daemon.lock().unwrap();
                    if let Ok(tx) = daemon.start() {
                        self.shutdown_tx = Some(tx);
                    }
                }
            }

            Message::SyncNow => {
                self.sync_status = SyncStatus::Connecting;
                let cfg = self.config.cloud.clone();
                let pool = self.pool.clone();
                let watch_dirs = self.config.watch_dirs.clone();
                return Task::perform(async move {
                    if cfg.enabled {
                        match crate::sync::identity::load_identity() {
                            Ok(identity) => {
                                match crate::sync::run_sync(&cfg, &pool, &identity, &watch_dirs).await {
                                    Ok(()) => Message::SyncComplete,
                                    Err(_e) => Message::SyncComplete,
                                }
                            }
                            Err(_) => Message::SyncComplete,
                        }
                    } else {
                        Message::SyncComplete
                    }
                }, |msg| msg);
            }

            Message::SyncComplete => {
                self.sync_status = SyncStatus::Synced;
            }

            Message::CheckForUpdate => {
                return Task::perform(check_for_update_async(), Message::UpdateCheckResult);
            }

            Message::UpdateCheckResult(status) => {
                self.update_status = status;
            }

            Message::UpdateDownload => {
                self.status_message = Some("Downloading update...".into());
            }

            Message::UpdateInstall => {
                self.status_message = Some("Installing update and restarting...".into());
            }

            Message::FirstRunOllamaUrlChanged(s) => { self.first_run_url = s; }
            Message::FirstRunModelChanged(s)     => { self.first_run_model = s; }
            Message::FirstRunWatchDirChanged(s)  => { self.first_run_watch = s; }

            Message::FirstRunComplete => {
                self.config.ollama_base_url = self.first_run_url.clone();
                self.config.ollama_model    = self.first_run_model.clone();
                let watch = PathBuf::from(
                    shellexpand::tilde(&self.first_run_watch).as_ref()
                );
                self.config.watch_dirs = vec![watch];
                let _ = config::save(&self.config);
                let mut daemon = self.daemon.lock().unwrap();
                daemon.update_config(self.config.clone());
                self.shutdown_tx = daemon.start().ok();
                drop(daemon);
                self.panel = Panel::Browser;
                return self.refresh_all();
            }

            Message::DaemonStatsRefreshed(stats) => { self.daemon_stats = stats; }

            Message::DeleteFile(file_id) => {
                let pool = self.pool.clone();
                let path = self.selected_file.as_ref()
                    .filter(|f| f.id == file_id)
                    .map(|f| f.path.clone());
                if let Some(path) = path {
                    return Task::perform(
                        async move {
                            // Soft delete in DB first
                            let _ = db::soft_delete_file(&pool, &path);
                            // Then delete actual file from disk
                            tokio::fs::remove_file(&path).await.is_ok()
                        },
                        Message::FileDeleted,
                    );
                }
            }

            Message::FileDeleted(ok) => {
                if ok {
                    self.selected_file = None;
                    return self.refresh_all();
                }
            }

            Message::AddTagToFile(..) => {}
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        match &self.panel {
            Panel::FirstRun  => super::settings::view_first_run(self),
            Panel::Browser   => super::browser::view(self),
            Panel::Pending   => super::tag_manager::view_pending(self),
            Panel::Taxonomy  => super::tag_manager::view_taxonomy(self),
            Panel::Settings  => super::settings::view(self),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let mut subs = vec![
            time::every(std::time::Duration::from_secs(3))
                .map(|_| Message::Tick),
            time::every(std::time::Duration::from_secs(3600))
                .map(|_| Message::CheckForUpdate),
        ];

        if self.config.auto_update.enabled {
            let interval = std::time::Duration::from_secs(self.config.auto_update.check_interval_secs);
            subs.push(
                time::every(interval).map(|_| Message::AutoUpdateCheck),
            );
        }

        Subscription::batch(subs)
    }

    // ── Helper commands ───────────────────────────────────────────────────────

    pub fn load_files(&self) -> Task<Message> {
        let pool = self.pool.clone();
        let filter = SearchFilter {
            query: if self.search_query.is_empty() { None }
                   else { Some(self.search_query.clone()) },
            tags: self.active_tags.clone(),
            category: self.active_category.clone(),
            limit: 500,
            offset: 0,
        };
        Task::perform(
            async move { db::search_files(&pool, &filter).unwrap_or_default() },
            Message::FilesLoaded,
        )
    }

    pub fn load_tags(&self) -> Task<Message> {
        let pool = self.pool.clone();
        Task::perform(
            async move { db::all_tags(&pool).unwrap_or_default() },
            Message::TagsLoaded,
        )
    }

    pub fn load_stats(&self) -> Task<Message> {
        let pool = self.pool.clone();
        Task::perform(
            async move {
                db::get_stats(&pool).unwrap_or(crate::db::DbStats {
                    total: 0, errors: 0, untagged: 0, by_category: vec![]
                })
            },
            Message::StatsLoaded,
        )
    }

    pub fn load_pending(&self) -> Task<Message> {
        Task::perform(async {
            let pending = taxonomy::load_pending();
            let mut items: Vec<_> = pending.into_iter().collect();
            items.sort_by(|a, b| b.1.file_count.cmp(&a.1.file_count));
            items
        }, Message::PendingLoaded)
    }

    pub fn load_taxonomy(&self) -> Task<Message> {
        Task::perform(async {
            let tax = taxonomy::load_taxonomy();
            let mut items: Vec<_> = tax.into_iter().collect();
            items.sort_by(|a, b| a.0.cmp(&b.0));
            items
        }, Message::TaxonomyLoaded)
    }

    pub fn refresh_daemon_stats(&self) -> Task<Message> {
        let stats = self.daemon.lock().unwrap().stats();
        Task::perform(async move { stats }, Message::DaemonStatsRefreshed)
    }

    pub fn refresh_all(&self) -> Task<Message> {
        Task::batch(vec![
            self.load_files(),
            self.load_tags(),
            self.load_stats(),
            self.refresh_daemon_stats(),
        ])
    }
}
