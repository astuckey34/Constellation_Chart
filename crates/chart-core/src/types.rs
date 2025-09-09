// File: crates/chart-core/src/types.rs
// Summary: Shared types and constants (sizes, colors, paddings).

/// Default surface width in pixels.
pub const WIDTH: i32 = 1024;
/// Default surface height in pixels.
pub const HEIGHT: i32 = 640;

/// Screen margins, in pixels.
/// Contract: all fields are non-negative.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Insets {
    pub left: u32,
    pub right: u32,
    pub top: u32,
    pub bottom: u32,
}

impl Insets {
    /// Create new insets (non-negative by type).
    pub const fn new(left: u32, right: u32, top: u32, bottom: u32) -> Self {
        Self { left, right, top, bottom }
    }
    /// Total horizontal inset (left + right).
    pub const fn hsum(&self) -> u32 { self.left + self.right }
    /// Total vertical inset (top + bottom).
    pub const fn vsum(&self) -> u32 { self.top + self.bottom }
}

impl Default for Insets {
    fn default() -> Self {
        Self::new(72, 24, 24, 56)
    }
}
