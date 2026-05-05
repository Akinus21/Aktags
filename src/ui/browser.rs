use iced::{
    widget::{
        button, column, container, horizontal_rule, image::Image as IcedImage,
        row, scrollable, text, text_input, Column, Row, Space,
    },
    Alignment, Border, Color, Element, Length,
};

use super::app::{AkTags, Message, Panel, ViewMode};
use super::theme::{self, ThemeColors, PADDING, DETAIL_W, HEADER_H, SIDEBAR_W, SPACING, CARD_W, CARD_H};
use crate::db::FileRecord;
use crate::icon::{IconCache, load_icon_for_ext, load_thumbnail_for_path, is_image_file};

// ── Theme-aware style helpers ─────────────────────────────────────────────────

fn bg_style(bg: Color) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| container::Style {
        background: Some(bg.into()),
        ..Default::default()
    }
}

/// Plain button: transparent bg, themed text color
fn btn_plain(colors: ThemeColors) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| button::Style {
        background: match status {
            button::Status::Hovered | button::Status::Pressed => {
                Some(colors.surface2().into())
            }
            _ => None,
        },
        text_color: colors.text(),
        border: Border { radius: 6.0.into(), ..Default::default() },
        ..Default::default()
    }
}

/// Accent (primary action) button
fn btn_accent(colors: ThemeColors) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| button::Style {
        background: Some(match status {
            button::Status::Hovered | button::Status::Pressed => colors.accent2().into(),
            _ => colors.accent().into(),
        }),
        text_color: Color::WHITE,
        border: Border { radius: 6.0.into(), ..Default::default() },
        ..Default::default()
    }
}

/// Ghost/dim button (for tab items, category items)
fn btn_ghost(colors: ThemeColors, active: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| button::Style {
        background: if active {
            Some(colors.surface2().into())
        } else {
            match status {
                button::Status::Hovered => Some(colors.surface().into()),
                _ => None,
            }
        },
        text_color: if active { colors.accent() } else { colors.text_dim() },
        border: Border { radius: 6.0.into(), ..Default::default() },
        ..Default::default()
    }
}

/// Tag chip button
fn btn_tag(colors: ThemeColors) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| button::Style {
        background: Some(match status {
            button::Status::Hovered => colors.surface2().into(),
            _ => colors.tag_bg().into(),
        }),
        text_color: colors.text(),
        border: Border { radius: 4.0.into(), ..Default::default() },
        ..Default::default()
    }
}

// ── Root view ─────────────────────────────────────────────────────────────────

pub fn view(app: &AkTags) -> Element<'_, Message> {
    let colors = theme::default_colors(app.theme_type);
    let header = view_header(app);
    let nav    = view_nav(app);
    let body   = row![
        view_sidebar(app),
        view_main(app),
        if app.selected_file.is_some() { view_detail(app) } else { Space::with_width(0.0).into() },
    ]
    .height(Length::Fill);

    container(
        column![header, nav, body]
            .width(Length::Fill)
            .height(Length::Fill)
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(bg_style(colors.bg()))
    .into()
}

// ── Header ────────────────────────────────────────────────────────────────────

fn view_header(app: &AkTags) -> Element<'_, Message> {
    let colors = theme::default_colors(app.theme_type);
    let ollama_ok = !app.config.ollama_base_url.is_empty();
    let status_color = if app.daemon_stats.running && ollama_ok {
        colors.green()
    } else {
        colors.red()
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

    let sync_badge = match &app.sync_status {
        super::app::SyncStatus::Idle => String::new(),
        super::app::SyncStatus::Connecting => "● connecting".to_string(),
        super::app::SyncStatus::Synced => "● synced".to_string(),
        super::app::SyncStatus::Syncing => "● syncing".to_string(),
        super::app::SyncStatus::Error(e) => format!("● error: {}", e),
    };

    let sync_color = match &app.sync_status {
        super::app::SyncStatus::Idle => colors.text_dim(),
        super::app::SyncStatus::Connecting => colors.text_dim(),
        super::app::SyncStatus::Synced => colors.green(),
        super::app::SyncStatus::Syncing => colors.accent(),
        super::app::SyncStatus::Error(_) => colors.red(),
    };

    let inner = row![
        text("AkTags").size(20).color(colors.accent()),
        Space::with_width(12.0),
        text("●").size(10).color(status_color),
        Space::with_width(8.0),
        text(status_label).size(12).color(colors.text_dim()),
        Space::with_width(8.0),
        text(queue_badge).size(11).color(colors.yellow()),
        Space::with_width(8.0),
        text(sync_badge).size(11).color(sync_color),
        Space::with_width(Length::Fill),
        button(text("Re-tag All").size(13).color(colors.text()))
            .on_press(Message::RetagAll)
            .padding([6, 14])
            .style(btn_plain(colors)),
        Space::with_width(8.0),
        button(text("Settings").size(13).color(colors.text()))
            .on_press(Message::SwitchPanel(Panel::Settings))
            .padding([6, 14])
            .style(btn_plain(colors)),
    ]
    .padding([0, 20])
    .height(HEADER_H)
    .align_y(Alignment::Center)
    .width(Length::Fill);

    container(inner)
        .width(Length::Fill)
        .style(bg_style(colors.surface()))
        .into()
}

