// File: crates/chart-core/src/scale.rs
// Summary: Time (X) and Value (Y) scale transforms with zoom/pan hooks.

/// Logical X coordinate (e.g., bar index or timestamp).
pub type Logical = f64;
/// Value Y coordinate (e.g., price).
pub type Value = f64;

/// General scale transform operations for X/Y axes.
pub trait ScaleTransform {
    fn to_screen_x(&self, x: Logical) -> f32;
    fn to_screen_y(&self, y: Value) -> f32;
    fn from_screen_x(&self, px: f32) -> Logical;
    fn from_screen_y(&self, py: f32) -> Value;
    fn zoom(&mut self, cx_logical: Logical, factor: f32);
    fn pan(&mut self, dx_px: f32, dy_px: f32);
}

/// Horizontal time scale controlled via logical start and bar spacing (px per logical).
#[derive(Clone, Copy, Debug)]
pub struct TimeScale {
    pub left_px: f32,
    pub start_logical: Logical,
    pub bar_spacing: f32,
}

impl TimeScale {
    pub fn new(left_px: f32, start_logical: Logical, bar_spacing: f32) -> Self {
        Self { left_px, start_logical, bar_spacing: bar_spacing.max(0.01) }
    }
    #[inline]
    pub fn to_px(&self, x: Logical) -> f32 {
        self.left_px + ((x - self.start_logical) as f32) * self.bar_spacing
    }
    #[inline]
    pub fn from_px(&self, px: f32) -> Logical {
        self.start_logical + ((px - self.left_px) / self.bar_spacing) as f64
    }
    pub fn zoom_at(&mut self, cursor_px: f32, factor: f32) {
        let cx = self.from_px(cursor_px);
        let new_spacing = (self.bar_spacing * factor).clamp(0.5, 200.0);
        // keep cx under cursor: solve for new start so to_px(cx) == cursor_px
        self.bar_spacing = new_spacing;
        self.start_logical = cx - ((cursor_px - self.left_px) / self.bar_spacing) as f64;
    }
    pub fn pan_px(&mut self, dx_px: f32) {
        self.start_logical -= (dx_px / self.bar_spacing) as f64;
    }
}

/// Vertical value scale mapping data range to [top, bottom] pixels.
#[derive(Clone, Copy, Debug)]
pub struct ValueScale {
    pub top_px: f32,
    pub bottom_px: f32,
    pub vmin: Value,
    pub vmax: Value,
    pub log: bool,
    // cached log endpoints when log is true
    log_min: f64,
    log_max: f64,
}

impl ValueScale {
    pub fn new_linear(top_px: f32, bottom_px: f32, vmin: Value, vmax: Value) -> Self {
        let mut s = Self { top_px, bottom_px, vmin, vmax, log: false, log_min: 0.0, log_max: 0.0 };
        if (s.vmax - s.vmin).abs() < 1e-12 { s.vmax = s.vmin + 1.0; }
        s
    }
    pub fn new_log10(top_px: f32, bottom_px: f32, mut vmin: Value, mut vmax: Value) -> Self {
        // Ensure strictly positive range for log scale
        let eps = 1e-12;
        vmin = if vmin <= eps { eps } else { vmin };
        vmax = if vmax <= vmin { vmin * 10.0 } else { vmax };
        let log_min = vmin.log10();
        let log_max = vmax.log10();
        Self { top_px, bottom_px, vmin, vmax, log: true, log_min, log_max }
    }
    #[inline]
    pub fn to_px(&self, y: Value) -> f32 {
        if self.log {
            let yy = y.max(1e-12).log10();
            let span = (self.log_max - self.log_min).max(1e-12);
            self.bottom_px - ((yy - self.log_min) / span) as f32 * (self.bottom_px - self.top_px)
        } else {
            let span = (self.vmax - self.vmin).max(1e-12);
            self.bottom_px - ((y - self.vmin) / span) as f32 * (self.bottom_px - self.top_px)
        }
    }
    #[inline]
    pub fn from_px(&self, py: f32) -> Value {
        if self.log {
            let span = (self.log_max - self.log_min).max(1e-12);
            let yy = self.log_min + ((self.bottom_px - py) / (self.bottom_px - self.top_px)) as f64 * span;
            10f64.powf(yy)
        } else {
            let span = (self.vmax - self.vmin).max(1e-12);
            self.vmin + ((self.bottom_px - py) / (self.bottom_px - self.top_px)) as f64 * span
        }
    }
    pub fn pan_px(&mut self, dy_px: f32) {
        if self.log {
            // Pan in log domain
            let span = (self.log_max - self.log_min).max(1e-12);
            let frac = dy_px / (self.bottom_px - self.top_px).max(1.0);
            let delta = (span as f32 * frac) as f64;
            self.log_min += delta;
            self.log_max += delta;
            self.vmin = 10f64.powf(self.log_min);
            self.vmax = 10f64.powf(self.log_max);
        } else {
            let span = (self.vmax - self.vmin).max(1e-12);
            let frac = dy_px / (self.bottom_px - self.top_px).max(1.0);
            let delta = (span as f32 * frac) as f64;
            self.vmin += delta;
            self.vmax += delta;
        }
    }
    pub fn zoom_center(&mut self, center_y: Value, factor: f32) {
        if self.log {
            let cy = center_y.max(1e-12).log10();
            let span = (self.log_max - self.log_min).max(1e-12);
            let new_span = (span as f32 / factor).max(1e-9) as f64;
            self.log_min = cy - new_span * 0.5;
            self.log_max = cy + new_span * 0.5;
            self.vmin = 10f64.powf(self.log_min);
            self.vmax = 10f64.powf(self.log_max);
        } else {
            // zoom around a value by shrinking/expanding range
            let span = (self.vmax - self.vmin).max(1e-12);
            let new_span = (span as f32 / factor).max(1e-9) as f64;
            self.vmin = center_y - new_span * 0.5;
            self.vmax = center_y + new_span * 0.5;
        }
    }
}
