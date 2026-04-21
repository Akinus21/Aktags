use iced::{
    widget::{button, column, container, row, scrollable, text, text_input, Column, Row, Space},
    Alignment, Element, Length,
};

use super::app::{AkTags, Message, Panel};
use super::theme;

const CATEGORIES: &[&str] = &["work", "education", "technical", "personal", "military", "misc"];

// ── Pending tags panel ────────────────────────────────────────────────────────

pub fn view_pending(app: &AkTags) -> Element<'_, Message> {
    let colors = theme::default_colors(app.theme_type);

    let header = row![
        button(text("<- Back").size(13))
            .on_press(Message::SwitchPanel(Panel::Browser))
            .padding([6, 14]),
        Space::with_width(16.0),
        column![
            text("Pending Tag Approvals").size(16),
            text("Tags proposed by AI not yet in your library. Approve, reject, or merge.")
                .size(12)
                .color(colors.text_dim()),
        ],
        Space::with_width(Length::Fill),
        button(text("Approve All").size(13))
            .on_press(Message::ApproveAll)
            .padding([6, 14]),
        Space::with_width(8.0),
        button(text("Reject All").size(13))
            .on_press(Message::RejectAll)
            .padding([6, 14]),
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

    let cat_buttons: Vec<Element<'_, Message>> = CATEGORIES.iter()
        .map(|&cat| {
            button(text(cat).size(11))
                .on_press(Message::PendingApprove(tag.to_string(), cat.to_string()))
                .padding([3, 8])
                .into()
        })
        .collect();

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
    .width(Length::Fill)
    .into()
}

// ── Taxonomy panel ────────────────────────────────────────────────────────────

pub fn view_taxonomy(app: &AkTags) -> Element<'_, Message> {
    let colors = theme::default_colors(app.theme_type);

    let header = row![
        button(text("<- Back").size(13))
            .on_press(Message::SwitchPanel(Panel::Browser))
            .padding([6, 14]),
        Space::with_width(16.0),
        text("Approved Tag Library").size(16),
        Space::with_width(Length::Fill),
        text_input("tag name", &app.new_tag_name)
            .on_input(Message::NewTagNameChanged)
            .padding([6, 10])
            .width(130.0),
        Space::with_width(8.0),
        {
            let cat_buttons: Vec<Element<'_, Message>> = CATEGORIES.iter()
                .map(|&cat| {
                    button(text(cat).size(11))
                        .on_press(Message::NewTagCategoryChanged(cat.to_string()))
                        .padding([4, 8])
                        .style(|_t, _s| button::Style::default())
                        .into()
                })
                .collect();
            Element::from(Row::with_children(cat_buttons).spacing(4))
        },
        Space::with_width(8.0),
        text_input("aliases (comma separated)", &app.new_tag_aliases)
            .on_input(Message::NewTagAliasesChanged)
            .padding([6, 10])
            .width(200.0),
        Space::with_width(8.0),
        button(text("+ Add Tag").size(13))
            .on_press(Message::AddNewTag)
            .padding([6, 14]),
    ]
    .align_y(Alignment::Center)
    .padding([16, 20]);

    let mut by_category: std::collections::HashMap<String, Vec<&(String, crate::taxonomy::TagMeta)>> =
        std::collections::HashMap::new();
    for item in &app.taxonomy {
        by_category.entry(item.1.category.clone()).or_default().push(item);
    }

    let mut sections: Vec<Element<'_, Message>> = vec![];
    let mut cats: Vec<&String> = by_category.keys().collect();
    cats.sort();

    for cat in cats {
        let tags = &by_category[cat];
        let tag_chips: Vec<Element<'_, Message>> = tags.iter()
            .map(|(name, meta)| taxonomy_tag_chip(name.clone(), meta, &colors))
            .collect();

        sections.push(
            column![
                text(format!("{cat} ({})", tags.len()))
                    .size(11)
                    .color(colors.text_dim()),
                Space::with_height(8.0),
                Row::with_children(tag_chips).spacing(8.0),
                Space::with_height(16.0),
            ]
            .spacing(0)
            .into()
        );
    }

    column![
        header,
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

fn taxonomy_tag_chip(
    name: String,
    meta: &crate::taxonomy::TagMeta,
    colors: &theme::ThemeColors,
) -> Element<'static, Message> {
    let aliases_text = if meta.aliases.is_empty() {
        String::new()
    } else {
        format!(" -> {}", meta.aliases.join(", "))
    };
    let name_for_text = name.clone();
    let aliases_for_text = aliases_text.clone();
    let has_aliases = !aliases_text.is_empty();

    row![
        text(name_for_text).size(13),
        if has_aliases {
            Element::from(text(aliases_for_text).size(11).color(colors.text_dim()))
        } else {
            Element::from(Space::with_width(0.0))
        },
        Space::with_width(6.0),
        button(text("X").size(12))
            .on_press(Message::RemoveTaxonomyTag(name))
            .padding([2, 5])
            .style(|_t, _s| button::Style::default()),
    ]
    .align_y(Alignment::Center)
    .spacing(2)
    .into()
}