// ── Nav tabs ──────────────────────────────────────────────────────────────────

fn view_nav(app: &AkTags) -> Element<'_, Message> {
    let colors = theme::default_colors(app.theme_type);
    let pending_count = crate::taxonomy::pending_count();
    let pending_label = if pending_count > 0 {
        format!("Pending ({})", pending_count)
    } else {
        String::from("Pending")
    };

    let inner = row![
        tab_button(String::from("Files"),        Panel::Browser,  app),
        tab_button(pending_label,                Panel::Pending,  app),
        tab_button(String::from("Tag Library"),  Panel::Taxonomy, app),
    ]
    .padding([0, 20])
    .height(42.0)
    .align_y(Alignment::End)
    .spacing(4);

    container(inner)
        .width(Length::Fill)
        .style(bg_style(colors.surface()))
        .into()
}

fn tab_button(label: String, panel: Panel, app: &AkTags) -> Element<'_, Message> {
    let active = app.panel == panel;
    button(
        text(label).size(13).color(if active {
            theme::default_colors(app.theme_type).accent()
        } else {
            theme::default_colors(app.theme_type).text_dim()
        })
    )
    .on_press(Message::SwitchPanel(panel))
    .padding([8, 18])
    .style(btn_ghost(theme::default_colors(app.theme_type), active))
    .into()
}

// ── Sidebar ───────────────────────────────────────────────────────────────────

fn view_sidebar(app: &AkTags) -> Element<'_, Message> {
    let colors = theme::default_colors(app.theme_type);
    let stats = app.stats.as_ref();
    let total = stats.map(|s| s.total).unwrap_or(0);

    let mut cat_items: Vec<Element<'_, Message>> = vec![
        category_item(String::from("All Files"), total, None, app.active_category.clone(), app.theme_type),
    ];
    if let Some(s) = stats {
        for (cat, count) in &s.by_category {
            cat_items.push(category_item(
                format!("{} {cat}", category_icon(cat)),
                *count,
                Some(cat.clone()),
                app.active_category.clone(),
                app.theme_type,
            ));
        }
    }

    // Build tag chips — sorted alphabetically by tag name, free-flowing wrap
    let mut tag_pairs: Vec<(String, i64)> = app.all_tags.iter()
        .take(100)
        .map(|(tag, count)| (tag.clone(), *count))
        .collect();
    tag_pairs.sort_by(|a, b| a.0.cmp(&b.0));

    let tag_chips: Vec<Element<'_, Message>> = tag_pairs
        .iter()
        .map(|(tag, count)| {
            let label = format!("{} {}", tag, count);
            let colors2 = colors;
            button(text(label).size(11).color(colors2.text()))
                .on_press(Message::TagToggled(tag.clone()))
                .padding([3, 8])
                .style(btn_tag(colors2))
                .into()
        })
        .collect();

    // Wrap tags into rows that fit sidebar width
    let wrapped_tags = wrap_tag_rows(tag_chips, 4.0, SIDEBAR_W - 28.0);

    let sidebar_content = column![
        text("Categories").size(11).color(colors.text_dim()),
        Space::with_height(8.0),
        Column::with_children(cat_items).spacing(2),
        horizontal_rule(1),
        Space::with_height(12.0),
        text("Tags").size(11).color(colors.text_dim()),
        Space::with_height(8.0),
        scrollable(wrapped_tags).height(Length::Fill),
    ]
    .spacing(4)
    .padding(14)
    .height(Length::Fill);

    container(sidebar_content)
        .width(SIDEBAR_W)
        .height(Length::Fill)
        .style(bg_style(colors.surface()))
        .into()
}

