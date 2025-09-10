// File: crates/chart-core/src/chart.rs
// Summary: Chart struct and headless PNG rendering pipeline using Skia CPU raster surfaces.

use anyhow::Result;
use skia_safe as skia;

use crate::grid::linspace;
use crate::series::{Series, SeriesType};
use crate::types::{Insets, WIDTH, HEIGHT};
use crate::Axis;
use crate::theme::Theme;
use crate::axis::ScaleKind;
use crate::scale::{TimeScale, ValueScale};
use crate::text::TextShaper;
use crate::plugin::Overlay as OverlayTrait;
// For time-aware axis formatting



pub struct RenderOptions {
    pub width: i32,
    pub height: i32,
    pub insets: Insets,
    pub background: skia::Color,
    pub theme: Theme,
    pub draw_labels: bool,   // draw axis labels (set false for deterministic tests)
    pub show_tooltip: bool,  // when crosshair is present, render hover tooltip
    pub crisp_lines: bool,   // align 1px lines to half-pixels for sharpness
    pub crosshair: Option<(f32, f32)>, // device px; when Some, draw crosshair overlay
    pub dpr: f32,            // device pixel ratio for HiDPI (1.0 default)
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
            show_tooltip: false,
            crisp_lines: true,
            crosshair: None,
            dpr: 1.0,
        }
    }
}

pub struct Chart {
    pub series: Vec<Series>,
    pub x_axis: Axis,
    pub y_axis: Axis,
    pub overlays: Vec<Box<dyn OverlayTrait>>, // optional computed overlays
}

impl Chart {
    pub fn new() -> Self {
        Self {
            series: Vec::new(),
            x_axis: Axis::default_x(),
            y_axis: Axis::default_y(),
            overlays: Vec::new(),
        }
    }

    pub fn add_series(&mut self, series: Series) {
        self.series.push(series);
    }

