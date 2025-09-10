// File: crates/chart-core/src/view.rs
// First-class view state: visible ranges and helpers for pan/zoom/autoscale.

use crate::{Chart};
use crate::series::SeriesType;
use crate::types::Insets;
use crate::scale::{TimeScale, ValueScale};

#[derive(Clone, Copy, Debug)]
pub struct ViewState {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
}

impl ViewState {
    pub fn from_chart(chart: &Chart) -> Self {
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        for s in &chart.series {
            match s.series_type {
                SeriesType::Line | SeriesType::Histogram | SeriesType::Baseline => {
                    for &(x, y) in &s.data_xy {
                        x_min = x_min.min(x);
                        x_max = x_max.max(x);
                        y_min = y_min.min(y);
                        y_max = y_max.max(y);
                    }
                    if let Some(b) = s.baseline { y_min = y_min.min(b); y_max = y_max.max(b); }
                }
                SeriesType::Candlestick | SeriesType::Bar => {
                    for c in &s.data_ohlc {
                        x_min = x_min.min(c.t);
                        x_max = x_max.max(c.t);
                        y_min = y_min.min(c.l);
                        y_max = y_max.max(c.h);
                    }
                }
            }
        }
        if !x_min.is_finite() || !x_max.is_finite() || !y_min.is_finite() || !y_max.is_finite() {
            return Self { x_min: 0.0, x_max: 1.0, y_min: 0.0, y_max: 1.0 };
        }
        if (x_max - x_min).abs() < 1e-9 { x_max = x_min + 1.0; }
        if (y_max - y_min).abs() < 1e-9 { y_max = y_min + 1.0; }
        let ym = (y_max - y_min) * 0.02;
        Self { x_min, x_max, y_min: y_min - ym, y_max: y_max + ym }
    }

    pub fn apply_to_chart(&self, chart: &mut Chart) {
        chart.x_axis.min = self.x_min;
        chart.x_axis.max = self.x_max;
        chart.y_axis.min = self.y_min;
        chart.y_axis.max = self.y_max;
    }

    pub fn pan_by_pixels(&mut self, dx: f64, dy: f64, width: i32, height: i32, insets: &Insets) {
        let plot_w = ((width - insets.right as i32 - insets.left as i32) as f64).max(1.0);
        let plot_h = ((height - insets.bottom as i32 - insets.top as i32) as f64).max(1.0);
        let x_span = self.x_max - self.x_min;
        let y_span = self.y_max - self.y_min;
        let wx = -dx / plot_w * x_span;
        let wy = dy / plot_h * y_span;
        self.x_min += wx; self.x_max += wx;
        self.y_min += wy; self.y_max += wy;
    }

    pub fn zoom_at_pixel(&mut self, scroll: f64, cursor_x: f64, cursor_y: f64, width: i32, height: i32, insets: &Insets) {
        let w = width as f64; let h = height as f64;
        let l = insets.left as f64; let rpx = (w - insets.right as f64);
        let t = insets.top as f64; let bpx = (h - insets.bottom as f64);
        let plot_w = (rpx - l).max(1.0); let plot_h = (bpx - t).max(1.0);
        let cx = cursor_x.clamp(l, rpx); let cy = cursor_y.clamp(t, bpx);
        let x_span = self.x_max - self.x_min; let y_span = self.y_max - self.y_min;
        let wx = self.x_min + (cx - l) / plot_w * x_span;
        let wy = self.y_max - (cy - t) / plot_h * y_span;
        let factor = (1.0 - scroll).clamp(0.1, 10.0);
        let nx = x_span * factor; let ny = y_span * factor;
        let rx = (wx - self.x_min) / x_span; let ry = (self.y_max - wy) / y_span;
        self.x_min = wx - rx * nx; self.x_max = self.x_min + nx;
        self.y_max = wy + ry * ny; self.y_min = self.y_max - ny;
    }

    pub fn autoscale_y_visible(&mut self, chart: &Chart) -> bool {
        if let Some((ymin, ymax)) = visible_y_range(chart, self.x_min, self.x_max) {
            let m = (ymax - ymin) * 0.02;
            self.y_min = ymin - m;
            self.y_max = ymax + m;
            true
        } else { false }
    }
}

pub fn visible_y_range(chart: &Chart, x_min: f64, x_max: f64) -> Option<(f64, f64)> {
    let mut y_min = f64::INFINITY;
    let mut y_max = f64::NEG_INFINITY;
    let mut any = false;
    for s in &chart.series {
        match s.series_type {
            SeriesType::Line | SeriesType::Histogram | SeriesType::Baseline => {
                for &(x, y) in &s.data_xy {
                    if x >= x_min && x <= x_max {
                        y_min = y_min.min(y);
                        y_max = y_max.max(y);
                        any = true;
                    }
                }
                if let Some(b) = s.baseline { y_min = y_min.min(b); y_max = y_max.max(b); }
            }
            SeriesType::Candlestick | SeriesType::Bar => {
                for c in &s.data_ohlc {
                    if c.t >= x_min && c.t <= x_max {
                        y_min = y_min.min(c.l);
                        y_max = y_max.max(c.h);
                        any = true;
                    }
                }
            }
        }
    }
    if any { Some((y_min, y_max)) } else { None }
}