fn category_item(
    label: String,
    count: i64,
    cat: Option<String>,
    active: Option<String>,
    theme_type: theme::ThemeType,
) -> Element<'static, Message> {
    let colors = theme::default_colors(theme_type);
    let is_active = active == cat;
    button(
        row![
            text(label).size(13).color(if is_active {
                colors.accent()
            } else {
                colors.text()
            }),
            Space::with_width(Length::Fill),
            text(count.to_string()).size(11).color(colors.text_dim()),
        ]
        .align_y(Alignment::Center)
    )
    .on_press(Message::CategorySelected(cat))
    .padding([5, 10])
    .width(Length::Fill)
    .style(btn_ghost(colors, is_active))
    .into()
}

fn category_icon(cat: &str) -> &'static str {
    match cat {
        "documents" => "[DOC]",
        "images"    => "[IMG]",
        "code"      => "[COD]",
        "audio"     => "[AUD]",
        "video"     => "[VID]",
        _           => "[DIR]",
    }
}

// ── Main area ─────────────────────────────────────────────────────────────────

fn view_main(app: &AkTags) -> Element<'_, Message> {
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

fn view_toolbar(app: &AkTags) -> Element<'_, Message> {
    let colors = theme::default_colors(app.theme_type);
    let count_label = format!("{} files", app.files.len());
    let view_icon = match app.view_mode {
        ViewMode::Grid => "Grid",
        ViewMode::List => "List",
    };

    let inner = row![
        text_input("Search files...", &app.search_query)
            .on_input(Message::SearchChanged)
            .on_submit(Message::SearchSubmit)
            .padding([8, 14])
            .width(Length::Fill),
        Space::with_width(10.0),
        text(count_label).size(12).color(colors.text_dim()),
        Space::with_width(10.0),
        button(text(view_icon).size(13).color(colors.text()))
            .on_press(Message::ViewToggled)
            .padding([6, 10])
            .style(btn_plain(colors)),
    ]
    .padding([12, 16])
    .align_y(Alignment::Center);

    container(inner)
        .width(Length::Fill)
        .style(bg_style(colors.bg()))
        .into()
}

fn view_active_filters(app: &AkTags) -> Element<'_, Message> {
    let colors = theme::default_colors(app.theme_type);
    if app.active_tags.is_empty() && app.active_category.is_none() {
        return Space::with_height(0.0).into();
    }

    let mut chips: Vec<Element<'_, Message>> = vec![];

    if let Some(cat) = &app.active_category {
        chips.push(filter_chip(format!("{cat}"), Message::CategorySelected(None), colors));
    }
    for tag in &app.active_tags {
        chips.push(filter_chip(tag.clone(), Message::TagToggled(tag.clone()), colors));
    }
    chips.push(
        button(text("Clear all").size(12).color(colors.text_dim()))
            .on_press(Message::ClearFilters)
            .padding([3, 10])
            .style(btn_plain(colors))
            .into()
    );

    row(chips).spacing(6).padding([6, 16]).into()
}

fn filter_chip(label: String, on_remove: Message, colors: ThemeColors) -> Element<'static, Message> {
    button(
        row![
            text(label).size(12).color(colors.text()),
            Space::with_width(4.0),
            text("×").size(14).color(colors.text_dim()),
        ]
        .align_y(Alignment::Center)
    )
    .on_press(on_remove)
    .padding([3, 10])
    .style(btn_tag(colors))
    .into()
}

// ── Grid view ─────────────────────────────────────────────────────────────────

