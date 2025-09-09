// File: crates/chart-core/src/chart.rs
// Summary: Chart struct and headless PNG rendering pipeline using Skia CPU raster surfaces.

use anyhow::Result;
use skia_safe as skia;

use crate::grid::linspace;
use crate::series::{Series, SeriesType};
use crate::types::{Insets, WIDTH, HEIGHT};
use crate::Axis;


pub struct RenderOptions {
    pub width: i32,
    pub height: i32,
    pub insets: Insets,
    pub background: skia::Color,
    pub draw_labels: bool,   // draw axis labels (set false for deterministic tests)
    pub crisp_lines: bool,   // align 1px lines to half-pixels for sharpness
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            width: WIDTH,
            height: HEIGHT,
            insets: Insets::default(),
            background: skia::Color::from_argb(255, 18, 18, 20), // near-black
            draw_labels: true,
            crisp_lines: true,
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
        canvas.clear(opts.background);

        // Plot rect
        let plot_left = opts.insets.left as i32;
        let plot_right = opts.width - opts.insets.right as i32;
        let plot_top = opts.insets.top as i32;
        let plot_bottom = opts.height - opts.insets.bottom as i32;

        // Grid & axes
        draw_grid(canvas, plot_left, plot_top, plot_right, plot_bottom, opts.crisp_lines);
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
        );

        // Series
        for s in &self.series {
            match s.series_type {
                SeriesType::Line => draw_line_series(
                    canvas, plot_left, plot_top, plot_right, plot_bottom, &self.x_axis, &self.y_axis, s,
                ),
                SeriesType::Candlestick => draw_candle_series(
                    canvas, plot_left, plot_top, plot_right, plot_bottom, &self.x_axis, &self.y_axis, s,
                ),
                SeriesType::Bar => draw_bar_series(
                    canvas, plot_left, plot_top, plot_right, plot_bottom, &self.x_axis, &self.y_axis, s,
                ),
                SeriesType::Histogram => draw_histogram_series(
                    canvas, plot_left, plot_top, plot_right, plot_bottom, &self.x_axis, &self.y_axis, s,
                ),
                SeriesType::Baseline => draw_baseline_series(
                    canvas, plot_left, plot_top, plot_right, plot_bottom, &self.x_axis, &self.y_axis, s,
                ),
            }
        }
    }
}

// ---- helpers ----------------------------------------------------------------

