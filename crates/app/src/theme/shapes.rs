use eframe::egui::Rounding;

// ============================================================================
// Cosmo Downloader Geometry & Spacing Tokens
// ============================================================================

pub const R_SM: Rounding = Rounding { nw: 6.0, ne: 6.0, sw: 6.0, se: 6.0 };
pub const R: Rounding = Rounding { nw: 10.0, ne: 10.0, sw: 10.0, se: 10.0 };

pub const SHAPE_SMALL: Rounding = R_SM;
pub const SHAPE_MEDIUM: Rounding = R_SM;
pub const SHAPE_LARGE: Rounding = R;
pub const SHAPE_EXTRA_LARGE: Rounding = R;
pub const SHAPE_FULL: Rounding = R_SM;

pub const CONTROL_HEIGHT: f32 = 34.0;
pub const CHIP_HEIGHT: f32 = 28.0;

pub const S1: f32 = 4.0;
pub const S2: f32 = 8.0;
pub const S3: f32 = 12.0;
pub const S4: f32 = 16.0;
pub const S5: f32 = 24.0;
pub const S6: f32 = 32.0;
