// File: crates/chart-core/src/chart.rs
// Summary: Chart struct and headless PNG rendering pipeline using Skia CPU raster surfaces.

use anyhow::Result;
use skia_safe as skia;

use crate::grid::linspace;
use crate::series::{Series, SeriesType};
use crate::types::{Insets, WIDTH, HEIGHT};
use crate::Axis;
use crate::theme::Theme;
use crate::scale::{TimeScale, ValueScale};
// For time-aware axis formatting



pub struct RenderOptions {
    pub width: i32,
    pub height: i32,
    pub insets: Insets,
    pub background: skia::Color,
    pub theme: Theme,
    pub draw_labels: bool,   // draw axis labels (set false for deterministic tests)
    pub crisp_lines: bool,   // align 1px lines to half-pixels for sharpness
    pub crosshair: Option<(f32, f32)>, // device px; when Some, draw crosshair overlay
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            width: WIDTH,
            height: HEIGHT,
            insets: Insets::default(),
            background: skia::Color::from_argb(255, 18, 18, 20), // kept for backwards-compat; unused if theme provided
            theme: Theme::dark(),
            draw_labels: true,
            crisp_lines: true,
            crosshair: None,
        }
    }
}

pub struct Chart {
    pub series: Vec<Series>,
    pub x_axis: Axis,
    pub y_axis: Axis,
}

impl Chart {
    pub fn new() -> Self {
        Self {
            series: Vec::new(),
            x_axis: Axis::default_x(),
            y_axis: Axis::default_y(),
        }
    }

    pub fn add_series(&mut self, series: Series) {
        self.series.push(series);
    }

    /// Auto-scale x/y axes to fit all attached series. Optional margin fraction expands the y range.
    pub fn autoscale_axes(&mut self, y_margin_frac: f64) {
        if self.series.is_empty() { return; }

        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        for s in &self.series {
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
            return;
        }
        if (x_max - x_min).abs() < 1e-12 { x_max = x_min + 1.0; }
        if (y_max - y_min).abs() < 1e-12 { y_max = y_min + 1.0; }

        // Apply margin to Y
        let span = y_max - y_min;
        let m = span * y_margin_frac.max(0.0);
        self.x_axis.min = x_min;
        self.x_axis.max = x_max;
        self.y_axis.min = y_min - m;
        self.y_axis.max = y_max + m;
    }

    /// Render the chart to a PNG at `output_png_path` using a CPU raster surface.
    pub fn render_to_png(
        &self,
        opts: &RenderOptions,
        output_png_path: impl AsRef<std::path::Path>,
    ) -> Result<()> {
        let bytes = self.render_to_png_bytes(opts)?;
        if let Some(parent) = output_png_path.as_ref().parent() { std::fs::create_dir_all(parent)?; }
        std::fs::write(output_png_path, &bytes)?;
        Ok(())
    }

    /// Render the chart and return PNG-encoded bytes (headless).
    pub fn render_to_png_bytes(&self, opts: &RenderOptions) -> Result<Vec<u8>> {
        let mut surface = skia::surfaces::raster_n32_premul((opts.width, opts.height))
            .ok_or_else(|| anyhow::anyhow!("failed to create raster surface"))?;
        let canvas = surface.canvas();
        self.draw_into(canvas, opts);
        let image = surface.image_snapshot();
        #[allow(deprecated)]
        let data = image
            .encode_to_data(skia::EncodedImageFormat::PNG)
            .ok_or_else(|| anyhow::anyhow!("encode PNG failed"))?;
        Ok(data.as_bytes().to_vec())
    }

    /// Render the chart into a CPU RGBA8 buffer (row-major), suitable for window blitting.
    /// Returns (pixels RGBA, width, height, row_bytes).
    pub fn render_to_rgba8(&self, opts: &RenderOptions) -> Result<(Vec<u8>, i32, i32, usize)> {
        let mut surface = skia::surfaces::raster_n32_premul((opts.width, opts.height))
            .ok_or_else(|| anyhow::anyhow!("failed to create raster surface"))?;
        let canvas = surface.canvas();
        self.draw_into(canvas, opts);

        let info = skia::ImageInfo::new(
            (opts.width, opts.height),
            skia::ColorType::RGBA8888,
            skia::AlphaType::Premul,
            None,
        );
        let row_bytes = (opts.width as usize) * 4;
        let mut pixels = vec![0u8; row_bytes * (opts.height as usize)];
        let ok = surface.read_pixels(&info, pixels.as_mut_slice(), row_bytes, (0, 0));
        if !ok {
            anyhow::bail!("read_pixels failed");
        }
        Ok((pixels, opts.width, opts.height, row_bytes))
    }

