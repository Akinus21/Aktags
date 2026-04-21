use iced::Color;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeType {
    Light,
    Dark,
    PurpleHaze,
    Noctalia,
}

impl ThemeType {
    pub fn to_string(&self) -> String {
        match self {
            ThemeType::Light => "Light".to_string(),
            ThemeType::Dark => "Dark".to_string(),
            ThemeType::PurpleHaze => "PurpleHaze".to_string(),
            ThemeType::Noctalia => "Noctalia".to_string(),
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "Light" => ThemeType::Light,
            "PurpleHaze" => ThemeType::PurpleHaze,
            "Noctalia" => ThemeType::Noctalia,
            _ => ThemeType::Dark,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ThemeColors {
    pub bg: [f32; 4],
    pub surface: [f32; 4],
    pub surface2: [f32; 4],
    pub border: [f32; 4],
    pub accent: [f32; 4],
    pub accent2: [f32; 4],
    pub text: [f32; 4],
    pub text_dim: [f32; 4],
    pub green: [f32; 4],
    pub red: [f32; 4],
    pub yellow: [f32; 4],
    pub orange: [f32; 4],
    pub tag_bg: [f32; 4],
}

impl ThemeColors {
    pub fn bg(&self) -> Color { Color::from_rgba(self.bg[0], self.bg[1], self.bg[2], self.bg[3]) }
    pub fn surface(&self) -> Color { Color::from_rgba(self.surface[0], self.surface[1], self.surface[2], self.surface[3]) }
    pub fn surface2(&self) -> Color { Color::from_rgba(self.surface2[0], self.surface2[1], self.surface2[2], self.surface2[3]) }
    pub fn border(&self) -> Color { Color::from_rgba(self.border[0], self.border[1], self.border[2], self.border[3]) }
    pub fn accent(&self) -> Color { Color::from_rgba(self.accent[0], self.accent[1], self.accent[2], self.accent[3]) }
    pub fn accent2(&self) -> Color { Color::from_rgba(self.accent2[0], self.accent2[1], self.accent2[2], self.accent2[3]) }
    pub fn text(&self) -> Color { Color::from_rgba(self.text[0], self.text[1], self.text[2], self.text[3]) }
    pub fn text_dim(&self) -> Color { Color::from_rgba(self.text_dim[0], self.text_dim[1], self.text_dim[2], self.text_dim[3]) }
    pub fn green(&self) -> Color { Color::from_rgba(self.green[0], self.green[1], self.green[2], self.green[3]) }
    pub fn red(&self) -> Color { Color::from_rgba(self.red[0], self.red[1], self.red[2], self.red[3]) }
    pub fn yellow(&self) -> Color { Color::from_rgba(self.yellow[0], self.yellow[1], self.yellow[2], self.yellow[3]) }
    pub fn orange(&self) -> Color { Color::from_rgba(self.orange[0], self.orange[1], self.orange[2], self.orange[3]) }
    pub fn tag_bg(&self) -> Color { Color::from_rgba(self.tag_bg[0], self.tag_bg[1], self.tag_bg[2], self.tag_bg[3]) }
}

pub fn themes_dir() -> PathBuf {
    crate::config::config_dir().join("themes")
}

pub fn theme_file_path(name: &str) -> PathBuf {
    themes_dir().join(format!("{name}.json"))
}

pub fn list_themes() -> Vec<ThemeType> {
    let dir = themes_dir();
    if !dir.exists() {
        return vec![ThemeType::Dark, ThemeType::Light, ThemeType::PurpleHaze];
    }
    let mut themes = vec![ThemeType::Dark, ThemeType::Light, ThemeType::PurpleHaze];
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            if entry.path().extension().map(|ext| ext == "json").unwrap_or(false) {
                if let Some(stem) = entry.path().file_stem() {
                    let name = stem.to_string_lossy();
                    if !["Dark", "Light", "PurpleHaze"].contains(&name.as_ref()) {
                        themes.push(ThemeType::from_string(&name));
                    }
                }
            }
        }
    }
    themes
}

pub fn load_theme(theme_type: ThemeType) -> ThemeColors {
    let name = theme_type.to_string();
    let path = theme_file_path(&name);
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(colors) = serde_json::from_str::<ThemeColors>(&content) {
                return colors;
            }
        }
    }
    default_colors(theme_type)
}

pub fn save_theme(theme_type: ThemeType, colors: &ThemeColors) -> std::io::Result<()> {
    let dir = themes_dir();
    std::fs::create_dir_all(&dir)?;
    let path = theme_file_path(&theme_type.to_string());
    let json = serde_json::to_string_pretty(colors).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(path, json)
}