fn view_grid(app: &AkTags) -> Element<'_, Message> {
    if app.files.is_empty() {
        return empty_state("No files found", "Try adjusting your search or filters.", app.theme_type);
    }

    let selected_id = app.selected_file.as_ref().map(|s| s.id);
    let files = &app.files;
    let icon_cache = &app.icon_cache;
    let mut rows: Vec<Element<'_, Message>> = Vec::new();
    for chunk in files.chunks(4) {
        let row_items: Vec<Element<'_, Message>> = chunk.iter()
            .map(|f| file_card(f, app.theme_type, selected_id == Some(f.id), icon_cache))
            .collect();
        rows.push(
            Row::with_children(row_items)
                .spacing(SPACING)
                .width(Length::Fill)
                .into()
        );
    }

    scrollable(
        Column::with_children(rows)
            .spacing(SPACING)
            .padding(PADDING)
    )
    .width(Length::Fill)
    .into()
}

fn file_card(
    file: &FileRecord,
    theme_type: theme::ThemeType,
    selected: bool,
    icon_cache: &IconCache,
) -> Element<'_, Message> {
    let colors = theme::default_colors(theme_type);
    let icon_elem = icon_view(icon_cache, &file.extension, &file.path, 48);
    let name = truncate(&file.filename, 22);
    let summary = file.summary.as_deref().unwrap_or("").to_string();
    let summary_short = truncate(&summary, 50);

    let tags: Vec<Element<'_, Message>> = file.tags.iter().take(3)
        .map(|t| {
            button(text(t).size(10).color(colors.text()))
                .on_press(Message::TagToggled(t.clone()))
                .padding([2, 6])
                .style(btn_tag(colors))
                .into()
        })
        .collect();

    let card_content = column![
        icon_elem,
        Space::with_height(8.0),
        text(name).size(12).color(colors.text()),
        text(summary_short).size(11).color(colors.text_dim()),
        Space::with_height(4.0),
        Row::with_children(tags).spacing(3),
    ]
    .spacing(4)
    .padding(12)
    .width(CARD_W)
    .height(CARD_H);

    let bg = if selected { colors.surface2() } else { colors.surface() };
    let border_color = if selected { colors.accent() } else { colors.border() };

    button(card_content)
        .on_press(Message::FileSelected(file.id))
        .style(move |_, status| button::Style {
            background: Some(match status {
                button::Status::Hovered => colors.surface2().into(),
                _ => bg.into(),
            }),
            text_color: colors.text(),
            border: Border {
                color: border_color,
                width: if selected { 1.5 } else { 1.0 },
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
}

// ── List view ─────────────────────────────────────────────────────────────────

fn view_list(app: &AkTags) -> Element<'_, Message> {
    if app.files.is_empty() {
        return empty_state("No files found", "Try adjusting your search or filters.", app.theme_type);
    }

    let colors = theme::default_colors(app.theme_type);
    let selected_id = app.selected_file.as_ref().map(|s| s.id);

    // Sort indicator arrow (unused - each sort_header computes its own arrow)
    let _arrow = |field: super::app::SortField| {
        if app.sort_field == field {
            match app.sort_direction {
                super::app::SortDirection::Ascending => " ▲",
                super::app::SortDirection::Descending => " ▼",
            }
        } else {
            ""
        }.to_string()
    };

    // Column header row
    let header = row![
        text("").size(12).width(30.0), // icon column spacer
        sort_header("Name", super::app::SortField::Name, colors, &app),
        Space::with_width(Length::Fill),
        text("Tags").size(11).color(colors.text_dim()),
        Space::with_width(12.0),
        sort_header("Category", super::app::SortField::Category, colors, &app),
        Space::with_width(12.0),
        sort_header("Size", super::app::SortField::Size, colors, &app),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .padding([6, 12])
    .width(Length::Fill);

    let header_container = container(header)
        .width(Length::Fill)
        .style(bg_style(colors.surface2()));

    let icon_cache = &app.icon_cache;
    let rows: Vec<Element<'_, Message>> = app.files.iter()
        .map(|f| file_row(f, app.theme_type, selected_id == Some(f.id), icon_cache))
        .collect();

    column![
        header_container,
        Column::with_children(rows)
            .spacing(4)
            .padding([0u16, 16])
            .width(Length::Fill),
    ]
    .width(Length::Fill)
    .into()
}

fn sort_header<'a>(
    label: &'a str,
    field: super::app::SortField,
    colors: ThemeColors,
    app: &'a AkTags,
) -> Element<'a, Message> {
    let is_active = app.sort_field == field;
    let arrow = if is_active {
        match app.sort_direction {
            super::app::SortDirection::Ascending => " ▲",
            super::app::SortDirection::Descending => " ▼",
        }
    } else {
        ""
    };
    let full_label = format!("{}{}", label, arrow);

    button(text(full_label).size(11).color(if is_active {
        colors.accent()
    } else {
        colors.text_dim()
    }))
        .on_press(Message::SortChanged(field))
        .padding([2, 6])
        .style(move |_, status| button::Style {
            background: match status {
                button::Status::Hovered => Some(colors.surface().into()),
                _ => None,
            },
            text_color: if is_active { colors.accent() } else { colors.text_dim() },
            ..Default::default()
        })
        .into()
}

