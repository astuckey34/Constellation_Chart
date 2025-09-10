// File: crates/chart-core/src/text.rs
// Summary: Simple text shaper/renderer using Skia textlayout with sensible defaults.

use skia_safe as skia;
use skia::textlayout::{FontCollection, Paragraph, ParagraphBuilder, ParagraphStyle, TextStyle};

pub struct TextShaper {
    fonts: FontCollection,
}

impl TextShaper {
    pub fn new() -> Self {
        let mut fc = FontCollection::new();
        // Use system manager fallback
        fc.set_default_font_manager(skia::FontMgr::default(), None);
        Self { fonts: fc }
    }

    fn make_style(size: f32, color: skia::Color, mono_numeric: bool) -> TextStyle {
        let mut ts = TextStyle::new();
        ts.set_font_size(size.max(1.0));
        ts.set_color(color);
        if mono_numeric {
            // Prefer monospaced/tabular-number families for numeric alignment
            ts.set_font_families(&["Roboto Mono", "Consolas", "Menlo", "DejaVu Sans Mono", "monospace"]);
        } else {
            ts.set_font_families(&["Segoe UI", "Arial", "Helvetica", "Roboto", "DejaVu Sans", "sans-serif"]);
        }
        ts
    }

    pub fn layout(&self, text: &str, size: f32, color: skia::Color, mono_numeric: bool) -> Paragraph {
        let mut pstyle = ParagraphStyle::new();
        pstyle.set_text_align(skia::textlayout::TextAlign::Left);
        let mut builder = ParagraphBuilder::new(&pstyle, &self.fonts);
        let style = Self::make_style(size, color, mono_numeric);
        builder.push_style(&style);
        builder.add_text(text);
        let mut paragraph = builder.build();
        paragraph.layout(10_000.0);
        paragraph
    }

    pub fn measure_width(&self, text: &str, size: f32, mono_numeric: bool) -> f32 {
        let p = self.layout(text, size, skia::Color::from_argb(0, 0, 0, 0), mono_numeric);
        // width of the longest line
        p.longest_line()
    }

    pub fn draw_left(&self, canvas: &skia::Canvas, text: &str, x: f32, y: f32, size: f32, color: skia::Color, mono_numeric: bool) {
        let mut p = self.layout(text, size, color, mono_numeric);
        // Paragraph draws from top-left; adjust baseline by glyph height approximation
        p.paint(canvas, (x, y - size * 0.8));
    }
}

