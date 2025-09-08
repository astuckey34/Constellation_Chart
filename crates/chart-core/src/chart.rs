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
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            width: WIDTH,
            height: HEIGHT,
            insets: Insets::default(),
            background: skia::Color::from_argb(255, 18, 18, 20), // near-black
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

    /// Render the chart to a PNG at `output_png_path` using a CPU raster surface.
    pub fn render_to_png(
        &self,
        opts: &RenderOptions,
        output_png_path: impl AsRef<std::path::Path>,
    ) -> Result<()> {
        // Create raster surface
        let mut surface = skia::surfaces::raster_n32_premul((opts.width, opts.height))
            .ok_or_else(|| anyhow::anyhow!("failed to create raster surface"))?;
        let canvas = surface.canvas();

        // Background
        canvas.clear(opts.background);

        // Paddings & plot rect
        let plot_left = opts.insets.left;
        let plot_right = opts.width - opts.insets.right;
        let plot_top = opts.insets.top;
        let plot_bottom = opts.height - opts.insets.bottom;

        // Grid & axes
        draw_grid(canvas, plot_left, plot_top, plot_right, plot_bottom);
        draw_axes(
            canvas,
            plot_left,
            plot_top,
            plot_right,
            plot_bottom,
            &self.x_axis,
            &self.y_axis,
        );

        // Series
        for s in &self.series {
            match s.series_type {
                SeriesType::Line => draw_line_series(
                    canvas,
                    plot_left, plot_top, plot_right, plot_bottom,
                    &self.x_axis, &self.y_axis, s,
                ),
                SeriesType::Candlestick => draw_candle_series(
                    canvas,
                    plot_left, plot_top, plot_right, plot_bottom,
                    &self.x_axis, &self.y_axis, s,
                ),
            }
        }

        // Snapshot and write PNG
        let image = surface.image_snapshot();
        #[allow(deprecated)]
        let data = image
            .encode_to_data(skia::EncodedImageFormat::PNG)
            .ok_or_else(|| anyhow::anyhow!("encode PNG failed"))?;

        if let Some(parent) = output_png_path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(output_png_path, data.as_bytes())?;
        Ok(())
    }
}

// ---- helpers ----------------------------------------------------------------

fn draw_grid(canvas: &skia::Canvas, l: i32, t: i32, r: i32, b: i32) {
    let mut paint = skia::Paint::default();
    paint.set_color(skia::Color::from_argb(255, 40, 40, 45));
    paint.set_anti_alias(true);
    paint.set_stroke_width(1.0);

    // verticals
    for x in linspace(l as f64, r as f64, 10) {
        canvas.draw_line((x as f32, t as f32), (x as f32, b as f32), &paint);
    }
    // horizontals
    for y in linspace(t as f64, b as f64, 6) {
        canvas.draw_line((l as f32, y as f32), (r as f32, y as f32), &paint);
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
) {
    let mut axis_paint = skia::Paint::default();
    axis_paint.set_color(skia::Color::from_argb(255, 180, 180, 190));
    axis_paint.set_anti_alias(true);
    axis_paint.set_stroke_width(1.5);

    // X and Y axis lines
    canvas.draw_line((l as f32, b as f32), (r as f32, b as f32), &axis_paint);
    canvas.draw_line((l as f32, t as f32), (l as f32, b as f32), &axis_paint);

    // Labels
    let mut paint_text = skia::Paint::default();
    paint_text.set_color(skia::Color::from_argb(255, 210, 210, 220));
    let mut font = skia::Font::default();
    font.set_size(14.0);

    canvas.draw_str(&x.label, (r as f32 - 80.0, b as f32 + 24.0), &font, &paint_text);
    canvas.draw_str(&y.label, (l as f32 - 56.0, t as f32 + 14.0), &font, &paint_text);
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
