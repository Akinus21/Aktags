use iced::{
    widget::{button, column, container, row, scrollable, text, text_input, Column, Row, Space},
    Alignment, Element, Length,
};

use super::app::{AkTags, Message, Panel};
use super::theme;

const CATEGORIES: &[&str] = &["work", "education", "technical", "personal", "military", "misc"];

// ── Reusable button helpers ───────────────────────────────────────────────────

fn btn_plain<'a>(label: &'a str) -> button::Button<'a, Message> {
    button(text(label).size(12)).padding([5, 10])
}

fn btn_accent<'a>(label: &'a str) -> button::Button<'a, Message> {
    button(text(label).size(12)).padding([5, 10])
}

fn btn_tag<'a>(label: &'a str) -> button::Button<'a, Message> {
    button(text(label).size(11)).padding([3, 8])
}

// ── Pending tags panel ────────────────────────────────────────────────────────

pub fn view_pending(app: &AkTags) -> Element<'_, Message> {
    let colors = theme::default_colors(app.theme_type);

    let header = row![
        btn_plain("<- Back")
            .on_press(Message::SwitchPanel(Panel::Browser)),
        Space::with_width(16.0),
        column![
            text("Pending Tag Approvals").size(16),
            text("Tags proposed by AI not yet in your library. Approve, reject, or merge.")
                .size(12)
                .color(colors.text_dim()),
        ],
        Space::with_width(Length::Fill),
        btn_accent("Approve All").on_press(Message::ApproveAll),
        Space::with_width(8.0),
        btn_plain("Reject All").on_press(Message::RejectAll),
    ]
    .align_y(Alignment::Center)
    .padding([16, 20]);

    let items: Vec<Element<'_, Message>> = if app.pending.is_empty() {
        vec![
            container(
                column![
                    text("All clear").size(48),
                    Space::with_height(12.0),
                    text("No pending tags").size(16),
                    text("All AI-proposed tags have been reviewed.")
                        .size(13)
                        .color(colors.text_dim()),
                ]
                .spacing(8)
                .align_x(Alignment::Center)
                .padding(60),
            )
            .center_x(Length::Fill)
            .width(Length::Fill)
            .into(),
        ]
    } else {
        app.pending.iter().map(|(tag, meta)| pending_card(app, tag, meta)).collect()
    };

    column![
        header,
        scrollable(
            Column::with_children(items)
                .spacing(10)
                .padding([0, 20])
                .width(Length::Fill)
        )
        .height(Length::Fill),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn pending_card<'a>(
    app: &'a AkTags,
    tag: &'a str,
    meta: &'a crate::taxonomy::PendingTag,
) -> Element<'a, Message> {
    let colors = theme::default_colors(app.theme_type);
    let merge_input = app.pending_merge_inputs
        .get(tag)
        .map(|s| s.as_str())
        .unwrap_or("");

    let files_preview = meta.example_files.iter().take(3)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");

    // Category approval buttons — compact row
    let cat_buttons: Vec<Element<'_, Message>> = CATEGORIES.iter()
        .map(|&cat| {
            btn_tag(cat)
                .on_press(Message::PendingApprove(tag.to_string(), cat.to_string()))
                .into()
        })
        .collect();

    container(
        column![
            row![
                text(tag).size(16).color(colors.orange()),
                Space::with_width(12.0),
                text(format!("{} file{}", meta.file_count, if meta.file_count != 1 { "s" } else { "" }))
                    .size(12)
                    .color(colors.text_dim()),
                Space::with_width(Length::Fill),
                button(text("Reject").size(12))
                    .on_press(Message::PendingReject(tag.to_string()))
                    .padding([4, 10]),
            ]
            .align_y(Alignment::Center),

            text(files_preview).size(11)
                .color(colors.text_dim()),

            Space::with_height(8.0),

            text("Approve as:").size(11)
                .color(colors.text_dim()),
            Row::with_children(cat_buttons).spacing(6.0),

            Space::with_height(8.0),

            row![
                text_input("Merge into existing tag...", merge_input)
                    .on_input(|v| Message::PendingMergeInputChanged(tag.to_string(), v))
                    .padding([5, 10])
                    .width(220.0),
                Space::with_width(8.0),
                button(text("Merge as Alias").size(12))
                    .on_press(Message::PendingMerge(
                        tag.to_string(),
                        merge_input.to_string(),
                    ))
                    .padding([5, 10]),
            ]
            .align_y(Alignment::Center),
        ]
        .spacing(6)
        .padding(16)
        .width(Length::Fill),
    )
    .style(move |_| container::Style {
        background: Some(colors.surface().into()),
        border: iced::Border {
            color: colors.border(),
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    })
    .width(Length::Fill)
    .into()
}

