use iced::{
    widget::{button, column, container, row, scrollable, text, text_input, Column, Space},
    Alignment, Element, Length,
};
use std::path::PathBuf;

use super::{app::{AkTags, Message, Panel}, theme::*};

// ── First-run screen ──────────────────────────────────────────────────────────

pub fn view_first_run(app: &AkTags) -> Element<Message> {
    let content = column![
        Space::with_height(Length::Fill),

        container(
            column![
                text("Welcome to AkTags").size(28)
                    .color(Palette::ACCENT),
                Space::with_height(8.0),
                text("AI-powered tag-based file browser")
                    .size(14)
                    .color(Palette::TEXT_DIM),

                Space::with_height(32.0),

                // Ollama URL
                text("Ollama Base URL").size(12)
                    .color(Palette::TEXT_DIM),
                Space::with_height(6.0),
                text_input("https://ollama.akinus21.com", &app.first_run_url)
                    .on_input(Message::FirstRunOllamaUrlChanged)
                    .padding([10, 14])
                    .width(400.0),

                Space::with_height(16.0),

                // Model
                text("Ollama Model").size(12)
                    .color(Palette::TEXT_DIM),
                Space::with_height(4.0),
                text("Run 'ollama list' on your server to see available models.")
                    .size(11)
                    .color(Palette::TEXT_DIM),
                Space::with_height(6.0),
                text_input("gpt-oss:20b-cloud", &app.first_run_model)
                    .on_input(Message::FirstRunModelChanged)
                    .padding([10, 14])
                    .width(400.0),

                Space::with_height(16.0),

                // Watch directory
                text("Watch Directory").size(12)
                    .color(Palette::TEXT_DIM),
                Space::with_height(4.0),
                text("AkTags will monitor this folder and tag all files automatically.")
                    .size(11)
                    .color(Palette::TEXT_DIM),
                Space::with_height(6.0),
                text_input("~/Documents", &app.first_run_watch)
                    .on_input(Message::FirstRunWatchDirChanged)
                    .padding([10, 14])
                    .width(400.0),

                Space::with_height(32.0),

                button(
                    text("Get Started →").size(15)
                )
                .on_press(Message::FirstRunComplete)
                .padding([12, 32])
                .style(|_t, _s| button::Style::default()),
            ]
            .spacing(0)
            .align_x(Alignment::Start)
            .padding(40)
        )
        .width(500.0),

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
        .into()
}

// ── Settings panel ────────────────────────────────────────────────────────────

pub fn view(app: &AkTags) -> Element<Message> {
    let header = row![
        text("⚙ Settings").size(20),
        Space::with_width(Length::Fill),
        button(text("← Back").size(13))
            .on_press(Message::SwitchPanel(Panel::Browser))
            .padding([6, 14]),
    ]
    .align_y(Alignment::Center)
    .padding([16, 20]);

    let content = column![
        // ── Ollama ────────────────────────────────────────────────────────
        section_header("Ollama Connection".to_string()),

        label("Base URL".to_string()),
        text_input("https://ollama.akinus21.com", &app.settings_ollama_url)
            .on_input(Message::OllamaUrlChanged)
            .padding([8, 12])
            .width(400.0),

        Space::with_height(12.0),
        label("Model".to_string()),
        text_input("gpt-oss:20b-cloud", &app.settings_ollama_model)
            .on_input(Message::OllamaModelChanged)
            .padding([8, 12])
            .width(400.0),

        Space::with_height(24.0),

        // ── Watch Directories ─────────────────────────────────────────────
        section_header("Watch Directories".to_string()),

        {
            let dir_rows: Vec<Element<Message>> = app.config.watch_dirs.iter()
                .map(|dir| watch_dir_row(dir))
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
            button(text("+ Add").size(13))
                .on_press(Message::WatchDirAdd(app.settings_watch_dir_input.clone()))
                .padding([8, 14]),
        ]
        .align_y(Alignment::Center),

        Space::with_height(24.0),

        // ── Daemon ────────────────────────────────────────────────────────
        section_header("Daemon".to_string()),

        {
            let s = &app.daemon_stats;
            column![
                stat_row("Status", if s.running { "Running" } else { "Stopped" }),
                stat_row("Processed", &s.processed.to_string()),
                stat_row("Errors", &s.errors.to_string()),
                stat_row("Queue", &s.queue_size.to_string()),
                if let Some(f) = &s.current_file {
                    Element::from(stat_row("Current", f))
                } else {
                    Element::from(Space::with_height(0.0))
                },
            ]
            .spacing(4)
        },

        Space::with_height(24.0),

        // ── Save / Actions ────────────────────────────────────────────────
        row![
            button(text("Save Settings").size(13))
                .on_press(Message::SaveSettings)
                .padding([8, 20])
                .style(|_t, _s| button::Style::default()),
            Space::with_width(12.0),
            button(text("↺ Re-tag All Files").size(13))
                .on_press(Message::RetagAll)
                .padding([8, 20]),
        ],

        if let Some(msg) = &app.status_message {
            Element::from(text(msg).size(12).color(Palette::GREEN))
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

fn watch_dir_row(dir: &PathBuf) -> Element<Message> {
    let dir_str = dir.to_string_lossy().to_string();
    row![
        text(dir_str).size(13).width(Length::Fill),
        button(text("×").size(14))
            .on_press(Message::WatchDirRemove(dir.clone()))
            .padding([3, 8])
            .style(|_t, _s| button::Style::default()),
    ]
    .align_y(Alignment::Center)
    .spacing(8)
    .padding([6, 10])
    .into()
}

fn section_header(title: String) -> Element<'static, Message> {
    column![
        text(title).size(13).color(Palette::ACCENT2),
        Space::with_height(8.0),
    ]
    .into()
}

fn label(s: String) -> Element<'static, Message> {
    text(s).size(11)
        .color(Palette::TEXT_DIM)
        .into()
}

fn stat_row<'a>(label: &'a str, value: &'a str) -> Element<'a, Message> {
    row![
        text(label).size(12)
            .color(Palette::TEXT_DIM)
            .width(100.0),
        text(value).size(12),
    ]
    .spacing(8)
    .into()
}