    fn draw_into(&self, canvas: &skia::Canvas, opts: &RenderOptions) {
        // Background
        canvas.clear(opts.theme.background);

        // Plot rect
        let plot_left = opts.insets.left as i32;
        let plot_right = opts.width - opts.insets.right as i32;
        let plot_top = opts.insets.top as i32;
        let plot_bottom = opts.height - opts.insets.bottom as i32;

        // Grid & axes
        draw_grid(canvas, plot_left, plot_top, plot_right, plot_bottom, opts.crisp_lines, &opts.theme);
        draw_axes(
            canvas,
            plot_left,
            plot_top,
            plot_right,
            plot_bottom,
            &self.x_axis,
            &self.y_axis,
            opts.draw_labels,
            opts.crisp_lines,
            &opts.theme,
        );

        // Series
        for s in &self.series {
            match s.series_type {
                SeriesType::Line => draw_line_series(
                    canvas, plot_left, plot_top, plot_right, plot_bottom, &self.x_axis, &self.y_axis, s, &opts.theme,
                ),
                SeriesType::Candlestick => draw_candle_series(
                    canvas, plot_left, plot_top, plot_right, plot_bottom, &self.x_axis, &self.y_axis, s, &opts.theme,
                ),
                SeriesType::Bar => draw_bar_series(
                    canvas, plot_left, plot_top, plot_right, plot_bottom, &self.x_axis, &self.y_axis, s, &opts.theme,
                ),
                SeriesType::Histogram => draw_histogram_series(
                    canvas, plot_left, plot_top, plot_right, plot_bottom, &self.x_axis, &self.y_axis, s, &opts.theme,
                ),
                SeriesType::Baseline => draw_baseline_series(
                    canvas, plot_left, plot_top, plot_right, plot_bottom, &self.x_axis, &self.y_axis, s, &opts.theme,
                ),
            }
        }

        // Crosshair overlay (if provided)
        if let Some((cx, cy)) = opts.crosshair {
            let ix = cx.clamp(plot_left as f32, (plot_right - 1) as f32);
            let iy = cy.clamp(plot_top as f32, (plot_bottom - 1) as f32);
            let mut paint = skia::Paint::default();
            paint.set_anti_alias(false);
            paint.set_style(skia::paint::Style::Stroke);
            paint.set_color(opts.theme.crosshair);
            paint.set_stroke_width(1.0);
            // horizontal
            canvas.draw_line((plot_left as f32, iy), (plot_right as f32, iy), &paint);
            // vertical
            canvas.draw_line((ix, plot_top as f32), (ix, plot_bottom as f32), &paint);
        }
    }
}

// ---- helpers ----------------------------------------------------------------

fn draw_grid(canvas: &skia::Canvas, l: i32, t: i32, r: i32, b: i32, crisp: bool, theme: &Theme) {
    let mut paint = skia::Paint::default();
    paint.set_color(theme.grid);
    paint.set_anti_alias(true);
    paint.set_stroke_width(1.0);

    // verticals
    for x in linspace(l as f64, r as f64, 10) {
        let xf = if crisp { align_half(x as f32) } else { x as f32 };
        canvas.draw_line((xf, t as f32), (xf, b as f32), &paint);
    }
    // horizontals
    for y in linspace(t as f64, b as f64, 6) {
        let yf = if crisp { align_half(y as f32) } else { y as f32 };
        canvas.draw_line((l as f32, yf), (r as f32, yf), &paint);
    }
}