// ── Rejected tags panel ────────────────────────────────────────────────────────

pub fn view_rejected(app: &AkTags) -> Element<'_, Message> {
    let colors = theme::default_colors(app.theme_type);

    let top_row = row![
        btn_plain("<- Back")
            .on_press(Message::SwitchPanel(Panel::Browser)),
        Space::with_width(12.0),
        text("Rejected Tags").size(16).color(colors.accent2()),
        Space::with_width(Length::Fill),
        if !app.rejected_tags.is_empty() {
            btn_plain("Clear All")
                .on_press(Message::ClearRejectedTags)
                .style(move |_, _| button::Style {
                    background: Some(colors.red().into()),
                    text_color: colors.bg(),
                    ..Default::default()
                })
                .into()
        } else {
            Space::with_width(0.0) as Element<'_, Message>
        },
    ]
    .align_y(Alignment::Center)
    .padding([8, 12]);

    if app.rejected_tags.is_empty() {
        return column![
            top_row,
            Space::with_height(20.0),
            container(
                text("No rejected tags.").size(14).color(colors.text_dim())
            )
            .width(Length::Fill)
            .center_x(Length::Fill),
        ]
        .width(Length::Fill)
        .into();
    }

    let chips: Vec<Element<'_, Message>> = app.rejected_tags.iter()
        .map(|tag| {
            let tag_owned = tag.clone();
            row![
                button(text(tag).size(12).color(colors.text()))
                    .padding([4, 8])
                    .style(move |_, _| button::Style {
                        background: Some(colors.tag_bg().into()),
                        text_color: colors.text(),
                        border: Default::default(),
                        shadow: Default::default(),
                    }),
                Space::with_width(4.0),
                button(text("×").size(12).color(colors.text_dim()))
                    .padding([4, 6])
                    .on_press(Message::UnrejectTag(tag_owned.clone()))
                    .style(move |_, _| button::Style {
                        background: Some(colors.surface2().into()),
                        text_color: colors.text_dim(),
                        ..Default::default()
                    }),
            ]
            .spacing(2)
            .into()
        })
        .collect();

    column![
        top_row,
        Space::with_height(12.0),
        scrollable(
            container(
                scrollable(
                    Row::with_children(chips).spacing(8).wrap()
                )
            )
            .padding([0, 12])
        )
        .height(Length::Fill),
    ]
    .width(Length::Fill)
    .into()
}

// ── Taxonomy panel ────────────────────────────────────────────────────────────

