use iced::{
    widget::{
        button, column, container, row, scrollable, text,
        text_input, toggler, Column, Space,
    },
    Alignment, Element, Length,
};
use std::path::PathBuf;

use super::app::{AkTags, Message, Panel, SyncStatus};
use super::theme::{self, ThemeType};

// ── First-run screen ──────────────────────────────────────────────────────────

pub fn view_first_run(app: &AkTags) -> Element<'_, Message> {
    let t = app.theme_type;
    let colors = theme::default_colors(t);

    let content = column![
        Space::with_height(Length::Fill),

        container(
            column![
                text("Welcome to AkTags").size(28)
                    .color(colors.accent()),
                Space::with_height(8.0),
                text("AI-powered tag-based file browser")
                    .size(14)
                    .color(colors.text_dim()),

                Space::with_height(32.0),

                // Ollama URL
                text("Ollama Base URL").size(12)
                    .color(colors.text_dim()),
                Space::with_height(6.0),
                text_input("https://ollama.akinus21.com", &app.first_run_url)
                    .on_input(Message::FirstRunOllamaUrlChanged)
                    .padding([10, 14])
                    .width(400.0),

                Space::with_height(16.0),

                // Model
                text("Ollama Model").size(12)
                    .color(colors.text_dim()),
                Space::with_height(4.0),
                text("Run 'ollama list' on your server to see available models.")
                    .size(11)
                    .color(colors.text_dim()),
                Space::with_height(6.0),
                text_input("gpt-oss:20b-cloud", &app.first_run_model)
                    .on_input(Message::FirstRunModelChanged)
                    .padding([10, 14])
                    .width(400.0),

                Space::with_height(16.0),

                // Watch directory
                text("Watch Directory").size(12)
                    .color(colors.text_dim()),
                Space::with_height(4.0),
                text("AkTags will monitor this folder and tag all files automatically.")
                    .size(11)
                    .color(colors.text_dim()),
                Space::with_height(6.0),
                text_input("~/Documents", &app.first_run_watch)
                    .on_input(Message::FirstRunWatchDirChanged)
                    .padding([10, 14])
                    .width(400.0),

                Space::with_height(32.0),

                button(
                    text("Get Started").size(15)
                        .color(colors.text())
                )
                .on_press(Message::FirstRunComplete)
                .padding([12, 32])
                .style(|_theme, _status| button::Style::default()),
            ]
            .spacing(0)
            .align_x(Alignment::Start)
            .padding(40),
        ),

        Space::with_height(Length::Fill),
    ]
    .align_x(Alignment::Center)
    .width(Length::Fill)
    .height(Length::Fill);

    container(content)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_theme| container::Style {
            background: Some(theme::default_colors(t).bg().into()),
            ..Default::default()
        })
        .into()
}

// ── Settings panel ────────────────────────────────────────────────────────────

