// File: crates/chart-render-skia/src/lib.rs
// Summary: Skia renderer crate (stub). Will host GPU/CPU surfaces and text shaping.

use skia_safe as skia;

pub struct SkiaRenderer;

impl SkiaRenderer {
    pub fn new() -> Self { Self }

    /// Temporary stub to affirm crate wiring.
    pub fn render_stub(&self) {
        let _ = skia::Color::from_argb(255, 0, 0, 0);
    }
}