pub fn view_taxonomy(app: &AkTags) -> Element<'_, Message> {
    let colors = theme::default_colors(app.theme_type);

    // Top row: back + title
    let top_row = row![
        btn_plain("<- Back")
            .on_press(Message::SwitchPanel(Panel::Browser)),
        Space::with_width(16.0),
        text("Approved Tag Library").size(16),
        Space::with_width(Length::Fill),
    ]
    .align_y(Alignment::Center)
    .padding([16, 20]);

    // Second row: new tag inputs + category selector + add button
    // Category selector as compact pill buttons
    let cat_buttons: Vec<Element<'_, Message>> = CATEGORIES.iter()
        .map(|&cat| {
            let is_selected = app.new_tag_category == cat;
            let bg = if is_selected { colors.accent() } else { colors.surface2() };
            let fg = if is_selected { colors.bg() } else { colors.text() };
            button(text(cat).size(11).color(fg))
                .on_press(Message::NewTagCategoryChanged(cat.to_string()))
                .padding([4, 10])
                .style(move |_t, _s| button::Style {
                    background: Some(bg.into()),
                    text_color: fg,
                    border: iced::Border {
                        color: if is_selected { colors.accent() } else { colors.border() },
                        width: 1.0,
                        radius: 4.0.into(),
                    },
                    ..Default::default()
                })
                .into()
        })
        .collect();

    let add_row = row![
        text_input("tag name", &app.new_tag_name)
            .on_input(Message::NewTagNameChanged)
            .padding([6, 10])
            .width(140.0),
        Space::with_width(8.0),
        Row::with_children(cat_buttons).spacing(4.0),
        Space::with_width(8.0),
        text_input("aliases (comma separated)", &app.new_tag_aliases)
            .on_input(Message::NewTagAliasesChanged)
            .padding([6, 10])
            .width(180.0),
        Space::with_width(8.0),
        btn_accent("+ Add Tag").on_press(Message::AddNewTag),
    ]
    .align_y(Alignment::Center)
    .padding([12, 20]);

    // Build taxonomy sections with wrapped tag chips
    let mut by_category: std::collections::HashMap<String, Vec<&(String, crate::taxonomy::TagMeta)>> =
        std::collections::HashMap::new();
    for item in &app.taxonomy {
        by_category.entry(item.1.category.clone()).or_default().push(item);
    }

    let mut sections: Vec<Element<'_, Message>> = vec![];
    let mut cats: Vec<String> = by_category.keys().cloned().collect();
    cats.sort();

    for cat in cats {
        let tags = &by_category[&cat];
        let chips: Vec<Element<'_, Message>> = tags.iter()
            .map(|(name, meta)| taxonomy_tag_chip(name, meta, colors))
            .collect();

        sections.push(
            container(
                column![
                    row![
                        text(cat.clone()).size(13).color(colors.accent2()),
                        Space::with_width(8.0),
                        text(format!("{}", tags.len()))
                            .size(11)
                            .color(colors.text_dim()),
                    ]
                    .align_y(Alignment::Center),
                    Space::with_height(8.0),
                    scrollable(
                        Row::with_children(chips).spacing(6).wrap()
                    ).height(Length::Shrink),
                ]
                .spacing(0)
                .width(Length::Fill)
                .padding(12),
            )
            .style(move |_| container::Style {
                background: Some(colors.surface().into()),
                border: iced::Border {
                    color: colors.border(),
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            })
            .width(Length::Fill)
            .into()
        );
        sections.push(Space::with_height(12.0).into());
    }

    column![
        top_row,
        add_row,
        scrollable(
            Column::with_children(sections)
                .padding([0, 20])
                .width(Length::Fill)
        )
        .height(Length::Fill),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn taxonomy_tag_chip<'a>(
    name: &'a str,
    meta: &'a crate::taxonomy::TagMeta,
    colors: theme::ThemeColors,
) -> Element<'a, Message> {
    let aliases_text = if meta.aliases.is_empty() {
        String::new()
    } else {
        format!(" → {}", meta.aliases.join(", "))
    };
    let has_aliases = !aliases_text.is_empty();
    let name_owned = name.to_string();

    container(
        row![
            text(name).size(12).color(colors.text()),
            if has_aliases {
                Element::from(text(aliases_text).size(10).color(colors.text_dim()))
            } else {
                Element::from(Space::with_width(0.0))
            },
            Space::with_width(4.0),
            button(text("×").size(12).color(colors.red()))
                .on_press(Message::RemoveTaxonomyTag(name_owned))
                .padding([1, 4])
                .style(move |_t, _s| button::Style {
                    background: None,
                    text_color: colors.red(),
                    ..Default::default()
                }),
        ]
        .align_y(Alignment::Center)
        .spacing(2),
    )
    .style(move |_| container::Style {
        background: Some(colors.tag_bg().into()),
        border: iced::Border {
            color: colors.border(),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    })
    .padding([4, 8])
    .into()
}