fn draw_axes(
    canvas: &skia::Canvas,
    l: i32,
    t: i32,
    r: i32,
    b: i32,
    x: &Axis,
    y: &Axis,
    draw_labels: bool,
    crisp: bool,
    theme: &Theme,
) {
    let mut axis_paint = skia::Paint::default();
    axis_paint.set_color(theme.axis_line);
    axis_paint.set_anti_alias(true);
    axis_paint.set_stroke_width(1.5);

    // X and Y axis lines
    let bx = if crisp { align_half(b as f32) } else { b as f32 };
    let lx = if crisp { align_half(l as f32) } else { l as f32 };
    canvas.draw_line((l as f32, bx), (r as f32, bx), &axis_paint);
    canvas.draw_line((lx, t as f32), (lx, b as f32), &axis_paint);

    if draw_labels {
        let mut paint_text = skia::Paint::default();
        paint_text.set_color(theme.axis_label);
        paint_text.set_anti_alias(true);
        let mut font = choose_font(12.0);

        // Draw axis titles
        canvas.draw_str(&x.label, (r as f32 - 80.0, b as f32 + 28.0), &font, &paint_text);
        canvas.draw_str(&y.label, (l as f32 + 8.0, t as f32 + 14.0), &font, &paint_text);

        // Ticks configuration
        let target_xticks = 8usize;
        let target_yticks = 6usize;

        // Compute "nice" ticks in value space
        let xticks = nice_ticks(x.min, x.max, target_xticks.max(2));
        let yticks = nice_ticks(y.min, y.max, target_yticks.max(2));

        // Build scales to place ticks in pixel space
        let xspan = (x.max - x.min).max(1e-9);
        let ts = TimeScale::new(l as f32, x.min, ((r - l) as f32) / (xspan as f32));
        let vs = ValueScale::new(t as f32, b as f32, y.min, y.max);

        let sx = |vx: f64| -> f32 { ts.to_px(vx) };
        let sy = |vy: f64| -> f32 { vs.to_px(vy) };

        // Tick paints
        let mut tick_paint = skia::Paint::default();
        tick_paint.set_color(theme.tick);
        tick_paint.set_anti_alias(true);
        tick_paint.set_stroke_width(1.0);

        // X ticks and labels (bottom)
        for vx in xticks.iter().copied() {
            if !vx.is_finite() { continue; }
            let xpx = if crisp { align_half(sx(vx)) } else { sx(vx) };
            // small tick up from baseline
            canvas.draw_line((xpx, bx), (xpx, bx - 6.0), &tick_paint);
            // label
            let label = if detect_time_like(x.min, x.max).is_some() {
                format_time_tick(vx, x.min, x.max)
            } else {
                format_tick(vx, x.min, x.max)
            };
            // center roughly: shift by half label width
            let advance = font.measure_str(&label, Some(&paint_text)).0;
            canvas.draw_str(label, (xpx - advance * 0.5, b as f32 + 18.0), &font, &paint_text);
        }

        // Y ticks and labels (left)
        for vy in yticks.iter().copied() {
            if !vy.is_finite() { continue; }
            let ypx = if crisp { align_half(sy(vy)) } else { sy(vy) };
            // small tick to the right from axis
            canvas.draw_line((lx, ypx), (lx + 6.0, ypx), &tick_paint);
            // label to the left of axis, right-aligned
            let label = format_tick(vy, y.min, y.max);
            let advance = font.measure_str(&label, Some(&paint_text)).0;
            canvas.draw_str(label, (l as f32 - 8.0 - advance, ypx + 4.0), &font, &paint_text);
        }
    }
}

