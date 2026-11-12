use eframe::egui::Color32;

// ============================================================================
// Cosmo Downloader Design Tokens (Quiet, Hand-Crafted, Dark & Light)
// ============================================================================

// Dark Tokens (Primary Default)
pub const BG_DARK: Color32 = Color32::from_rgb(0x18, 0x1A, 0x1E);        // #181a1e
pub const SURFACE_DARK: Color32 = Color32::from_rgb(0x20, 0x23, 0x28);   // #202328
pub const RAISE_DARK: Color32 = Color32::from_rgb(0x2B, 0x30, 0x38);     // #2b3038

// Premultiplied alpha: (255 * 20 / 255 = 20)
pub const LINE_DARK: Color32 = Color32::from_rgba_premultiplied(20, 20, 20, 20); // rgba(255,255,255,.08)
pub const TEXT_DARK: Color32 = Color32::from_rgb(0xF5, 0xF7, 0xFA);      // #f5f7fa — crisp, bright white
pub const MUTED_DARK: Color32 = Color32::from_rgb(0xB8, 0xC0, 0xCC);     // #b8c0cc — high contrast secondary text
pub const ACCENT_DARK: Color32 = Color32::from_rgb(0x8F, 0xB4, 0xFF);    // #8fb4ff
pub const ON_ACCENT_DARK: Color32 = Color32::from_rgb(0x0E, 0x1B, 0x36); // #0e1b36

// Premultiplied alpha: (143 * 31 / 255 = 17, 180 * 31 / 255 = 22, 255 * 31 / 255 = 31)
pub const ACCENT_SOFT_DARK: Color32 = Color32::from_rgba_premultiplied(17, 22, 31, 31); // 12% accent

// Light Tokens
pub const BG_LIGHT: Color32 = Color32::from_rgb(0xF3, 0xF4, 0xF6);       // #f3f4f6
pub const SURFACE_LIGHT: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);  // #ffffff
pub const RAISE_LIGHT: Color32 = Color32::from_rgb(0xEC, 0xEE, 0xF1);    // #eceef1
pub const LINE_LIGHT: Color32 = Color32::from_rgba_premultiplied(2, 2, 3, 26); // rgba(20,24,30,.1)
pub const TEXT_LIGHT: Color32 = Color32::from_rgb(0x1B, 0x1D, 0x21);     // #1b1d21
pub const MUTED_LIGHT: Color32 = Color32::from_rgb(0x5A, 0x61, 0x6C);    // #5a616c
pub const ACCENT_LIGHT: Color32 = Color32::from_rgb(0x2C, 0x5B, 0xD1);   // #2c5bd1
pub const ON_ACCENT_LIGHT: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);// #ffffff
pub const ACCENT_SOFT_LIGHT: Color32 = Color32::from_rgba_premultiplied(4, 9, 21, 26); // rgba(44,91,209,.1)

// Default / Active Dark Aliases
pub const BG: Color32 = BG_DARK;
pub const SURFACE: Color32 = SURFACE_DARK;
pub const RAISE: Color32 = RAISE_DARK;
pub const LINE: Color32 = LINE_DARK;
pub const TEXT: Color32 = TEXT_DARK;
pub const MUTED: Color32 = MUTED_DARK;
pub const ACCENT: Color32 = ACCENT_DARK;
pub const ON_ACCENT: Color32 = ON_ACCENT_DARK;
pub const ACCENT_SOFT: Color32 = ACCENT_SOFT_DARK;

// M3 Compatibility Aliases
pub const PRIMARY: Color32 = ACCENT;
pub const ON_PRIMARY: Color32 = ON_ACCENT;
pub const PRIMARY_CONTAINER: Color32 = ACCENT_SOFT;
pub const ON_PRIMARY_CONTAINER: Color32 = ACCENT;

pub const SECONDARY: Color32 = MUTED;
pub const ON_SECONDARY: Color32 = TEXT;
pub const SECONDARY_CONTAINER: Color32 = RAISE;
pub const ON_SECONDARY_CONTAINER: Color32 = TEXT;

pub const TERTIARY: Color32 = ACCENT;
pub const ON_TERTIARY: Color32 = ON_ACCENT;
pub const TERTIARY_CONTAINER: Color32 = ACCENT_SOFT;
pub const ON_TERTIARY_CONTAINER: Color32 = ACCENT;

pub const ERROR: Color32 = Color32::from_rgb(0xE0, 0x6C, 0x75);
pub const ON_ERROR: Color32 = Color32::WHITE;
pub const ERROR_CONTAINER: Color32 = Color32::from_rgba_premultiplied(26, 13, 14, 30);
pub const ON_ERROR_CONTAINER: Color32 = Color32::from_rgb(0xF0, 0xA0, 0xA5);

pub const SURFACE_DIM: Color32 = BG;
pub const SURFACE_BRIGHT: Color32 = RAISE;
pub const SURFACE_CONTAINER_LOWEST: Color32 = BG;
pub const SURFACE_CONTAINER_LOW: Color32 = BG;
pub const SURFACE_CONTAINER: Color32 = SURFACE;
pub const SURFACE_CONTAINER_HIGH: Color32 = SURFACE;
pub const SURFACE_CONTAINER_HIGHEST: Color32 = RAISE;
pub const SURFACE_VARIANT: Color32 = RAISE;

pub const ON_SURFACE: Color32 = TEXT;
pub const ON_SURFACE_VARIANT: Color32 = MUTED;
pub const OUTLINE: Color32 = LINE;
pub const OUTLINE_VARIANT: Color32 = LINE;

// Uniform Category Icons (Crisp Bright Neutral)
pub const CAT_VIDEO: Color32 = Color32::from_rgb(0xD0, 0xD6, 0xE2);
pub const CAT_MUSIC: Color32 = Color32::from_rgb(0xD0, 0xD6, 0xE2);
pub const CAT_DOCUMENTS: Color32 = Color32::from_rgb(0xD0, 0xD6, 0xE2);
pub const CAT_COMPRESSED: Color32 = Color32::from_rgb(0xD0, 0xD6, 0xE2);
pub const CAT_PROGRAMS: Color32 = Color32::from_rgb(0xD0, 0xD6, 0xE2);
pub const CAT_OTHER: Color32 = Color32::from_rgb(0xD0, 0xD6, 0xE2);
