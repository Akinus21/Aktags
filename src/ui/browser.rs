use iced::{
    widget::{
        button, column, container, horizontal_rule, row, scrollable,
        text, text_input, Column, Row, Space,
    },
    Alignment, Element, Length,
};

use super::{app::{AkTags, Message, Panel, ViewMode}, theme::*};
use crate::db::FileRecord;

pub fn view(app: &AkTags) -> Element<Message> {
    let header = view_header(app);
    let nav    = view_nav(app);
    let body   = row![
        view_sidebar(app),
        view_main(app),
        if app.selected_file.is_some() { view_detail(app) } else { Space::with_width(0.0).into() },
    ]
    .height(Length::Fill);

    column![header, nav, body]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

// ── Header ────────────────────────────────────────────────────────────────────

fn view_header(app: &AkTags) -> Element<Message> {
    let ollama_ok = !app.config.ollama_base_url.is_empty();
    let status_color = if app.daemon_stats.running && ollama_ok {
        Palette::GREEN
    } else {
        Palette::RED
    };

    let status_label = if app.daemon_stats.running {
        format!(
            "{} @ {} · {} files",
            app.config.ollama_model,
            app.config.ollama_base_url,
            app.stats.as_ref().map(|s| s.total).unwrap_or(0)
        )
    } else {
        "Daemon not running".to_string()
    };

    let queue_badge = if app.daemon_stats.queue_size > 0 {
        format!("{} queued", app.daemon_stats.queue_size)
    } else {
        String::new()
    };

    let status_label_owned = status_label;
    let queue_badge_owned = queue_badge;

    row![
        text("AkTags").size(20).color(Palette::ACCENT),
        Space::with_width(12.0),
        text("[*]").size(12).color(status_color),
        Space::with_width(8.0),
        text(status_label_owned).size(12).color(Palette::TEXT_DIM),
        Space::with_width(8.0),
        text(queue_badge_owned).size(11).color(Palette::YELLOW),
        Space::with_width(Length::Fill),
        nav_button("Re-tag All", Message::RetagAll),
        Space::with_width(8.0),
        nav_button("Settings", Message::SwitchPanel(Panel::Settings)),
    ]
    .padding([0, 20])
    .height(HEADER_H)
    .align_y(Alignment::Center)
    .width(Length::Fill)
    .into()
}

fn nav_button(label: &str, msg: Message) -> Element<Message> {
    button(text(label).size(13))
        .on_press(msg)
        .padding([6, 14])
        .into()
}

// ── Nav tabs ──────────────────────────────────────────────────────────────────

fn view_nav(app: &AkTags) -> Element<Message> {
    let pending_count = crate::taxonomy::pending_count();
    let pending_label = if pending_count > 0 {
        format!("Pending ({})", pending_count)
    } else {
        String::from("Pending")
    };

    row![
        tab_button(String::from("Files"),       Panel::Browser,  app),
        tab_button(pending_label.clone(), Panel::Pending,  app),
        tab_button(String::from("Tag Library"),  Panel::Taxonomy, app),
    ]
    .padding([0, 20])
    .height(42.0)
    .align_y(Alignment::End)
    .spacing(4)
    .into()
}

fn tab_button(label: String, panel: Panel, app: &AkTags) -> Element<Message> {
    let active = app.panel == panel;
    button(
        text(label).size(13).color(if active {
            Palette::ACCENT
        } else {
            Palette::TEXT_DIM
        })
    )
    .on_press(Message::SwitchPanel(panel))
    .padding([8, 18])
    .style(|_t, _s| button::Style::default())
    .into()
}

// ── Sidebar ───────────────────────────────────────────────────────────────────

fn view_sidebar(app: &AkTags) -> Element<Message> {
    let stats = app.stats.as_ref();
    let total = stats.map(|s| s.total).unwrap_or(0);

    // Categories
    let mut cat_items: Vec<Element<Message>> = vec![
        category_item(String::from("All Files"), total, None, app.active_category.clone()),
    ];
    if let Some(s) = stats {
        for (cat, count) in &s.by_category {
            cat_items.push(category_item(
                format!("{} {cat}", category_icon(cat)),
                *count,
                Some(cat.clone()),
                app.active_category.clone(),
            ));
        }
    }

    // Tag cloud
    let tag_items: Vec<Element<Message>> = app.all_tags.iter()
        .take(100)
        .map(|(tag, count)| {
            let label = format!("{tag} {count}");
            button(text(label).size(12))
                .on_press(Message::TagToggled(tag.clone()))
                .padding([3, 10])
                .style(|_t, _s| button::Style::default())
                .into()
        })
        .collect();

    let sidebar_content = column![
        // Categories section
        text("Categories").size(11).color(Palette::TEXT_DIM),
        Space::with_height(8.0),
        Column::with_children(cat_items).spacing(2),
        horizontal_rule(1),
        Space::with_height(8.0),
        text("Tags").size(11).color(Palette::TEXT_DIM),
        Space::with_height(8.0),
        scrollable(
            Row::with_children(tag_items)
                .spacing(4)
                
        ).height(Length::Fill),
    ]
    .spacing(4)
    .padding(14)
    .height(Length::Fill);

    container(sidebar_content)
        .width(SIDEBAR_W)
        .height(Length::Fill)
        .into()
}

fn category_item(
    label: String,
    count: i64,
    cat: Option<String>,
    active: Option<String>,
) -> Element<'static, Message> {
    let is_active = active == cat;
    button(
        row![
            text(label).size(13).color(if is_active {
                Palette::ACCENT
            } else {
                Palette::TEXT
            }),
            Space::with_width(Length::Fill),
            text(count.to_string()).size(11)
                .color(Palette::TEXT_DIM),
        ]
        .align_y(Alignment::Center)
    )
    .on_press(Message::CategorySelected(cat))
    .padding([5, 10])
    .width(Length::Fill)
    .style(|_t, _s| button::Style::default())
    .into()
}