fn draw_grid(canvas: &skia::Canvas, l: i32, t: i32, r: i32, b: i32, crisp: bool) {
    let mut paint = skia::Paint::default();
    paint.set_color(skia::Color::from_argb(255, 40, 40, 45));
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
) {
    let mut axis_paint = skia::Paint::default();
    axis_paint.set_color(skia::Color::from_argb(255, 180, 180, 190));
    axis_paint.set_anti_alias(true);
    axis_paint.set_stroke_width(1.5);

    // X and Y axis lines
    let bx = if crisp { align_half(b as f32) } else { b as f32 };
    let lx = if crisp { align_half(l as f32) } else { l as f32 };
    let rx = if crisp { align_half(r as f32) } else { r as f32 };
    let ty = if crisp { align_half(t as f32) } else { t as f32 };
    canvas.draw_line((l as f32, bx), (r as f32, bx), &axis_paint);
    canvas.draw_line((lx, t as f32), (lx, b as f32), &axis_paint);

    if draw_labels {
        let mut paint_text = skia::Paint::default();
        paint_text.set_color(skia::Color::from_argb(255, 210, 210, 220));
        let mut font = skia::Font::default();
        font.set_size(14.0);
        canvas.draw_str(&x.label, (r as f32 - 80.0, b as f32 + 24.0), &font, &paint_text);
        canvas.draw_str(&y.label, (l as f32 - 56.0, t as f32 + 14.0), &font, &paint_text);
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
) {
    let data = &series.data_xy;
    if data.len() < 2 {
        return;
    }

    // Scale helpers
    let xspan = (x_axis.max - x_axis.min).max(1e-9);
    let yspan = (y_axis.max - y_axis.min).max(1e-9);
    let sx = |x: f64| -> f32 { l as f32 + ((x - x_axis.min) / xspan) as f32 * (r - l) as f32 };
    let sy = |y: f64| -> f32 { b as f32 - ((y - y_axis.min) / yspan) as f32 * (b - t) as f32 };

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
    stroke.set_color(skia::Color::from_argb(255, 64, 160, 255));

    canvas.draw_path(&path, &stroke);
}

fn draw_candle_series(
    canvas: &skia::Canvas,
    l: i32, t: i32, r: i32, b: i32,
    x_axis: &Axis, y_axis: &Axis,
    series: &Series,
) {
    if series.data_ohlc.is_empty() { return; }

    let xspan = (x_axis.max - x_axis.min).max(1e-9);
    let yspan = (y_axis.max - y_axis.min).max(1e-9);
    let sx = |x: f64| -> f32 { l as f32 + ((x - x_axis.min) / xspan) as f32 * (r - l) as f32 };
    let sy = |y: f64| -> f32 { b as f32 - ((y - y_axis.min) / yspan) as f32 * (b - t) as f32 };

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
        let color = if up {
            skia::Color::from_argb(255, 40, 200, 120)
        } else {
            skia::Color::from_argb(255, 220, 80, 80)
        };

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

fn draw_bar_series(
    canvas: &skia::Canvas,
    l: i32, t: i32, r: i32, b: i32,
    x_axis: &Axis, y_axis: &Axis,
    series: &Series,
) {
    if series.data_ohlc.is_empty() { return; }

    let xspan = (x_axis.max - x_axis.min).max(1e-9);
    let yspan = (y_axis.max - y_axis.min).max(1e-9);
    let sx = |x: f64| -> f32 { l as f32 + ((x - x_axis.min) / xspan) as f32 * (r - l) as f32 };
    let sy = |y: f64| -> f32 { b as f32 - ((y - y_axis.min) / yspan) as f32 * (b - t) as f32 };

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
        let color = if up {
            skia::Color::from_argb(255, 40, 200, 120)
        } else {
            skia::Color::from_argb(255, 220, 80, 80)
        };
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
) {
    let data = &series.data_xy;
    if data.is_empty() { return; }

    let xspan = (x_axis.max - x_axis.min).max(1e-9);
    let yspan = (y_axis.max - y_axis.min).max(1e-9);
    let sx = |x: f64| -> f32 { l as f32 + ((x - x_axis.min) / xspan) as f32 * (r - l) as f32 };
    let sy = |y: f64| -> f32 { b as f32 - ((y - y_axis.min) / yspan) as f32 * (b - t) as f32 };

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
    fill.set_color(skia::Color::from_argb(255, 96, 156, 255));

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
) {
    let data = &series.data_xy;
    if data.len() < 2 { return; }

    let xspan = (x_axis.max - x_axis.min).max(1e-9);
    let yspan = (y_axis.max - y_axis.min).max(1e-9);
    let sx = |x: f64| -> f32 { l as f32 + ((x - x_axis.min) / xspan) as f32 * (r - l) as f32 };
    let sy = |y: f64| -> f32 { b as f32 - ((y - y_axis.min) / yspan) as f32 * (b - t) as f32 };

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
    fill.set_color(skia::Color::from_argb(96, 64, 160, 255));
    canvas.draw_path(&area, &fill);

    let mut stroke = skia::Paint::default();
    stroke.set_anti_alias(true);
    stroke.set_style(skia::paint::Style::Stroke);
    stroke.set_stroke_width(2.0);
    stroke.set_color(skia::Color::from_argb(255, 64, 160, 255));
    canvas.draw_path(&path, &stroke);
}
