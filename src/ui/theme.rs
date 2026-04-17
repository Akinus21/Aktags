use iced::Color;

pub struct Palette;

impl Palette {
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