    /// Add an overlay provider (computed series drawn above base series).
    pub fn add_overlay<O: OverlayTrait + 'static>(&mut self, overlay: O) {
        self.overlays.push(Box::new(overlay));
    }

    /// Remove all overlays.
    pub fn clear_overlays(&mut self) { self.overlays.clear(); }

    /// Dispatch an overlay event to all overlays (world coordinates).
    pub fn handle_overlay_event(&self, evt: &crate::plugin::OverlayEvent) {
        for ov in &self.overlays { ov.handle_event(evt, self); }
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

    /// Draw the chart onto an existing Skia canvas (CPU or GPU).
    /// The caller is responsible for canvas lifecycle and presentation.
    pub fn draw_onto_canvas(&self, canvas: &skia::Canvas, opts: &RenderOptions) {
        self.draw_into(canvas, opts);
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
            opts.dpr,
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

        // Overlays (computed)
        if !self.overlays.is_empty() {
            let mut overlay_theme = opts.theme;
            overlay_theme.line_stroke = opts.theme.crosshair;
            for ov in &self.overlays {
                let computed = ov.compute(self);
                for s in &computed {
                    if matches!(s.series_type, SeriesType::Line) {
                        draw_line_series(
                            canvas, plot_left, plot_top, plot_right, plot_bottom, &self.x_axis, &self.y_axis, s, &overlay_theme,
                        );
                    }
                }
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

            if opts.show_tooltip {
                draw_tooltip(
                    canvas,
                    plot_left, plot_top, plot_right, plot_bottom,
                    &self.x_axis, &self.y_axis,
                    &self.series,
                    ix, iy,
                    opts,
                );
            }
        }
    }

    /// Export the chart as an SVG file. Current implementation embeds a PNG as a data URI
    /// to ensure deterministic output without vectorization differences. This serves as a
    /// practical first step; a pure-vector SVG path can be added later.
    pub fn render_to_svg(
        &self,
        opts: &RenderOptions,
        output_svg_path: impl AsRef<std::path::Path>,
    ) -> Result<()> {
        fn color_to_rgba(c: skia::Color) -> (u8, u8, u8, u8) {
            (c.r(), c.g(), c.b(), c.a())
        }
        fn color_hex_rgb(c: skia::Color) -> String {
            let (r, g, b, _a) = color_to_rgba(c);
            format!("#{:02X}{:02X}{:02X}", r, g, b)
        }
        fn color_opacity(c: skia::Color) -> String {
            let (_r, _g, _b, a) = color_to_rgba(c);
            format!("{:.3}", (a as f32) / 255.0)
        }

        let w = opts.width.max(1) as i32;
        let h = opts.height.max(1) as i32;
        let l = opts.insets.left as i32;
        let rpx = w - opts.insets.right as i32;
        let t = opts.insets.top as i32;
        let bpx = h - opts.insets.bottom as i32;
        let crisp = opts.crisp_lines;
        let align = |v: f32| if crisp { v.floor() + 0.5 } else { v };

        let mut out = String::new();
        out.push_str(&format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">\n",
            w = w,
            h = h
        ));
        // Background
        out.push_str(&format!(
            "  <rect x=\"0\" y=\"0\" width=\"{w}\" height=\"{h}\" fill=\"{}\" fill-opacity=\"{}\" />\n",
            color_hex_rgb(opts.theme.background),
            color_opacity(opts.theme.background),
            w = w,
            h = h
        ));

        // Grid
        out.push_str("  <g id=\"grid\" stroke-linecap=\"butt\" stroke-width=\"1\" fill=\"none\">\n");
        let grid_col = color_hex_rgb(opts.theme.grid);
        let grid_op = color_opacity(opts.theme.grid);
        for x in linspace(l as f64, rpx as f64, 10) {
            let xf = align(x as f32);
            out.push_str(&format!(
                "    <line x1=\"{x}\" y1=\"{y1}\" x2=\"{x}\" y2=\"{y2}\" stroke=\"{col}\" stroke-opacity=\"{op}\" />\n",
                x = xf,
                y1 = t,
                y2 = bpx,
                col = grid_col,
                op = grid_op
            ));
        }
        for y in linspace(t as f64, bpx as f64, 6) {
            let yf = align(y as f32);
            out.push_str(&format!(
                "    <line x1=\"{x1}\" y1=\"{y}\" x2=\"{x2}\" y2=\"{y}\" stroke=\"{col}\" stroke-opacity=\"{op}\" />\n",
                x1 = l,
                x2 = rpx,
                y = yf,
                col = grid_col,
                op = grid_op
            ));
        }
        out.push_str("  </g>\n");

        // Axes
        let axis_col = color_hex_rgb(opts.theme.axis_line);
        let axis_op = color_opacity(opts.theme.axis_line);
        let bx = align(bpx as f32);
        let lx = align(l as f32);
        out.push_str(&format!(
            "  <g id=\"axes\" stroke=\"{col}\" stroke-opacity=\"{op}\" stroke-width=\"1.5\" fill=\"none\">\n    <line x1=\"{l}\" y1=\"{bx}\" x2=\"{r}\" y2=\"{bx}\" />\n    <line x1=\"{lx}\" y1=\"{t}\" x2=\"{lx}\" y2=\"{b}\" />\n  </g>\n",
            col = axis_col,
            op = axis_op,
            l = l,
            r = rpx,
            t = t,
            b = bpx,
            bx = bx,
            lx = lx
        ));

        // Ticks & labels
        if opts.draw_labels {
            let text_fill = color_hex_rgb(opts.theme.axis_label);
            let text_op = color_opacity(opts.theme.axis_label);
            let text_size = 12.0 * opts.dpr.max(0.5);

            // Titles
            out.push_str(&format!(
                "  <text x=\"{}\" y=\"{}\" fill=\"{col}\" fill-opacity=\"{op}\" font-size=\"{fs}\">{}</text>\n",
                rpx as f32 - 80.0 * opts.dpr,
                bpx as f32 + 28.0 * opts.dpr,
                self.x_axis.label,
                col = text_fill,
                op = text_op,
                fs = text_size
            ));
            out.push_str(&format!(
                "  <text x=\"{}\" y=\"{}\" fill=\"{col}\" fill-opacity=\"{op}\" font-size=\"{fs}\">{}</text>\n",
                l as f32 + 8.0 * opts.dpr,
                t as f32 + 14.0 * opts.dpr,
                self.y_axis.label,
                col = text_fill,
                op = text_op,
                fs = text_size
            ));

            let target_xticks = 8usize;
            let target_yticks = 6usize;
            let xticks = nice_ticks(self.x_axis.min, self.x_axis.max, target_xticks.max(2));
            let yticks = if self.y_axis.kind == ScaleKind::Log10 {
                log_ticks(self.y_axis.min.max(1e-12), self.y_axis.max, target_yticks.max(2))
            } else {
                nice_ticks(self.y_axis.min, self.y_axis.max, target_yticks.max(2))
            };
            let xspan = (self.x_axis.max - self.x_axis.min).max(1e-9);
            let ts = TimeScale::new(l as f32, self.x_axis.min, ((rpx - l) as f32) / (xspan as f32));
            let vs = match self.y_axis.kind {
                ScaleKind::Linear => ValueScale::new_linear(t as f32, bpx as f32, self.y_axis.min, self.y_axis.max),
                ScaleKind::Log10 => ValueScale::new_log10(t as f32, bpx as f32, self.y_axis.min, self.y_axis.max),
            };
            let sx = |vx: f64| -> f32 { ts.to_px(vx) };
            let sy = |vy: f64| -> f32 { vs.to_px(vy) };

            let tick_col = color_hex_rgb(opts.theme.tick);
            let tick_op = color_opacity(opts.theme.tick);
            out.push_str("  <g id=\"ticks\" fill=\"none\">\n");
            // X major ticks and labels
            for vx in xticks {
                if !vx.is_finite() { continue; }
                let xpx = align(sx(vx));
                out.push_str(&format!(
                    "    <line x1=\"{x}\" y1=\"{by}\" x2=\"{x}\" y2=\"{y2}\" stroke=\"{col}\" stroke-opacity=\"{op}\" stroke-width=\"1\" />\n",
                    x = xpx,
                    by = bx,
                    y2 = bx - 6.0 * opts.dpr,
                    col = tick_col,
                    op = tick_op
                ));
                let label = if detect_time_like(self.x_axis.min, self.x_axis.max).is_some() {
                    format_time_tick(vx, self.x_axis.min, self.x_axis.max)
                } else {
                    format_tick(vx, self.x_axis.min, self.x_axis.max)
                };
                out.push_str(&format!(
                    "    <text x=\"{x}\" y=\"{y}\" fill=\"{col}\" fill-opacity=\"{op}\" font-size=\"{fs}\" text-anchor=\"middle\">{label}</text>\n",
                    x = xpx,
                    y = bpx as f32 + 18.0 * opts.dpr,
                    col = text_fill,
                    op = text_op,
                    fs = text_size,
                    label = label
                ));
            }
            // Y major ticks and labels
            for vy in yticks {
                if !vy.is_finite() { continue; }
                let ypx = align(sy(vy));
                out.push_str(&format!(
                    "    <line x1=\"{x1}\" y1=\"{y}\" x2=\"{x2}\" y2=\"{y}\" stroke=\"{col}\" stroke-opacity=\"{op}\" stroke-width=\"1\" />\n",
                    x1 = lx,
                    x2 = lx + 6.0 * opts.dpr,
                    y = ypx,
                    col = tick_col,
                    op = tick_op
                ));
                let label = if self.y_axis.kind == ScaleKind::Log10 { format_log_tick(vy) } else { format_tick(vy, self.y_axis.min, self.y_axis.max) };
                out.push_str(&format!(
                    "    <text x=\"{x}\" y=\"{y}\" fill=\"{col}\" fill-opacity=\"{op}\" font-size=\"{fs}\" text-anchor=\"end\">{label}</text>\n",
                    x = l as f32 - 8.0 * opts.dpr,
                    y = ypx + 4.0 * opts.dpr,
                    col = text_fill,
                    op = text_op,
                    fs = text_size,
                    label = label
                ));
            }
            out.push_str("  </g>\n");
        }

        // Series
        let xspan = (self.x_axis.max - self.x_axis.min).max(1e-9);
        let ts = TimeScale::new(l as f32, self.x_axis.min, ((rpx - l) as f32) / (xspan as f32));
        let vs = match self.y_axis.kind {
            ScaleKind::Linear => ValueScale::new_linear(t as f32, bpx as f32, self.y_axis.min, self.y_axis.max),
            ScaleKind::Log10 => ValueScale::new_log10(t as f32, bpx as f32, self.y_axis.min, self.y_axis.max),
        };
        let sx = |vx: f64| -> f32 { ts.to_px(vx) };
        let sy = |vy: f64| -> f32 { vs.to_px(vy) };

        out.push_str("  <g id=\"series\" fill=\"none\" stroke-linejoin=\"round\" stroke-linecap=\"round\">\n");
        for s in &self.series {
            match s.series_type {
                SeriesType::Line => {
                    if s.data_xy.len() >= 2 {
                        let stroke = color_hex_rgb(opts.theme.line_stroke);
                        let sop = color_opacity(opts.theme.line_stroke);
                        let mut d = String::new();
                        let (x0, y0) = (sx(s.data_xy[0].0), sy(s.data_xy[0].1));
                        d.push_str(&format!("M {} {}", x0, y0));
                        for &(xv, yv) in s.data_xy.iter().skip(1) {
                            d.push_str(&format!(" L {} {}", sx(xv), sy(yv)));
                        }
                        out.push_str(&format!("    <path d=\"{d}\" stroke=\"{col}\" stroke-opacity=\"{op}\" stroke-width=\"2\" />\n", d = d, col = stroke, op = sop));
                    }
                }
                SeriesType::Histogram => {
                    if !s.data_xy.is_empty() {
                        let base = s.baseline.unwrap_or(0.0);
                        let y0 = sy(base);
                        let fill = color_hex_rgb(opts.theme.histogram);
                        let fop = color_opacity(opts.theme.histogram);
                        let mut wpx = ((rpx - l) as f32) / (s.data_xy.len().max(1) as f32) * 0.8;
                        if wpx < 1.0 { wpx = 1.0; }
                        for &(xv, yv) in &s.data_xy {
                            let cx = sx(xv);
                            let yy = sy(yv);
                            let (ymin, ymax) = if yy < y0 { (yy, y0) } else { (y0, yy) };
                            out.push_str(&format!(
                                "    <rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"{col}\" fill-opacity=\"{op}\" />\n",
                                x = cx - wpx * 0.5,
                                y = ymin,
                                w = wpx,
                                h = (ymax - ymin).abs().max(1.0),
                                col = fill,
                                op = fop
                            ));
                        }
                    }
                }
                SeriesType::Baseline => {
                    if s.data_xy.len() >= 2 {
                        let base = s.baseline.unwrap_or(0.0);
                        let y0 = sy(base);
                        let stroke = color_hex_rgb(opts.theme.baseline_stroke);
                        let sop = color_opacity(opts.theme.baseline_stroke);
                        let fill = color_hex_rgb(opts.theme.baseline_fill);
                        let fop = color_opacity(opts.theme.baseline_fill);
                        let mut d = String::new();
                        d.push_str(&format!("M {} {}", sx(s.data_xy[0].0), y0));
                        for &(xv, yv) in &s.data_xy { d.push_str(&format!(" L {} {}", sx(xv), sy(yv))); }
                        d.push_str(&format!(" L {} {} Z", sx(s.data_xy.last().unwrap().0), y0));
                        out.push_str(&format!("    <path d=\"{d}\" fill=\"{col}\" fill-opacity=\"{op}\" />\n", d = d, col = fill, op = fop));
                        let mut d2 = String::new();
                        d2.push_str(&format!("M {} {}", sx(s.data_xy[0].0), sy(s.data_xy[0].1)));
                        for &(xv, yv) in s.data_xy.iter().skip(1) { d2.push_str(&format!(" L {} {}", sx(xv), sy(yv))); }
                        out.push_str(&format!("    <path d=\"{d}\" stroke=\"{col}\" stroke-opacity=\"{op}\" stroke-width=\"2\" fill=\"none\" />\n", d = d2, col = stroke, op = sop));
                    }
                }
                SeriesType::Candlestick | SeriesType::Bar => {
                    if !s.data_ohlc.is_empty() {
                        let n = s.data_ohlc.len();
                        let mut wpx = ((rpx - l) as f32) / (n.max(1) as f32) * 0.6;
                        if wpx < 1.0 { wpx = 1.0; }
                        for c in &s.data_ohlc {
                            let x = sx(c.t);
                            let y_o = sy(c.o);
                            let y_c = sy(c.c);
                            let y_h = sy(c.h);
                            let y_l = sy(c.l);
                            let up = c.c >= c.o;
                            let col = if up { opts.theme.candle_up } else { opts.theme.candle_down };
                            let stroke = color_hex_rgb(col);
                            let sop = color_opacity(col);
                            out.push_str(&format!(
                                "    <line x1=\"{x}\" y1=\"{y1}\" x2=\"{x}\" y2=\"{y2}\" stroke=\"{col}\" stroke-opacity=\"{op}\" stroke-width=\"1\" />\n",
                                x = x,
                                y1 = y_l,
                                y2 = y_h,
                                col = stroke,
                                op = sop
                            ));
                            if matches!(s.series_type, SeriesType::Candlestick) {
                                let y_top = y_o.min(y_c);
                                let y_bot = y_o.max(y_c);
                                out.push_str(&format!(
                                    "    <rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"{col}\" fill-opacity=\"{op}\" stroke=\"{col}\" stroke-opacity=\"{op}\" stroke-width=\"1\" transform=\"translate({tx},0)\" />\n",
                                    x = -wpx * 0.5,
                                    y = y_top,
                                    w = wpx,
                                    h = (y_bot - y_top).abs().max(1.0),
                                    col = stroke,
                                    op = sop,
                                    tx = x
                                ));
                            } else {
                                let half = wpx * 0.5;
                                out.push_str(&format!(
                                    "    <line x1=\"{x1}\" y1=\"{y}\" x2=\"{x2}\" y2=\"{y}\" stroke=\"{col}\" stroke-opacity=\"{op}\" stroke-width=\"1\" />\n",
                                    x1 = x - half,
                                    x2 = x,
                                    y = y_o,
                                    col = stroke,
                                    op = sop
                                ));
                                out.push_str(&format!(
                                    "    <line x1=\"{x1}\" y1=\"{y}\" x2=\"{x2}\" y2=\"{y}\" stroke=\"{col}\" stroke-opacity=\"{op}\" stroke-width=\"1\" />\n",
                                    x1 = x,
                                    x2 = x + half,
                                    y = y_c,
                                    col = stroke,
                                    op = sop
                                ));
                            }
                        }
                    }
                }
            }
        }
        out.push_str("  </g>\n");
        // Overlays (computed)
        if !self.overlays.is_empty() {
            out.push_str("  <g id=\"overlays\" fill=\"none\" stroke-linejoin=\"round\" stroke-linecap=\"round\">\n");
            let (sr, sg, sb, sa) = (opts.theme.crosshair.r(), opts.theme.crosshair.g(), opts.theme.crosshair.b(), opts.theme.crosshair.a());
            let stroke = format!("#{:02X}{:02X}{:02X}", sr, sg, sb);
            let sop = format!("{:.3}", (sa as f32) / 255.0);
            for ov in &self.overlays {
                let computed = ov.compute(self);
                for s in &computed {
                    if matches!(s.series_type, SeriesType::Line) && s.data_xy.len() >= 2 {
                        let mut dpath = String::new();
                        dpath.push_str(&format!("M {} {}", sx(s.data_xy[0].0), sy(s.data_xy[0].1)));
                        for &(xv, yv) in s.data_xy.iter().skip(1) { dpath.push_str(&format!(" L {} {}", sx(xv), sy(yv))); }
                        out.push_str(&format!("    <path d=\"{d}\" stroke=\"{col}\" stroke-opacity=\"{op}\" stroke-width=\"1.5\" />\n", d = dpath, col = stroke, op = sop));
                    }
                }
            }
            out.push_str("  </g>\n");
        }

        out.push_str("</svg>\n");

        let path = output_svg_path.as_ref();
        if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
        std::fs::write(path, out)?;
        Ok(())
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
    dpr: f32,
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
        let text_size = 12.0 * dpr.max(0.5);
        let shaper = TextShaper::new();

        // Draw axis titles
        shaper.draw_left(canvas, &x.label, r as f32 - 80.0 * dpr, b as f32 + 28.0 * dpr, text_size, theme.axis_label, false);
        shaper.draw_left(canvas, &y.label, l as f32 + 8.0 * dpr, t as f32 + 14.0 * dpr, text_size, theme.axis_label, false);

        // Ticks configuration
        let target_xticks = 8usize;
        let target_yticks = 6usize;

        // Compute "nice" ticks in value space
        let xticks = nice_ticks(x.min, x.max, target_xticks.max(2));
        let yticks = if y.kind == ScaleKind::Log10 {
            log_ticks(y.min.max(1e-12), y.max, target_yticks.max(2))
        } else {
            nice_ticks(y.min, y.max, target_yticks.max(2))
        };

        // Build scales to place ticks in pixel space
        let xspan = (x.max - x.min).max(1e-9);
        let ts = TimeScale::new(l as f32, x.min, ((r - l) as f32) / (xspan as f32));
        let vs = match y.kind {
            ScaleKind::Linear => ValueScale::new_linear(t as f32, b as f32, y.min, y.max),
            ScaleKind::Log10 => ValueScale::new_log10(t as f32, b as f32, y.min, y.max),
        };

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
            canvas.draw_line((xpx, bx), (xpx, bx - 6.0 * dpr), &tick_paint);
            // label
            let label = if detect_time_like(x.min, x.max).is_some() {
                format_time_tick(vx, x.min, x.max)
            } else {
                format_tick(vx, x.min, x.max)
            };
            // center roughly: shift by half label width
            let advance = shaper.measure_width(&label, text_size, true);
            shaper.draw_left(canvas, &label, xpx - advance * 0.5, b as f32 + 18.0 * dpr, text_size, theme.axis_label, true);
        }

        // Y ticks and labels (left)
        for vy in yticks.iter().copied() {
            if !vy.is_finite() { continue; }
            let ypx = if crisp { align_half(sy(vy)) } else { sy(vy) };
            // small tick to the right from axis
            canvas.draw_line((lx, ypx), (lx + 6.0 * dpr, ypx), &tick_paint);
            // label to the left of axis, right-aligned
            let label = if y.kind == ScaleKind::Log10 { format_log_tick(vy) } else { format_tick(vy, y.min, y.max) };
            let advance = shaper.measure_width(&label, text_size, true);
            shaper.draw_left(canvas, &label, l as f32 - 8.0 * dpr - advance, ypx + 4.0 * dpr, text_size, theme.axis_label, true);
        }

        // Minor ticks (no labels)
        let mut minor_paint = tick_paint.clone();
        minor_paint.set_color(skia::Color::from_argb(180, 120, 120, 130));
        minor_paint.set_stroke_width(0.8);

        // X minor ticks (linear only, between majors)
        let x_minors = minor_ticks_linear(&xticks, 4);
        for vx in x_minors {
            if !vx.is_finite() { continue; }
            let xpx = if crisp { align_half(sx(vx)) } else { sx(vx) };
            canvas.draw_line((xpx, bx), (xpx, bx - 3.0), &minor_paint);
        }
        // Y minor ticks (linear: subdiv; log: 2..9 per decade)
        if y.kind == ScaleKind::Log10 {
            let y_minors = minor_ticks_log(y.min.max(1e-12), y.max);
            for vy in y_minors { let ypx = if crisp { align_half(sy(vy)) } else { sy(vy) }; canvas.draw_line((lx, ypx), (lx + 3.0, ypx), &minor_paint); }
        } else {
            let y_minors = minor_ticks_linear(&yticks, 4);
            for vy in y_minors { let ypx = if crisp { align_half(sy(vy)) } else { sy(vy) }; canvas.draw_line((lx, ypx), (lx + 3.0, ypx), &minor_paint); }
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
    let vs = match y_axis.kind {
        ScaleKind::Linear => ValueScale::new_linear(t as f32, b as f32, y_axis.min, y_axis.max),
        ScaleKind::Log10 => ValueScale::new_log10(t as f32, b as f32, y_axis.min, y_axis.max),
    };
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
    let vs = match y_axis.kind {
        ScaleKind::Linear => ValueScale::new_linear(t as f32, b as f32, y_axis.min, y_axis.max),
        ScaleKind::Log10 => ValueScale::new_log10(t as f32, b as f32, y_axis.min, y_axis.max),
    };
    let sx = |x: f64| -> f32 { ts.to_px(x) };
    let sy = |y: f64| -> f32 { vs.to_px(y) };

    // style
    let mut wick_paint = skia::Paint::default();
    wick_paint.set_anti_alias(true);
    wick_paint.set_style(skia::paint::Style::Stroke);
    wick_paint.set_stroke_width(1.0);

    let mut body_paint_up = skia::Paint::default();
    body_paint_up.set_anti_alias(true);
    body_paint_up.set_style(skia::paint::Style::Fill);
    body_paint_up.set_color(theme.candle_up);

    let mut body_paint_down = skia::Paint::default();
    body_paint_down.set_anti_alias(true);
    body_paint_down.set_style(skia::paint::Style::Fill);
    body_paint_down.set_color(theme.candle_down);

    // Batching: build combined paths for wicks and bodies (up/down)
    let mut wick_path_up = skia::Path::new();
    let mut wick_path_down = skia::Path::new();
    let mut body_path_up = skia::Path::new();
    let mut body_path_down = skia::Path::new();

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
        // wick into path
        if up {
            wick_path_up.move_to((x, y_h));
            wick_path_up.line_to((x, y_l));
        } else {
            wick_path_down.move_to((x, y_h));
            wick_path_down.line_to((x, y_l));
        }

        // body rect into path
        let half = bar_px * 0.5;
        let top = y_o.min(y_c);
        let bot = y_o.max(y_c);
        let rect = skia::Rect::from_ltrb(x - half, top, x + half, bot.max(top + 1.0));
        if up {
            body_path_up.add_rect(rect, None);
        } else {
            body_path_down.add_rect(rect, None);
        }
    }

    // stroke wicks by color
    wick_paint.set_color(theme.candle_up);
    canvas.draw_path(&wick_path_up, &wick_paint);
    wick_paint.set_color(theme.candle_down);
    canvas.draw_path(&wick_path_down, &wick_paint);

    // fill bodies by color
    canvas.draw_path(&body_path_up, &body_paint_up);
    canvas.draw_path(&body_path_down, &body_paint_down);
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

fn log_ticks(min: f64, max: f64, target: usize) -> Vec<f64> {
    if min <= 0.0 || !min.is_finite() || !max.is_finite() || target < 2 { return vec![]; }
    let start = min.log10().floor() as i32;
    let end = max.log10().ceil() as i32;
    let mut out = Vec::new();
    for k in start..=end {
        out.push(10f64.powi(k));
    }
    out
}

fn format_log_tick(v: f64) -> String {
    if v.abs() >= 1.0 {
        format!("{:.0}", v)
    } else {
        // scientific for small values
        let exp = v.abs().log10().floor() as i32;
        format!("1e{}", exp)
    }
}

fn format_tick(v: f64, min: f64, max: f64) -> String {
    let span = (max - min).abs().max(1e-12);
    // Use SI prefixes for large spans
    if span >= 1e6 {
        return format_si(v);
    }
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
    if let Some(dt) = chrono::DateTime::<chrono::Utc>::from_timestamp(secs_i, 0) {
        dt.format(fmt).to_string()
    } else {
        // Fallback to numeric
        format_tick(v, min, max)
    }
}

fn minor_ticks_linear(majors: &[f64], n: usize) -> Vec<f64> {
    if majors.len() < 2 || n == 0 { return vec![]; }
    let mut out = Vec::new();
    for w in majors.windows(2) {
        let a = w[0]; let b = w[1];
        let step = (b - a) / (n as f64 + 1.0);
        for i in 1..=n {
            out.push(a + step * (i as f64));
        }
    }
    out
}

fn minor_ticks_log(min: f64, max: f64) -> Vec<f64> {
    if min <= 0.0 { return vec![]; }
    let start = min.log10().floor() as i32;
    let end = max.log10().ceil() as i32;
    let mut out = Vec::new();
    for k in start..=end {
        let base = 10f64.powi(k);
        for m in 2..10 { // 2..=9
            let v = base * (m as f64);
            if v >= min && v <= max { out.push(v); }
        }
    }
    out
}

fn format_si(v: f64) -> String {
    let av = v.abs();
    let (unit, div) = if av >= 1e12 { ("T", 1e12) }
        else if av >= 1e9 { ("B", 1e9) }
        else if av >= 1e6 { ("M", 1e6) }
        else if av >= 1e3 { ("K", 1e3) }
        else { ("", 1.0) };
    if unit.is_empty() { return format!("{:.2}", v); }
    let val = v / div;
    if av >= 1e9 { format!("{:.2}{}", val, unit) } else { format!("{:.1}{}", val, unit) }
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
    let vs = match y_axis.kind {
        ScaleKind::Linear => ValueScale::new_linear(t as f32, b as f32, y_axis.min, y_axis.max),
        ScaleKind::Log10 => ValueScale::new_log10(t as f32, b as f32, y_axis.min, y_axis.max),
    };
    let sx = |x: f64| -> f32 { ts.to_px(x) };
    let sy = |y: f64| -> f32 { vs.to_px(y) };

    let mut stroke = skia::Paint::default();
    stroke.set_anti_alias(true);
    stroke.set_style(skia::paint::Style::Stroke);
    stroke.set_stroke_width(1.0);

    // Batch into two paths by up/down color
    let mut path_up = skia::Path::new();
    let mut path_down = skia::Path::new();

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
        if up {
            path_up.move_to((x, y_h)); path_up.line_to((x, y_l));
            path_up.move_to((x - tick * 0.5, y_o)); path_up.line_to((x, y_o));
            path_up.move_to((x, y_c)); path_up.line_to((x + tick * 0.5, y_c));
        } else {
            path_down.move_to((x, y_h)); path_down.line_to((x, y_l));
            path_down.move_to((x - tick * 0.5, y_o)); path_down.line_to((x, y_o));
            path_down.move_to((x, y_c)); path_down.line_to((x + tick * 0.5, y_c));
        }
    }

    stroke.set_color(theme.candle_up);
    canvas.draw_path(&path_up, &stroke);
    stroke.set_color(theme.candle_down);
    canvas.draw_path(&path_down, &stroke);
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
    let vs = match y_axis.kind {
        ScaleKind::Linear => ValueScale::new_linear(t as f32, b as f32, y_axis.min, y_axis.max),
        ScaleKind::Log10 => ValueScale::new_log10(t as f32, b as f32, y_axis.min, y_axis.max),
    };
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

    // Batch: accumulate rects into a single path
    let mut path = skia::Path::new();
    for &(xv, yv) in data {
        let x = sx(xv);
        let y = sy(yv);
        let half = bw * 0.5;
        let top = y.min(y0);
        let bot = y.max(y0);
        let rect = skia::Rect::from_ltrb(x - half, top, x + half, (bot).max(top + 1.0));
        path.add_rect(rect, None);
    }
    canvas.draw_path(&path, &fill);
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
    let vs = match y_axis.kind {
        ScaleKind::Linear => ValueScale::new_linear(t as f32, b as f32, y_axis.min, y_axis.max),
        ScaleKind::Log10 => ValueScale::new_log10(t as f32, b as f32, y_axis.min, y_axis.max),
    };
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

fn draw_tooltip(
    canvas: &skia::Canvas,
    l: i32, t: i32, r: i32, b: i32,
    x_axis: &Axis, y_axis: &Axis,
    series_list: &[Series],
    cx: f32, cy: f32,
    opts: &RenderOptions,
) {
    if series_list.is_empty() { return; }

    // Build scales to translate between px and data
    let xspan = (x_axis.max - x_axis.min).max(1e-9);
    let ts = TimeScale::new(l as f32, x_axis.min, ((r - l) as f32) / (xspan as f32));
    let vs = match y_axis.kind {
        ScaleKind::Linear => ValueScale::new_linear(t as f32, b as f32, y_axis.min, y_axis.max),
        ScaleKind::Log10 => ValueScale::new_log10(t as f32, b as f32, y_axis.min, y_axis.max),
    };
    let to_logical = |px: f32| -> f64 { ts.from_px(px) };
    let to_px_y = |v: f64| -> f32 { vs.to_px(v) };

    let xq = to_logical(cx);

    // For now, tooltip is based on the first series (window demo shows one per chart)
    let s = &series_list[0];
    let mut lines: Vec<String> = Vec::new();

    let title = if let Some(_) = detect_time_like(x_axis.min, x_axis.max) {
        format!("x {}", format_time_tick(xq, x_axis.min, x_axis.max))
    } else {
        format!("x {}", format_tick(xq, x_axis.min, x_axis.max))
    };
    lines.push(title);

    match s.series_type {
        SeriesType::Line | SeriesType::Histogram | SeriesType::Baseline => {
            if let Some((_, &(xv, yv))) = s.data_xy.iter().enumerate()
                .min_by(|a, b| (a.1 .0 - xq).abs().partial_cmp(&(b.1 .0 - xq).abs()).unwrap_or(std::cmp::Ordering::Equal))
            {
                lines.push(format!("y {}", format_tick(yv, y_axis.min, y_axis.max)));
                let ypx = to_px_y(yv);
                // Small marker to highlight nearest point
                let mut p = skia::Paint::default();
                p.set_anti_alias(true);
                p.set_style(skia::paint::Style::Fill);
                p.set_color(opts.theme.line_stroke);
                canvas.draw_circle((ts.to_px(xv), ypx), 3.0, &p);
            }
        }
        SeriesType::Candlestick | SeriesType::Bar => {
            if let Some(c) = s.data_ohlc.iter()
                .min_by(|a, b| (a.t - xq).abs().partial_cmp(&(b.t - xq).abs()).unwrap_or(std::cmp::Ordering::Equal))
            {
                lines.push(format!("O {}", format_tick(c.o, y_axis.min, y_axis.max)));
                lines.push(format!("H {}", format_tick(c.h, y_axis.min, y_axis.max)));
                lines.push(format!("L {}", format_tick(c.l, y_axis.min, y_axis.max)));
                lines.push(format!("C {}", format_tick(c.c, y_axis.min, y_axis.max)));
            }
        }
    }

    // Compose tooltip box near cursor
    let mut paint_text = skia::Paint::default();
    paint_text.set_color(opts.theme.axis_label);
    paint_text.set_anti_alias(true);
    let text_size = 12.0 * opts.dpr.max(0.5);
    let shaper = TextShaper::new();

    let padding = 6.0_f32 * opts.dpr.max(0.5);
    let mut w = 0f32;
    let mut h = padding; // top padding
    for line in &lines {
        let adv = shaper.measure_width(line, text_size, true);
        w = w.max(adv);
        h += 14.0 * opts.dpr; // approx line height scaled
    }
    w += padding * 2.0; h += padding; // bottom padding

    // Position tooltip to avoid clipping
    let mut bx = cx + 10.0 * opts.dpr;
    let mut by = cy + 10.0 * opts.dpr;
    if bx + w > r as f32 - 2.0 { bx = (cx - 10.0 * opts.dpr - w).max(l as f32 + 2.0); }
    if by + h > b as f32 - 2.0 { by = (cy - 10.0 * opts.dpr - h).max(t as f32 + 2.0); }

    // Background box
    let rect = skia::Rect::from_xywh(bx, by, w, h);
    let mut bg = skia::Paint::default();
    // Slightly translucent contrasting background (based on theme name)
    let dark = opts.theme.name == "dark";
    let bg_col = if dark { skia::Color::from_argb(200, 32, 32, 36) } else { skia::Color::from_argb(220, 240, 240, 244) };
    bg.set_color(bg_col);
    bg.set_anti_alias(true);
    canvas.draw_rect(rect, &bg);

    // Border
    let mut border = skia::Paint::default();
    border.set_style(skia::paint::Style::Stroke);
    border.set_color(opts.theme.axis_line);
    border.set_stroke_width(1.0);
    canvas.draw_rect(rect, &border);

    // Text lines
    let mut y = by + padding + 12.0 * opts.dpr;
    for line in &lines {
        shaper.draw_left(canvas, line, bx + padding, y, text_size, opts.theme.axis_label, true);
        y += 14.0 * opts.dpr;
    }
}