fn draw_line_series(
    canvas: &skia::Canvas,
    l: i32,
    t: i32,
    r: i32,
    b: i32,
    x_axis: &Axis,
    y_axis: &Axis,
    series: &Series,
    theme: &Theme,
) {
    let data = &series.data_xy;
    if data.len() < 2 {
        return;
    }

    // Scale helpers via TimeScale/ValueScale
    let xspan = (x_axis.max - x_axis.min).max(1e-9);
    let ts = TimeScale::new(l as f32, x_axis.min, ((r - l) as f32) / (xspan as f32));
    let vs = ValueScale::new(t as f32, b as f32, y_axis.min, y_axis.max);
    let sx = |x: f64| -> f32 { ts.to_px(x) };
    let sy = |y: f64| -> f32 { vs.to_px(y) };

    let mut path = skia::Path::new();
    let (x0, y0) = data[0];
    path.move_to((sx(x0), sy(y0)));

    for &(x, y) in data.iter().skip(1) {
        path.line_to((sx(x), sy(y)));
    }

    let mut stroke = skia::Paint::default();
    stroke.set_anti_alias(true);
    stroke.set_style(skia::paint::Style::Stroke);
    stroke.set_stroke_width(2.0);
    stroke.set_color(theme.line_stroke);

    canvas.draw_path(&path, &stroke);
}

fn draw_candle_series(
    canvas: &skia::Canvas,
    l: i32, t: i32, r: i32, b: i32,
    x_axis: &Axis, y_axis: &Axis,
    series: &Series,
    theme: &Theme,
) {
    if series.data_ohlc.is_empty() { return; }

    let xspan = (x_axis.max - x_axis.min).max(1e-9);
    let ts = TimeScale::new(l as f32, x_axis.min, ((r - l) as f32) / (xspan as f32));
    let vs = ValueScale::new(t as f32, b as f32, y_axis.min, y_axis.max);
    let sx = |x: f64| -> f32 { ts.to_px(x) };
    let sy = |y: f64| -> f32 { vs.to_px(y) };

    // style
    let mut wick = skia::Paint::default();
    wick.set_anti_alias(true);
    wick.set_style(skia::paint::Style::Stroke);
    wick.set_stroke_width(1.0);

    let mut body = skia::Paint::default();
    body.set_anti_alias(true);
    body.set_style(skia::paint::Style::Fill);

    // body width in pixels (roughly one “bar width” as fraction of plot)
    let n = series.data_ohlc.len() as f32;
    let bar_px = ((r - l) as f32 / n).max(3.0) * 0.7;

    for c in &series.data_ohlc {
        let x = sx(c.t);
        let y_o = sy(c.o);
        let y_h = sy(c.h);
        let y_l = sy(c.l);
        let y_c = sy(c.c);

        let up = c.c >= c.o;
        let color = if up { theme.candle_up } else { theme.candle_down };

        wick.set_color(color);
        body.set_color(color);

        // wick
        canvas.draw_line((x, y_h), (x, y_l), &wick);

        // body rect
        let half = bar_px * 0.5;
        let top = y_o.min(y_c);
        let bot = y_o.max(y_c);
        let rect = skia::Rect::from_ltrb(x - half, top, x + half, bot.max(top + 1.0));
        canvas.draw_rect(rect, &body);
    }
}

#[inline]
fn align_half(v: f32) -> f32 {
    v.floor() + 0.5
}

fn choose_font(size: f32) -> skia::Font {
    // Try a chain of common families using the system font manager so text draws across platforms.
    let families = [
        "Segoe UI",       // Windows
        "Arial",          // Windows/macOS
        "Helvetica",      // macOS
        "Roboto",         // Linux/Android
        "DejaVu Sans",    // Linux
    ];
    let fm = skia::FontMgr::default();
    for fam in families.iter() {
        if let Some(tf) = fm.match_family_style(fam, skia::FontStyle::normal()) {
            let mut f = skia::Font::from_typeface(tf, size);
            f.set_edging(skia::font::Edging::SubpixelAntiAlias);
            return f;
        }
    }
    let mut f = skia::Font::default();
    f.set_size(size);
    f.set_edging(skia::font::Edging::SubpixelAntiAlias);
    f
}

// Generate "nice" tick positions over [min, max]
fn nice_ticks(min: f64, max: f64, target: usize) -> Vec<f64> {
    if !min.is_finite() || !max.is_finite() || target < 2 { return vec![]; }
    let span = (max - min).abs();
    if span <= 0.0 { return vec![min]; }
    let raw_step = span / (target as f64);
    let step = nice_step(raw_step);
    let start = (min / step).ceil() * step;
    let end = (max / step).floor() * step;
    let mut out = Vec::new();
    let mut v = start;
    // guard against infinite loops
    for _ in 0..(target * 4) {
        if v > end + step * 0.5 { break; }
        out.push(v);
        v += step;
    }
    out
}

