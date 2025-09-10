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
}

