use iced::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeType {
    Light,
    Dark,
    Eldritch,
}

pub struct Palette;

impl Palette {
    // ── Dark Theme (default) ────────────────────────────────────────────────
    pub const BG:         Color = Color { r: 0.059, g: 0.067, b: 0.090, a: 1.0 };
    pub const SURFACE:    Color = Color { r: 0.102, g: 0.114, b: 0.153, a: 1.0 };
    pub const SURFACE2:   Color = Color { r: 0.133, g: 0.149, b: 0.227, a: 1.0 };
    pub const BORDER:     Color = Color { r: 0.180, g: 0.200, b: 0.282, a: 1.0 };
    pub const ACCENT:     Color = Color { r: 0.486, g: 0.416, b: 0.969, a: 1.0 };
    pub const ACCENT2:    Color = Color { r: 0.353, g: 0.612, b: 0.961, a: 1.0 };
    pub const TEXT:       Color = Color { r: 0.886, g: 0.894, b: 0.937, a: 1.0 };
    pub const TEXT_DIM:   Color = Color { r: 0.545, g: 0.565, b: 0.690, a: 1.0 };
    pub const GREEN:      Color = Color { r: 0.290, g: 0.859, b: 0.502, a: 1.0 };
    pub const RED:        Color = Color { r: 0.973, g: 0.443, b: 0.443, a: 1.0 };
    pub const YELLOW:     Color = Color { r: 0.984, g: 0.749, b: 0.141, a: 1.0 };
    pub const ORANGE:     Color = Color { r: 0.984, g: 0.573, b: 0.137, a: 1.0 };
    pub const TAG_BG:     Color = Color { r: 0.165, g: 0.184, b: 0.290, a: 1.0 };

    // ── Light Theme ─────────────────────────────────────────────────────────
    pub const LIGHT_BG:       Color = Color { r: 0.973, g: 0.973, b: 0.973, a: 1.0 };
    pub const LIGHT_SURFACE:   Color = Color { r: 0.949, g: 0.949, b: 0.949, a: 1.0 };
    pub const LIGHT_SURFACE2:  Color = Color { r: 0.902, g: 0.902, b: 0.910, a: 1.0 };
    pub const LIGHT_BORDER:     Color = Color { r: 0.780, g: 0.780, b: 0.796, a: 1.0 };
    pub const LIGHT_ACCENT:    Color = Color { r: 0.176, g: 0.447, b: 0.890, a: 1.0 };
    pub const LIGHT_ACCENT2:   Color = Color { r: 0.306, g: 0.690, b: 0.310, a: 1.0 };
    pub const LIGHT_TEXT:      Color = Color { r: 0.110, g: 0.110, b: 0.110, a: 1.0 };
    pub const LIGHT_TEXT_DIM:  Color = Color { r: 0.455, g: 0.455, b: 0.475, a: 1.0 };
    pub const LIGHT_GREEN:    Color = Color { r: 0.145, g: 0.620, b: 0.204, a: 1.0 };
    pub const LIGHT_RED:      Color = Color { r: 0.839, g: 0.200, b: 0.200, a: 1.0 };
    pub const LIGHT_YELLOW:   Color = Color { r: 0.808, g: 0.620, b: 0.059, a: 1.0 };
    pub const LIGHT_ORANGE:   Color = Color { r: 0.839, g: 0.376, b: 0.059, a: 1.0 };
    pub const LIGHT_TAG_BG:   Color = Color { r: 0.867, g: 0.867, b: 0.878, a: 1.0 };

    // ── Eldritch Theme ────────────────────────────────────────────────────────
    pub const ELDRITCH_BG:        Color = Color { r: 0.129, g: 0.137, b: 0.216, a: 1.0 };
    pub const ELDRITCH_SURFACE:   Color = Color { r: 0.196, g: 0.204, b: 0.286, a: 1.0 };
    pub const ELDRITCH_SURFACE2:  Color = Color { r: 0.196, g: 0.204, b: 0.286, a: 1.0 };
    pub const ELDRITCH_BORDER:    Color = Color { r: 0.286, g: 0.310, b: 0.416, a: 1.0 };
    pub const ELDRITCH_COMMENT:    Color = Color { r: 0.439, g: 0.506, b: 0.816, a: 1.0 };
    pub const ELDRITCH_ACCENT:    Color = Color { r: 0.016, g: 0.820, b: 0.976, a: 1.0 };
    pub const ELDRITCH_ACCENT2:   Color = Color { r: 0.408, g: 0.612, b: 0.957, a: 1.0 };
    pub const ELDRITCH_TEXT:      Color = Color { r: 0.922, g: 0.980, b: 0.980, a: 1.0 };
    pub const ELDRITCH_TEXT_DIM:  Color = Color { r: 0.439, g: 0.506, b: 0.816, a: 1.0 };
    pub const ELDRITCH_GREEN:     Color = Color { r: 0.216, g: 0.957, b: 0.600, a: 1.0 };
    pub const ELDRITCH_YELLOW:    Color = Color { r: 0.914, g: 0.976, b: 0.255, a: 1.0 };
    pub const ELDRITCH_MAGENTA:   Color = Color { r: 0.949, g: 0.396, b: 0.710, a: 1.0 };
    pub const ELDRITCH_PURPLE:    Color = Color { r: 0.565, g: 0.443, b: 0.957, a: 1.0 };
    pub const ELDRITCH_TAG_BG:    Color = Color { r: 0.196, g: 0.204, b: 0.286, a: 1.0 };
}

pub fn iced_theme(theme_type: ThemeType) -> iced::Theme {
    match theme_type {
        ThemeType::Light => Theme::Light,
        ThemeType::Dark => Theme::Dark,
        ThemeType::Eldritch => Theme::Custom(
            iced::theme::Custom::new(iced::theme::Palette {
                background: iced::widget::container::Style {
                    background: Some(iced::Background::Color(Palette::ELDRITCH_BG)),
                    border_radius: 0.0.into(),
                    ..Default::default()
                },
                text: iced::widget::text::Style {
                    color: Some(Palette::ELDRITCH_TEXT),
                },
                ..Default::default()
            })
        ),
    }
}

pub const SIDEBAR_W:      f32 = 240.0;
pub const DETAIL_W:       f32 = 300.0;
pub const HEADER_H:       f32 = 52.0;
pub const THUMBNAIL_SIZE: u32 = 120;
pub const CARD_W:         f32 = 180.0;
pub const CARD_H:         f32 = 200.0;
pub const SPACING:        f32 = 12.0;
pub const PADDING:        f32 = 16.0;
pub const RADIUS:         f32 = 8.0;