fn nice_step(raw: f64) -> f64 {
    // 1-2-5 scheme scaled by power of 10
    let power = raw.abs().log10().floor();
    let base = 10f64.powf(power);
    let n = raw / base;
    let nice = if n <= 1.0 { 1.0 } else if n <= 2.0 { 2.0 } else if n <= 5.0 { 5.0 } else { 10.0 };
    nice * base
}

fn format_tick(v: f64, min: f64, max: f64) -> String {
    let span = (max - min).abs().max(1e-12);
    let mag = span.abs().log10();
    let decimals = if mag >= 6.0 { 0 } else if mag >= 3.0 { 1 } else if mag >= 1.0 { 2 } else { 3 };
    format!("{:.*}", decimals as usize, v)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TimeUnit { Seconds, Millis }

fn detect_time_like(min: f64, max: f64) -> Option<TimeUnit> {
    if !min.is_finite() || !max.is_finite() { return None; }
    let lo = min.min(max);
    let hi = min.max(max);
    // Heuristics: UNIX epoch seconds (>= 2000-01-01 ~ 946684800)
    if lo >= 800_000_000.0 && hi < 4_000_000_000.0 { return Some(TimeUnit::Seconds); }
    // Milliseconds epoch ~ > 1e12
    if lo >= 1_000_000_000_000.0 && hi < 10_000_000_000_000.0 { return Some(TimeUnit::Millis); }
    None
}

fn format_time_tick(v: f64, min: f64, max: f64) -> String {
    let unit = detect_time_like(min, max).unwrap_or(TimeUnit::Seconds);
    // Convert to seconds resolution for formatting
    let secs = match unit {
        TimeUnit::Seconds => v,
        TimeUnit::Millis => v / 1000.0,
    };
    let span_secs = match unit {
        TimeUnit::Seconds => (max - min).abs(),
        TimeUnit::Millis => ((max - min).abs()) / 1000.0,
    };
    // Choose format based on total span
    let fmt = if span_secs < 120.0 {
        "%H:%M:%S"
    } else if span_secs < 7200.0 {
        "%H:%M"
    } else if span_secs < 86_400.0 {
        "%m-%d %H:%M"
    } else if span_secs < 31.0 * 86_400.0 {
        "%m-%d"
    } else {
        "%Y-%m-%d"
    };
    let secs_i = if secs.is_finite() { secs.floor() as i64 } else { 0 };
    if let Some(dt) = chrono::NaiveDateTime::from_timestamp_opt(secs_i, 0) {
        dt.format(fmt).to_string()
    } else {
        // Fallback to numeric
        format_tick(v, min, max)
    }
}

fn draw_bar_series(
    canvas: &skia::Canvas,
    l: i32, t: i32, r: i32, b: i32,
    x_axis: &Axis, y_axis: &Axis,
    series: &Series,
    theme: &Theme,
) {
    if series.data_ohlc.is_empty() { return; }

    let xspan = (x_axis.max - x_axis.min).max(1e-9);
    let ts = TimeScale::new(l as f32, x_axis.min, ((r - l) as f32) / (xspan as f32));
    let vs = ValueScale::new(t as f32, b as f32, y_axis.min, y_axis.max);
    let sx = |x: f64| -> f32 { ts.to_px(x) };
    let sy = |y: f64| -> f32 { vs.to_px(y) };

    let mut stroke = skia::Paint::default();
    stroke.set_anti_alias(true);
    stroke.set_style(skia::paint::Style::Stroke);
    stroke.set_stroke_width(1.0);

    // tick width ~ 40% of bar slot width
    let n = series.data_ohlc.len() as f32;
    let slot = ((r - l) as f32 / n).max(3.0);
    let tick = (slot * 0.4).max(2.0);

    for c in &series.data_ohlc {
        let x = sx(c.t);
        let y_o = sy(c.o);
        let y_h = sy(c.h);
        let y_l = sy(c.l);
        let y_c = sy(c.c);

        let up = c.c >= c.o;
        let color = if up { theme.candle_up } else { theme.candle_down };
        stroke.set_color(color);

        // main stem
        canvas.draw_line((x, y_h), (x, y_l), &stroke);
        // open tick to the left
        canvas.draw_line((x - tick * 0.5, y_o), (x, y_o), &stroke);
        // close tick to the right
        canvas.draw_line((x, y_c), (x + tick * 0.5, y_c), &stroke);
    }
}

fn draw_histogram_series(
    canvas: &skia::Canvas,
    l: i32, t: i32, r: i32, b: i32,
    x_axis: &Axis, y_axis: &Axis,
    series: &Series,
    theme: &Theme,
) {
    let data = &series.data_xy;
    if data.is_empty() { return; }

    let xspan = (x_axis.max - x_axis.min).max(1e-9);
    let ts = TimeScale::new(l as f32, x_axis.min, ((r - l) as f32) / (xspan as f32));
    let vs = ValueScale::new(t as f32, b as f32, y_axis.min, y_axis.max);
    let sx = |x: f64| -> f32 { ts.to_px(x) };
    let sy = |y: f64| -> f32 { vs.to_px(y) };

    let baseline_val = series.baseline.unwrap_or(0.0);
    let y0 = sy(baseline_val);

    // Estimate bar width from min pixel distance between consecutive x
    let mut min_dx = f32::INFINITY;
    for w in data.windows(2) {
        let dx = (sx(w[1].0) - sx(w[0].0)).abs();
        if dx > 0.0 { min_dx = min_dx.min(dx); }
    }
    if !min_dx.is_finite() { min_dx = ((r - l) as f32 / data.len() as f32).max(2.0); }
    let bw = (min_dx * 0.8).max(2.0);

    let mut fill = skia::Paint::default();
    fill.set_anti_alias(true);
    fill.set_style(skia::paint::Style::Fill);
    fill.set_color(theme.histogram);

    for &(xv, yv) in data {
        let x = sx(xv);
        let y = sy(yv);
        let half = bw * 0.5;
        let top = y.min(y0);
        let bot = y.max(y0);
        let rect = skia::Rect::from_ltrb(x - half, top, x + half, (bot).max(top + 1.0));
        canvas.draw_rect(rect, &fill);
    }
}

fn draw_baseline_series(
    canvas: &skia::Canvas,
    l: i32, t: i32, r: i32, b: i32,
    x_axis: &Axis, y_axis: &Axis,
    series: &Series,
    theme: &Theme,
) {
    let data = &series.data_xy;
    if data.len() < 2 { return; }

    let xspan = (x_axis.max - x_axis.min).max(1e-9);
    let ts = TimeScale::new(l as f32, x_axis.min, ((r - l) as f32) / (xspan as f32));
    let vs = ValueScale::new(t as f32, b as f32, y_axis.min, y_axis.max);
    let sx = |x: f64| -> f32 { ts.to_px(x) };
    let sy = |y: f64| -> f32 { vs.to_px(y) };

    let baseline_val = series.baseline.unwrap_or(0.0);
    let y0 = sy(baseline_val);

    // Stroke path
    let mut path = skia::Path::new();
    path.move_to((sx(data[0].0), sy(data[0].1)));
    for &(xv, yv) in data.iter().skip(1) {
        path.line_to((sx(xv), sy(yv)));
    }

    // Fill area to baseline (single-color area)
    let mut area = skia::Path::new();
    area.move_to((sx(data[0].0), y0));
    for &(xv, yv) in data.iter() {
        area.line_to((sx(xv), sy(yv)));
    }
    area.line_to((sx(data.last().unwrap().0), y0));
    area.close();

    let mut fill = skia::Paint::default();
    fill.set_anti_alias(true);
    fill.set_style(skia::paint::Style::Fill);
    fill.set_color(theme.baseline_fill);
    canvas.draw_path(&area, &fill);

    let mut stroke = skia::Paint::default();
    stroke.set_anti_alias(true);
    stroke.set_style(skia::paint::Style::Stroke);
    stroke.set_stroke_width(2.0);
    stroke.set_color(theme.baseline_stroke);
    canvas.draw_path(&path, &stroke);
}
