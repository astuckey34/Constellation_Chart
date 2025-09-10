// File: crates/chart-core/src/theme.rs
// Summary: Light/Dark theming for chart rendering colors.

use skia_safe as skia;

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub name: &'static str,
    pub background: skia::Color,
    pub grid: skia::Color,
    pub axis_line: skia::Color,
    pub axis_label: skia::Color,
    pub tick: skia::Color,
    pub crosshair: skia::Color,
    pub line_stroke: skia::Color,
    pub candle_up: skia::Color,
    pub candle_down: skia::Color,
    pub histogram: skia::Color,
    pub baseline_stroke: skia::Color,
    pub baseline_fill: skia::Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            name: "dark",
            background: skia::Color::from_argb(255, 18, 18, 20),
            grid: skia::Color::from_argb(255, 40, 40, 45),
            axis_line: skia::Color::from_argb(255, 180, 180, 190),
            axis_label: skia::Color::from_argb(255, 235, 235, 245),
            tick: skia::Color::from_argb(255, 150, 150, 160),
            crosshair: skia::Color::from_argb(255, 255, 230, 70),
            line_stroke: skia::Color::from_argb(255, 64, 160, 255),
            candle_up: skia::Color::from_argb(255, 40, 200, 120),
            candle_down: skia::Color::from_argb(255, 220, 80, 80),
            histogram: skia::Color::from_argb(255, 96, 156, 255),
            baseline_stroke: skia::Color::from_argb(255, 64, 160, 255),
            baseline_fill: skia::Color::from_argb(96, 64, 160, 255),
        }
    }

    pub fn light() -> Self {
        Self {
            name: "light",
            background: skia::Color::from_argb(255, 250, 250, 252),
            grid: skia::Color::from_argb(255, 230, 230, 235),
            axis_line: skia::Color::from_argb(255, 60, 60, 70),
            axis_label: skia::Color::from_argb(255, 20, 20, 30),
            tick: skia::Color::from_argb(255, 100, 100, 110),
            crosshair: skia::Color::from_argb(255, 30, 120, 240),
            line_stroke: skia::Color::from_argb(255, 32, 120, 200),
            candle_up: skia::Color::from_argb(255, 20, 160, 90),
            candle_down: skia::Color::from_argb(255, 200, 60, 60),
            histogram: skia::Color::from_argb(255, 40, 120, 200),
            baseline_stroke: skia::Color::from_argb(255, 32, 120, 200),
            baseline_fill: skia::Color::from_argb(80, 32, 120, 200),
        }
    }

    pub fn solarized_dark() -> Self {
        // Base colors from Solarized dark palette
        Self {
            name: "solarized-dark",
            background: skia::Color::from_argb(255, 0x00, 0x2b, 0x36), // base03
            grid: skia::Color::from_argb(255, 0x07, 0x36, 0x42),       // base02
            axis_line: skia::Color::from_argb(255, 0x93, 0xa1, 0xa1),  // base1
            axis_label: skia::Color::from_argb(255, 0xee, 0xe8, 0xd5), // base2
            tick: skia::Color::from_argb(255, 0x83, 0x94, 0x96),       // base0
            crosshair: skia::Color::from_argb(255, 0xb5, 0x89, 0x00),  // yellow
            line_stroke: skia::Color::from_argb(255, 0x26, 0x8b, 0xd2), // blue
            candle_up: skia::Color::from_argb(255, 0x2a, 0xa1, 0x98),   // cyan/green
            candle_down: skia::Color::from_argb(255, 0xdc, 0x32, 0x2f), // red
            histogram: skia::Color::from_argb(255, 0x26, 0x8b, 0xd2),
            baseline_stroke: skia::Color::from_argb(255, 0x26, 0x8b, 0xd2),
            baseline_fill: skia::Color::from_argb(96, 0x26, 0x8b, 0xd2),
        }
    }

    pub fn solarized_light() -> Self {
        Self {
            name: "solarized-light",
            background: skia::Color::from_argb(255, 0xfd, 0xf6, 0xe3), // base3
            grid: skia::Color::from_argb(255, 0xee, 0xe8, 0xd5),       // base2
            axis_line: skia::Color::from_argb(255, 0x65, 0x7b, 0x83), // base00
            axis_label: skia::Color::from_argb(255, 0x00, 0x2b, 0x36), // base03
            tick: skia::Color::from_argb(255, 0x58, 0x6e, 0x75),       // base01
            crosshair: skia::Color::from_argb(255, 0xcb, 0x4b, 0x16),  // orange
            line_stroke: skia::Color::from_argb(255, 0x26, 0x8b, 0xd2),
            candle_up: skia::Color::from_argb(255, 0x2a, 0xa1, 0x98),
            candle_down: skia::Color::from_argb(255, 0xdc, 0x32, 0x2f),
            histogram: skia::Color::from_argb(255, 0x26, 0x8b, 0xd2),
            baseline_stroke: skia::Color::from_argb(255, 0x26, 0x8b, 0xd2),
            baseline_fill: skia::Color::from_argb(80, 0x26, 0x8b, 0xd2),
        }
    }

    pub fn high_contrast_dark() -> Self {
        Self {
            name: "high-contrast-dark",
            background: skia::Color::from_argb(255, 0x00, 0x00, 0x00),
            grid: skia::Color::from_argb(255, 0x22, 0x22, 0x22),
            axis_line: skia::Color::from_argb(255, 0xff, 0xff, 0xff),
            axis_label: skia::Color::from_argb(255, 0xff, 0xff, 0xff),
            tick: skia::Color::from_argb(255, 0xcc, 0xcc, 0xcc),
            crosshair: skia::Color::from_argb(255, 0xff, 0xff, 0x00),
            line_stroke: skia::Color::from_argb(255, 0x00, 0xff, 0xff),
            candle_up: skia::Color::from_argb(255, 0x00, 0xff, 0x00),
            candle_down: skia::Color::from_argb(255, 0xff, 0x00, 0x00),
            histogram: skia::Color::from_argb(255, 0x00, 0xaa, 0xff),
            baseline_stroke: skia::Color::from_argb(255, 0x00, 0xaa, 0xff),
            baseline_fill: skia::Color::from_argb(120, 0x00, 0xaa, 0xff),
        }
    }
}

/// Return a list of built-in theme presets.
pub fn presets() -> Vec<Theme> {
    vec![
        Theme::dark(),
        Theme::light(),
        Theme::solarized_dark(),
        Theme::solarized_light(),
        Theme::high_contrast_dark(),
    ]
}

/// Find a theme by its `name`, falling back to dark.
pub fn find(name: &str) -> Theme {
    for t in presets() { if t.name.eq_ignore_ascii_case(name) { return t; } }
    Theme::dark()
}
