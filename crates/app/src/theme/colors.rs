use eframe::egui::Color32;

// ============================================================================
// Cosmo Downloader Design Tokens (Light & Dark, Steel Slate Accent)
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct ThemeColors {
    pub is_dark: bool,
    pub bg: Color32,
    pub surface: Color32,
    pub raise: Color32,
    pub line: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub on_accent: Color32,
    pub accent_soft: Color32,
    pub danger: Color32,
    pub error: Color32,
}

pub const LIGHT: ThemeColors = ThemeColors {
    is_dark: false,
    bg: Color32::from_rgb(0xF3, 0xF4, 0xF6),         // #f3f4f6
    surface: Color32::from_rgb(0xFF, 0xFF, 0xFF),    // #ffffff
    raise: Color32::from_rgb(0xEC, 0xEE, 0xF1),      // #eceef1
    line: Color32::from_rgba_premultiplied(2, 2, 3, 26), // rgba(20,24,30,.10)
    text: Color32::from_rgb(0x1B, 0x1D, 0x21),       // #1b1d21
    muted: Color32::from_rgb(0x64, 0x6B, 0x75),      // #646b75
    accent: Color32::from_rgb(0x44, 0x54, 0x6A),     // #44546a
    on_accent: Color32::from_rgb(0xFF, 0xFF, 0xFF),  // #ffffff
    accent_soft: Color32::from_rgba_premultiplied(8, 10, 13, 31), // rgba(68,84,106,.12)
    danger: Color32::from_rgb(0xD9, 0x30, 0x25),     // #d93025
    error: Color32::from_rgb(0xD9, 0x30, 0x25),
};

pub const DARK: ThemeColors = ThemeColors {
    is_dark: true,
    bg: Color32::from_rgb(0x1A, 0x1C, 0x20),         // #1a1c20
    surface: Color32::from_rgb(0x22, 0x25, 0x2A),    // #22252a
    raise: Color32::from_rgb(0x2B, 0x2F, 0x35),      // #2b2f35
    line: Color32::from_rgba_premultiplied(20, 20, 20, 20), // rgba(255,255,255,.08)
    text: Color32::from_rgb(0xE6, 0xE8, 0xEB),       // #e6e8eb
    muted: Color32::from_rgb(0x90, 0x97, 0xA1),      // #9097a1
    accent: Color32::from_rgb(0x8F, 0xA6, 0xC4),     // #8fa6c4
    on_accent: Color32::from_rgb(0x0F, 0x1A, 0x2A),  // #0f1a2a
    accent_soft: Color32::from_rgba_premultiplied(23, 27, 31, 41), // rgba(143,166,196,.16)
    danger: Color32::from_rgb(0xD9, 0x30, 0x25),     // #d93025
    error: Color32::from_rgb(0xE0, 0x6C, 0x75),
};

pub fn palette(is_dark: bool) -> &'static ThemeColors {
    if is_dark {
        &DARK
    } else {
        &LIGHT
    }
}

// Default aliases kept for compatibility
pub const BG: Color32 = DARK.bg;
pub const SURFACE: Color32 = DARK.surface;
pub const RAISE: Color32 = DARK.raise;
pub const LINE: Color32 = DARK.line;
pub const TEXT: Color32 = DARK.text;
pub const MUTED: Color32 = DARK.muted;
pub const ACCENT: Color32 = DARK.accent;
pub const ON_ACCENT: Color32 = DARK.on_accent;
pub const ACCENT_SOFT: Color32 = DARK.accent_soft;
pub const ERROR: Color32 = DARK.error;
pub const SURFACE_CONTAINER_HIGH: Color32 = DARK.surface;