pub fn ensure_default_themes() {
    let dir = themes_dir();
    std::fs::create_dir_all(&dir).ok();
    for theme_type in [ThemeType::Dark, ThemeType::Light, ThemeType::PurpleHaze] {
        let path = theme_file_path(&theme_type.to_string());
        if !path.exists() {
            let colors = default_colors(theme_type);
            save_theme(theme_type, &colors).ok();
        }
    }
}

pub fn default_colors(t: ThemeType) -> ThemeColors {
    match t {
        ThemeType::Dark => ThemeColors {
            bg: [0.059, 0.067, 0.090, 1.0],
            surface: [0.102, 0.114, 0.153, 1.0],
            surface2: [0.133, 0.149, 0.227, 1.0],
            border: [0.180, 0.200, 0.282, 1.0],
            accent: [0.486, 0.416, 0.969, 1.0],
            accent2: [0.353, 0.612, 0.961, 1.0],
            text: [0.886, 0.894, 0.937, 1.0],
            text_dim: [0.545, 0.565, 0.690, 1.0],
            green: [0.290, 0.859, 0.502, 1.0],
            red: [0.973, 0.443, 0.443, 1.0],
            yellow: [0.984, 0.749, 0.141, 1.0],
            orange: [0.984, 0.573, 0.137, 1.0],
            tag_bg: [0.165, 0.184, 0.290, 1.0],
        },
        ThemeType::Light => ThemeColors {
            bg: [0.973, 0.973, 0.973, 1.0],
            surface: [0.949, 0.949, 0.949, 1.0],
            surface2: [0.902, 0.902, 0.910, 1.0],
            border: [0.780, 0.780, 0.796, 1.0],
            accent: [0.176, 0.447, 0.890, 1.0],
            accent2: [0.306, 0.690, 0.310, 1.0],
            text: [0.110, 0.110, 0.110, 1.0],
            text_dim: [0.455, 0.455, 0.475, 1.0],
            green: [0.145, 0.620, 0.204, 1.0],
            red: [0.839, 0.200, 0.200, 1.0],
            yellow: [0.808, 0.620, 0.059, 1.0],
            orange: [0.839, 0.376, 0.059, 1.0],
            tag_bg: [0.867, 0.867, 0.878, 1.0],
        },
        ThemeType::PurpleHaze => ThemeColors {
            bg: [0.082, 0.051, 0.149, 1.0],
            surface: [0.122, 0.090, 0.200, 1.0],
            surface2: [0.122, 0.090, 0.200, 1.0],
            border: [0.180, 0.140, 0.270, 1.0],
            accent: [0.545, 0.361, 0.965, 1.0],
            accent2: [0.000, 0.831, 1.000, 1.0],
            text: [0.878, 0.851, 0.961, 1.0],
            text_dim: [0.545, 0.500, 0.720, 1.0],
            green: [0.314, 0.980, 0.482, 1.0],
            red: [0.949, 0.396, 0.710, 1.0],
            yellow: [0.945, 0.980, 0.549, 1.0],
            orange: [0.949, 0.396, 0.710, 1.0],
            tag_bg: [0.122, 0.090, 0.200, 1.0],
        },
        ThemeType::Noctalia => ThemeColors {
            bg: [0.078, 0.078, 0.090, 1.0],
            surface: [0.118, 0.118, 0.140, 1.0],
            surface2: [0.145, 0.145, 0.170, 1.0],
            border: [0.200, 0.200, 0.230, 1.0],
            accent: [0.290, 0.392, 0.973, 1.0],
            accent2: [0.545, 0.361, 0.965, 1.0],
            text: [0.929, 0.922, 0.973, 1.0],
            text_dim: [0.600, 0.600, 0.680, 1.0],
            green: [0.314, 0.980, 0.482, 1.0],
            red: [0.949, 0.396, 0.710, 1.0],
            yellow: [0.945, 0.980, 0.549, 1.0],
            orange: [0.949, 0.396, 0.710, 1.0],
            tag_bg: [0.145, 0.145, 0.170, 1.0],
        },
    }
}

pub fn iced_theme(theme_type: ThemeType) -> iced::Theme {
    let colors = default_colors(theme_type);
    iced::Theme::Custom(
        Arc::new(iced::theme::Custom::new(
            theme_type.to_string(),
            iced::theme::Palette {
                background: colors.bg(),
                text: colors.text(),
                primary: colors.accent(),
                success: colors.green(),
                danger: colors.red(),
            },
        ))
    )
}

pub const SIDEBAR_W: f32 = 240.0;
pub const DETAIL_W: f32 = 300.0;
pub const HEADER_H: f32 = 52.0;
pub const THUMBNAIL_SIZE: u32 = 120;
pub const CARD_W: f32 = 180.0;
pub const CARD_H: f32 = 200.0;
pub const SPACING: f32 = 12.0;
pub const PADDING: f32 = 16.0;
pub const RADIUS: f32 = 8.0;
