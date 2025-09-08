// File: crates/chart-core/src/types.rs
// Summary: Shared types and constants (sizes, colors, paddings).

pub const WIDTH: i32 = 1024;
pub const HEIGHT: i32 = 640;

pub struct Insets {
    pub left: i32,
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
}

impl Default for Insets {
    fn default() -> Self {
        Self { left: 72, right: 24, top: 24, bottom: 56 }
    }
}
