// File: crates/chart-core/src/geometry.rs
// Summary: Lightweight geometry helpers for pixel math.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RectI32 {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl RectI32 {
    pub const fn from_ltrb(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self { left, top, right, bottom }
    }
    pub const fn from_ltwh(left: i32, top: i32, width: i32, height: i32) -> Self {
        Self { left, top, right: left + width, bottom: top + height }
    }
    pub const fn width(&self) -> i32 { self.right - self.left }
    pub const fn height(&self) -> i32 { self.bottom - self.top }
}

#[inline]
pub fn clamp<T: PartialOrd>(v: T, lo: T, hi: T) -> T {
    if v < lo { lo } else if v > hi { hi } else { v }
}