fn category_icon(cat: &str) -> &'static str {
    match cat {
        "documents" => "[doc]",
        "images"    => "[img]",
        "code"      => "[code]",
        "audio"     => "[audio]",
        "video"     => "[video]",
        _           => "[file]",
    }
}

// ── Main area ─────────────────────────────────────────────────────────────────

fn view_main(app: &AkTags) -> Element<Message> {
    let toolbar = view_toolbar(app);
    let active_filters = view_active_filters(app);
    let file_area = match app.view_mode {
        ViewMode::Grid => view_grid(app),
        ViewMode::List => view_list(app),
    };

    column![
        toolbar,
        active_filters,
        scrollable(file_area).height(Length::Fill),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn view_toolbar(app: &AkTags) -> Element<Message> {
    let count_label = format!("{} files", app.files.len());
    let view_icon = match app.view_mode {
        ViewMode::Grid => "Grid",
        ViewMode::List => "List",
    };

    let count_label_owned = count_label;

    row![
        text_input("Search files...", &app.search_query)
            .on_input(Message::SearchChanged)
            .on_submit(Message::SearchSubmit)
            .padding([8, 14])
            .width(Length::Fill),
        Space::with_width(10.0),
        text(count_label_owned).size(12).color(Palette::TEXT_DIM),
        Space::with_width(10.0),
        button(text(view_icon).size(13))
            .on_press(Message::ViewToggled)
            .padding([6, 10]),
    ]
    .padding([12, 16])
    .align_y(Alignment::Center)
    .into()
}

fn view_active_filters(app: &AkTags) -> Element<Message> {
    if app.active_tags.is_empty() && app.active_category.is_none() {
        return Space::with_height(0.0).into();
    }

    let mut chips: Vec<Element<Message>> = vec![];

    if let Some(cat) = &app.active_category {
        chips.push(filter_chip(format!("{cat}"), Message::CategorySelected(None)));
    }
    for tag in &app.active_tags {
        chips.push(filter_chip(tag.clone(), Message::TagToggled(tag.clone())));
    }
    chips.push(
        button(text("Clear all").size(12))
            .on_press(Message::ClearFilters)
            .padding([3, 10])
            .into()
    );

    row(chips).spacing(6).padding([6, 16]).into()
}

fn filter_chip(label: String, on_remove: Message) -> Element<'static, Message> {
    button(
        row![
            text(label).size(12),
            Space::with_width(4.0),
            text("×").size(14),
        ]
        .align_y(Alignment::Center)
    )
    .on_press(on_remove)
    .padding([3, 10])
    .style(|_t, _s| button::Style::default())
    .into()
}

// ── Grid view ─────────────────────────────────────────────────────────────────

fn view_grid(app: &AkTags) -> Element<Message> {
    if app.files.is_empty() {
        return empty_state("No files found", "Try adjusting your search or filters.");
    }

    let cards: Vec<Element<Message>> = app.files.iter()
        .map(|f| file_card(f, app.selected_file.as_ref().map(|s| s.id) == Some(f.id)))
        .collect();

    scrollable(
        Column::with_children(cards)
            .spacing(SPACING)
            .padding(PADDING)
    )
    .width(Length::Fill)
    .into()
}

fn file_card(file: &FileRecord, _selected: bool) -> Element<Message> {
    let icon = file_type_icon(&file.extension);
    let name = truncate(&file.filename, 22);
    let summary = file.summary.as_deref().unwrap_or("").to_string();
    let summary_short = truncate(&summary, 50);

    let tags: Vec<Element<Message>> = file.tags.iter().take(3)
        .map(|t| {
            button(text(t).size(10))
                .on_press(Message::TagToggled(t.clone()))
                .padding([2, 6])
                .style(|_t, _s| button::Style::default())
                .into()
        })
        .collect();

    let card_content = column![
        text(icon).size(32),
        Space::with_height(8.0),
        text(name).size(12).color(Palette::TEXT),
        text(summary_short).size(11).color(Palette::TEXT_DIM),
        Space::with_height(4.0),
        Row::with_children(tags).spacing(3),
    ]
    .spacing(4)
    .padding(12)
    .width(CARD_W)
    .height(CARD_H);

    let btn = button(card_content)
        .on_press(Message::FileSelected(file.id))
        .style(|_t, _s| button::Style::default());

    btn.into()
}

// ── List view ─────────────────────────────────────────────────────────────────

fn view_list(app: &AkTags) -> Element<'_, Message> {
    if app.files.is_empty() {
        return empty_state("No files found", "Try adjusting your search or filters.");
    }

    let rows: Vec<Element<Message>> = app.files.iter()
        .map(|f| file_row(f, app.selected_file.as_ref().map(|s| s.id) == Some(f.id)))
        .collect();

    Column::with_children(rows)
        .spacing(4)
        .padding(PADDING)
        .width(Length::Fill)
        .into()
}