pub fn view(app: &AkTags) -> Element<'_, Message> {
    let t = app.theme_type;
    let colors = theme::default_colors(t);

    let header = row![
        text("Settings").size(20).color(colors.text()),
        Space::with_width(Length::Fill),
        button(text("<- Back").size(13).color(colors.text()))
            .on_press(Message::SwitchPanel(Panel::Browser))
            .padding([6, 14])
            .style(|_theme, _status| button::Style::default()),
    ]
    .align_y(Alignment::Center)
    .padding([16, 20]);

    let content = column![
        // ── Ollama ────────────────────────────────────────────────────────
        section_header("Ollama Connection".to_string(), colors),

        label("Base URL".to_string(), colors),
        text_input("https://ollama.akinus21.com", &app.settings_ollama_url)
            .on_input(Message::OllamaUrlChanged)
            .padding([8, 12])
            .width(400.0),

        Space::with_height(12.0),
        label("Model".to_string(), colors),
        text_input("gpt-oss:20b-cloud", &app.settings_ollama_model)
            .on_input(Message::OllamaModelChanged)
            .padding([8, 12])
            .width(400.0),

        Space::with_height(24.0),

        // ── Watch Directories ─────────────────────────────────────────────
        section_header("Watch Directories".to_string(), colors),

        {
            let dir_rows: Vec<Element<'_, Message>> = app.config.watch_dirs.iter()
                .map(|dir| watch_dir_row(dir, colors))
                .collect();
            Element::from(Column::with_children(dir_rows).spacing(6))
        },

        Space::with_height(8.0),
        row![
            text_input("~/Downloads or /path/to/folder", &app.settings_watch_dir_input)
                .on_input(Message::WatchDirInputChanged)
                .on_submit(Message::WatchDirAdd(app.settings_watch_dir_input.clone()))
                .padding([8, 12])
                .width(320.0),
            Space::with_width(8.0),
            button(text("+ Add").size(13).color(colors.accent()))
                .on_press(Message::WatchDirAdd(app.settings_watch_dir_input.clone()))
                .padding([8, 14])
                .style(|_theme, _status| button::Style::default()),
        ]
        .align_y(Alignment::Center),

        Space::with_height(24.0),

        // ── Cloud Sync ────────────────────────────────────────────────────
        section_header("Cloud Sync".to_string(), colors),

        row![
            text("Enabled").size(12).color(colors.text_dim()),
            Space::with_width(Length::Fill),
            toggler(
                app.settings_cloud_enabled,
            )
            .on_toggle(Message::CloudEnabledToggled),
        ]
        .align_y(Alignment::Center),

        Space::with_height(8.0),
        label("Server URL".to_string(), colors),
        text_input("https://cloud.akinus21.com", &app.settings_cloud_url)
            .on_input(Message::CloudUrlChanged)
            .padding([8, 12])
            .width(400.0),

        Space::with_height(8.0),
        label("API Key".to_string(), colors),
        text_input(
            "Enter API key...",
            if app.settings_cloud_api_key.is_empty() { "" } else { "••••••••" }
        )
        .on_input(Message::CloudApiKeyChanged)
        .padding([8, 12])
        .width(400.0),

        Space::with_height(8.0),
        // Sync status row
        {
            let (status_label, status_color) = match &app.sync_status {
                SyncStatus::Idle => ("Not configured".to_string(), colors.text_dim()),
                SyncStatus::Connecting => ("Connecting...".to_string(), colors.text_dim()),
                SyncStatus::Synced => ("Synced".to_string(), colors.green()),
                SyncStatus::Syncing => ("Syncing...".to_string(), colors.accent()),
                SyncStatus::Error(e) => (format!("Error: {}", e), colors.red()),
            };
            row![
                text("Status").size(12).color(colors.text_dim()),
                Space::with_width(Length::Fill),
                text(status_label).size(12).color(status_color),
            ]
            .align_y(Alignment::Center)
            .into()
        },

        Space::with_height(8.0),
        button(text("Sync Now").size(13).color(colors.accent()))
            .on_press(Message::SyncNow)
            .padding([8, 20])
            .style(|_theme, _status| button::Style::default()),

        Space::with_height(24.0),

        // ── Daemon ────────────────────────────────────────────────────────
        section_header("Daemon".to_string(), colors),

        {
            let s = &app.daemon_stats;
            column![
                stat_row("Status".to_string(), if s.running { "Running" } else { "Stopped" }.to_string(), colors),
                stat_row("Processed".to_string(), s.processed.to_string(), colors),
                stat_row("Errors".to_string(), s.errors.to_string(), colors),
                stat_row("Queue".to_string(), s.queue_size.to_string(), colors),
                if let Some(f) = &s.current_file {
                    Element::from(stat_row("Current".to_string(), f.clone(), colors))
                } else {
                    Element::from(Space::with_height(0.0))
                },
            ]
            .spacing(4)
        },

        Space::with_height(24.0),

        // ── Theme ────────────────────────────────────────────────────────────
        section_header("Appearance".to_string(), colors),
        row![
            theme_button("Dark", "Dark", app.theme_type == ThemeType::Dark, colors),
            Space::with_width(8.0),
            theme_button("Light", "Light", app.theme_type == ThemeType::Light, colors),
            Space::with_width(8.0),
            theme_button("PurpleHaze", "PurpleHaze", app.theme_type == ThemeType::PurpleHaze, colors),
            Space::with_width(8.0),
            theme_button("Noctalia", "Noctalia", app.theme_type == ThemeType::Noctalia, colors),
        ],

        Space::with_height(24.0),

        // ── Updates ────────────────────────────────────────────────────────────
        section_header("Updates".to_string(), colors),
        row![
            text(format!("Version {}", crate::updater::current_version())).size(12).color(colors.text_dim()),
            Space::with_width(Length::Fill),
            match &app.update_status {
                crate::updater::UpdateStatus::UpToDate => {
                    Element::from(text("Up to date").size(12).color(colors.green()))
                }
                crate::updater::UpdateStatus::Available { version, .. } => {
                    row![
                        text(format!("Update available: v{}", version)).size(12)
                            .color(colors.accent()),
                        Space::with_width(8.0),
                        button(text("Download").size(11).color(colors.text()))
                            .on_press(Message::UpdateDownload)
                            .padding([4, 10])
                            .style(|_theme, _status| button::Style::default()),
                    ]
                    .into()
                }
                crate::updater::UpdateStatus::Downloading { version, progress } => {
                    row![
                        text(format!("Downloading v{}... {:.0}%", version, progress)).size(12)
                            .color(colors.accent2()),
                    ]
                    .into()
                }
                crate::updater::UpdateStatus::Ready { version, .. } => {
                    row![
                        text(format!("v{} ready to install", version)).size(12)
                            .color(colors.green()),
                        Space::with_width(8.0),
                        button(text("Install & Restart").size(11).color(colors.text()))
                            .on_press(Message::UpdateInstall)
                            .padding([4, 10])
                            .style(|_theme, _status| button::Style::default()),
                    ]
                    .into()
                }
                crate::updater::UpdateStatus::Error(e) => {
                    Element::from(text(format!("Error: {}", e)).size(12).color(colors.red()))
                }
            },
        ],

        Space::with_height(8.0),

        {
            let check_btn = button(text("Check for Updates").size(12).color(colors.text()))
                .on_press(Message::CheckForUpdate)
                .padding([6, 14])
                .style(|_theme, _status| button::Style::default());
            if matches!(app.update_status, crate::updater::UpdateStatus::UpToDate) {
                Element::from(check_btn)
            } else {
                Element::from(Space::with_height(0.0))
            }
        },

        Space::with_height(24.0),

        // ── Save / Actions ────────────────────────────────────────────────
        row![
            button(text("Save Settings").size(13).color(colors.accent()))
                .on_press(Message::SaveSettings)
                .padding([8, 20])
                .style(|_theme, _status| button::Style::default()),
            Space::with_width(12.0),
            button(text("Re-tag All Files").size(13).color(colors.text()))
                .on_press(Message::RetagAll)
                .padding([8, 20])
                .style(|_theme, _status| button::Style::default()),
        ],

        if let Some(msg) = &app.status_message {
            Element::from(text(msg).size(12).color(colors.green()))
        } else {
            Element::from(Space::with_height(0.0))
        },
    ]
    .spacing(4)
    .padding([20, 20]);

    column![
        header,
        scrollable(content).height(Length::Fill),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn watch_dir_row(dir: &PathBuf, colors: theme::ThemeColors) -> Element<'static, Message> {
    let dir_str = dir.to_string_lossy().to_string();
    row![
        text(dir_str).size(13).color(colors.text()).width(Length::Fill),
        button(text("x").size(14).color(colors.red()))
            .on_press(Message::WatchDirRemove(dir.clone()))
            .padding([3, 8])
            .style(|_theme, _status| button::Style::default()),
    ]
    .align_y(Alignment::Center)
    .spacing(8)
    .padding([6, 10])
    .into()
}

fn section_header(title: String, colors: theme::ThemeColors) -> Element<'static, Message> {
    column![
        text(title).size(13).color(colors.accent2()),
        Space::with_height(8.0),
    ]
    .into()
}

fn theme_button<'a>(label: &'a str, theme_name: &'a str, is_active: bool, colors: theme::ThemeColors) -> Element<'a, Message> {
    button(
        text(label).size(13).color(if is_active {
            colors.accent()
        } else {
            colors.text_dim()
        })
    )
    .on_press(Message::ThemeChanged(theme_name.to_string()))
    .padding([8, 16])
    .style(move |_theme, _status| button::Style {
        background: if is_active { Some(colors.surface2().into()) } else { None },
        ..Default::default()
    })
    .into()
}

fn label(s: String, colors: theme::ThemeColors) -> Element<'static, Message> {
    text(s).size(11)
        .color(colors.text_dim())
        .into()
}

fn stat_row(label: String, value: String, colors: theme::ThemeColors) -> Element<'static, Message> {
    row![
        text(label).size(12)
            .color(colors.text_dim())
            .width(100.0),
        text(value).size(12).color(colors.text()),
    ]
    .spacing(8)
    .into()
}