fn file_row(
    file: &FileRecord,
    theme_type: theme::ThemeType,
    selected: bool,
    icon_cache: &IconCache,
) -> Element<'_, Message> {
    let colors = theme::default_colors(theme_type);
    let icon_elem = icon_view(icon_cache, &file.extension, &file.path, 18);
    let tags: Vec<Element<'_, Message>> = file.tags.iter().take(4)
        .map(|t| {
            button(text(t).size(11).color(colors.text()))
                .on_press(Message::TagToggled(t.clone()))
                .padding([2, 6])
                .style(btn_tag(colors))
                .into()
        })
        .collect();

    let row_content = row![
        icon_elem.width(Length::Units(30)),
        column![
            text(&file.filename).size(13).color(colors.text()),
            text(file.summary.as_deref().unwrap_or("")).size(11)
                .color(colors.text_dim()),
        ]
        .spacing(2)
        .width(Length::Fill),
        Row::with_children(tags).spacing(3),
        text(&file.category).size(11)
            .color(colors.text_dim())
            .width(80.0),
        text(fmt_size(file.size_bytes)).size(11)
            .color(colors.text_dim())
            .width(70.0),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .padding([8, 12]);

    let bg = if selected { colors.surface2() } else { colors.bg() };
    button(row_content)
        .on_press(Message::FileSelected(file.id))
        .width(Length::Fill)
        .style(move |_, status| button::Style {
            background: Some(match status {
                button::Status::Hovered => colors.surface2().into(),
                _ => bg.into(),
            }),
            text_color: colors.text(),
            border: Border {
                color: if selected { colors.accent() } else { Color::TRANSPARENT },
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        })
        .into()
}

// ── Detail panel ──────────────────────────────────────────────────────────────

fn view_detail(app: &AkTags) -> Element<'_, Message> {
    let Some(file) = &app.selected_file else {
        return Space::with_width(0.0).into();
    };

    let colors = theme::default_colors(app.theme_type);

    let tags: Vec<Element<'_, Message>> = file.tags.iter()
        .map(|t| {
            row![
                button(text(t).size(12).color(colors.text()))
                    .on_press(Message::TagToggled(t.clone()))
                    .padding([3, 8])
                    .style(btn_tag(colors)),
                button(text("×").size(12).color(colors.red()))
                    .on_press(Message::RemoveTagFromFile(file.id, t.clone()))
                    .padding([3, 6])
                    .style(btn_plain(colors)),
            ]
            .spacing(2)
            .into()
        })
        .collect();

    let content = column![
        // Close + open buttons
        row![
            button(text("×").size(16).color(colors.text_dim()))
                .on_press(Message::FileDeselected)
                .style(btn_plain(colors)),
            Space::with_width(Length::Fill),
            button(text("Open").size(13).color(Color::WHITE))
                .on_press(Message::FileOpened(file.id))
                .padding([6, 12])
                .style(btn_accent(colors)),
        ]
        .align_y(Alignment::Center),
        Space::with_height(12.0),

        // File icon + name
        icon_view(&app.icon_cache, &file.extension, &file.path, 40),
        text(&file.filename).size(14).color(colors.text()),
        text(&file.category).size(11).color(colors.text_dim()),
        Space::with_height(4.0),
        text(fmt_size(file.size_bytes)).size(11).color(colors.text_dim()),
        Space::with_height(12.0),

        // Summary
        text("Summary").size(11).color(colors.text_dim()),
        text(file.summary.as_deref().unwrap_or("No summary yet"))
            .size(12).color(colors.text()),
        Space::with_height(12.0),

        // Tags
        text("Tags").size(11).color(colors.text_dim()),
        Row::with_children(tags).spacing(4),
        Space::with_height(8.0),

        // Add tag input
        row![
            text_input("Add tag...", &app.tag_input)
                .on_input(Message::TagInputChanged)
                .on_submit(Message::TagInputSubmit)
                .padding([6, 10])
                .width(Length::Fill),
            button(text("+").size(14).color(Color::WHITE))
                .on_press(Message::TagInputSubmit)
                .padding([6, 10])
                .style(btn_accent(colors)),
        ]
        .spacing(6),

        Space::with_height(12.0),

        // Path
        text("Path").size(11).color(colors.text_dim()),
        text(&file.path).size(11).color(colors.text_dim()),
    ]
    .spacing(4)
    .padding(16);

    container(scrollable(content))
        .width(DETAIL_W)
        .height(Length::Fill)
        .style(bg_style(colors.surface()))
        .into()
}