fn file_row(file: &FileRecord, _selected: bool) -> Element<'_, Message> {
    let icon = file_type_icon(&file.extension);
    let tags: Vec<Element<Message>> = file.tags.iter().take(4)
        .map(|t| {
            button(text(t).size(11))
                .on_press(Message::TagToggled(t.clone()))
                .padding([2, 6])
                .style(|_t, _s| button::Style::default())
                .into()
        })
        .collect();

    let row_content = row![
        text(icon).size(18).width(30.0),
        column![
            text(&file.filename).size(13),
            text(file.summary.as_deref().unwrap_or("")).size(11)
                .color(Palette::TEXT_DIM),
        ]
        .spacing(2)
        .width(Length::Fill),
        Row::with_children(tags).spacing(3),
        text(&file.category).size(11)
            .color(Palette::TEXT_DIM)
            .width(80.0),
        text(fmt_size(file.size_bytes)).size(11)
            .color(Palette::TEXT_DIM)
            .width(70.0),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .padding([8, 12]);

    button(row_content)
        .on_press(Message::FileSelected(file.id))
        .width(Length::Fill)
        .style(|_t, _s| button::Style::default())
        .into()
}

// ── Detail panel ──────────────────────────────────────────────────────────────

fn view_detail(app: &AkTags) -> Element<'_, Message> {
    let Some(file) = &app.selected_file else {
        return Space::with_width(0.0).into();
    };

    let tags: Vec<Element<Message>> = file.tags.iter()
        .map(|t| {
            row![
                button(text(t).size(12))
                    .on_press(Message::TagToggled(t.clone()))
                    .padding([3, 8])
                    .style(|_t, _s| button::Style::default()),
                button(text("×").size(12))
                    .on_press(Message::RemoveTagFromFile(file.id, t.clone()))
                    .padding([3, 6])
                    .style(|_t, _s| button::Style::default()),
            ]
            .spacing(2)
            .into()
        })
        .collect();

    let content = column![
        // Close + open buttons
        row![
            button(text("×").size(16))
                .on_press(Message::FileDeselected)
                .style(|_t, _s| button::Style::default()),
            Space::with_width(Length::Fill),
            button(text("Open").size(13))
                .on_press(Message::FileOpened(file.id))
                .padding([6, 12]),
        ]
        .align_y(Alignment::Center),
        Space::with_height(12.0),

        // File icon + name
        text(file_type_icon(&file.extension)).size(40),
        text(&file.filename).size(14),
        text(&file.category).size(11)
            .color(Palette::TEXT_DIM),
        Space::with_height(4.0),
        text(fmt_size(file.size_bytes)).size(11)
            .color(Palette::TEXT_DIM),
        Space::with_height(12.0),

        // Summary
        text("Summary").size(11)
            .color(Palette::TEXT_DIM),
        text(file.summary.as_deref().unwrap_or("No summary yet"))
            .size(12),
        Space::with_height(12.0),

        // Tags
        text("Tags").size(11)
            .color(Palette::TEXT_DIM),
        Row::with_children(tags).spacing(4),
        Space::with_height(8.0),

        // Add tag input
        row![
            text_input("Add tag...", &app.tag_input)
                .on_input(Message::TagInputChanged)
                .on_submit(Message::TagInputSubmit)
                .padding([6, 10])
                .width(Length::Fill),
            button(text("+").size(14))
                .on_press(Message::TagInputSubmit)
                .padding([6, 10]),
        ]
        .spacing(6),

        Space::with_height(12.0),

        // Path
        text("Path").size(11)
            .color(Palette::TEXT_DIM),
        text(&file.path).size(11)
            .color(Palette::TEXT_DIM),
    ]
    .spacing(4)
    .padding(16);

    container(scrollable(content))
        .width(DETAIL_W)
        .height(Length::Fill)
        .into()
}

// ── Empty state ───────────────────────────────────────────────────────────────

fn empty_state<'a>(title: &'a str, subtitle: &'a str) -> Element<'a, Message> {
    container(
        column![
            text("🔍").size(48),
            Space::with_height(12.0),
            text(title).size(16),
            text(subtitle).size(13)
                .color(Palette::TEXT_DIM),
        ]
        .spacing(8)
        .align_x(Alignment::Center)
        .padding(60),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn file_type_icon(ext: &str) -> &'static str {
    match ext {
        ".pdf"  => "[PDF]", ".doc" | ".docx" => "[DOC]",
        ".txt" | ".md" => "[TXT]", ".xls" | ".xlsx" => "[XLS]",
        ".ppt" | ".pptx" => "[PPT]", ".py" => "[PY]",
        ".js" | ".ts" => "[JS]", ".sh" | ".bash" => "[SH]",
        ".json" | ".yaml" | ".yml" => "[JSON]",
        ".jpg" | ".jpeg" | ".png" | ".gif" | ".webp" => "[IMG]",
        ".mp3" | ".wav" | ".flac" => "[AUDIO]",
        ".mp4" | ".mkv" | ".avi" => "[VIDEO]",
        _ => "[FILE]",
    }
}

fn fmt_size(bytes: i64) -> String {
    if bytes < 1024 { return format!("{bytes} B"); }
    if bytes < 1_048_576 { return format!("{:.1} KB", bytes as f64 / 1024.0); }
    format!("{:.1} MB", bytes as f64 / 1_048_576.0)
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max).collect::<String>())
    }
}