// ── Empty state ───────────────────────────────────────────────────────────────

fn empty_state<'a>(title: &'a str, subtitle: &'a str, theme_type: theme::ThemeType) -> Element<'a, Message> {
    let colors = theme::default_colors(theme_type);
    container(
        column![
            text("?").size(48).color(colors.text_dim()),
            Space::with_height(12.0),
            text(title).size(16).color(colors.text()),
            text(subtitle).size(13).color(colors.text_dim()),
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
        ".json" | ".yaml" | ".yml" => "[CFG]",
        ".jpg" | ".jpeg" | ".png" | ".gif" | ".webp" => "[IMG]",
        ".mp3" | ".wav" | ".flac" => "[AUD]",
        ".mp4" | ".mkv" | ".avi" => "[VID]",
        ".zip" | ".tar" | ".gz" | ".rar" => "[ARC]",
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

// ── Tag wrapping helper ───────────────────────────────────────────────────────

fn wrap_tag_rows(
    items: Vec<Element<'_, Message>>,
    spacing: f32,
    available_width: f32,
) -> Element<'_, Message> {
    if items.is_empty() {
        return Space::with_height(0.0).into();
    }
    let mut rows: Vec<Element<'_, Message>> = Vec::new();
    let mut current_row: Vec<Element<'_, Message>> = Vec::new();
    let mut current_row_width: f32 = 0.0;

    let chip_spacing = spacing;
    let chip_min_width = 90.0;

    for item in items {
        if !current_row.is_empty() && current_row_width + chip_min_width > available_width {
            rows.push(
                Row::with_children(std::mem::take(&mut current_row))
                    .spacing(chip_spacing)
                    .width(Length::Fill)
                    .into()
            );
            current_row_width = 0.0;
        }

        current_row.push(item);
        current_row_width += chip_min_width + chip_spacing;
    }

    if !current_row.is_empty() {
        rows.push(
            Row::with_children(current_row)
                .spacing(chip_spacing)
                .width(Length::Fill)
                .into()
        );
    }

    Column::with_children(rows)
        .spacing(spacing)
        .width(Length::Fill)
        .into()
}

fn icon_view(icon_cache: &IconCache, ext: &str, path: &str, size: u32) -> Element<'static, Message> {
    let ext_lower = ext.to_lowercase();

    if is_image_file(ext) {
        if let Some(cached) = icon_cache.get_path(path) {
            return IcedImage::new(iced::widget::image::Handle::from_pixels(
                cached.width, cached.height, (*cached.rgba).clone(),
            )).width(Length::Units(size)).into();
        }
        if let Some(icon) = load_thumbnail_for_path(path, size) {
            let elem = IcedImage::new(iced::widget::image::Handle::from_pixels(
                icon.width, icon.height, (*icon.rgba).clone(),
            )).width(Length::Units(size)).into();
            return elem;
        }
    }

    if let Some(cached) = icon_cache.get_ext(ext) {
        return IcedImage::new(iced::widget::image::Handle::from_pixels(
            cached.width, cached.height, (*cached.rgba).clone(),
        )).width(Length::Units(size)).into();
    }

    if let Some(icon) = load_icon_for_ext(ext) {
        let elem = IcedImage::new(iced::widget::image::Handle::from_pixels(
            icon.width, icon.height, (*icon.rgba).clone(),
        )).width(Length::Units(size)).into();
        return elem;
    }

    let fallback = file_type_icon(ext);
    text(fallback).size(size).into()
}
