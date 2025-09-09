// File: crates/window-demo/src/main.rs
// Summary: Minimal windowed demo that renders chart-core to a window via RGBA blit (CPU) using winit + softbuffer.

use chart_core::{Axis, Chart, RenderOptions, Series};
use chart_core::series::{Candle, SeriesType};
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};
use std::path::{Path, PathBuf};
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

fn main() {
    // Arg: CSV path (supports .csv/.cvs swap)
    let raw = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "binanceus_CRVUSDT_6h_2023-09-13_to_2025-01-21.cvs".to_string());
    let (path, _used_alt) = resolve_path_simple(&raw);

    // Load data
    let candles = load_ohlc_csv(&path);
    if candles.is_empty() { eprintln!("no candles loaded"); return; }

    // Prepare charts to showcase multiple series types
    let mut charts = build_charts(&candles);

    // Window + softbuffer setup
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Constellation Chart — Window Demo")
        .with_inner_size(winit::dpi::LogicalSize::new(1024.0, 640.0))
        .build(&event_loop)
        .expect("build window");

    let context = unsafe { softbuffer::Context::new(&window) }.expect("softbuffer context");
    let mut surface = unsafe { softbuffer::Surface::new(&context, &window) }.expect("softbuffer surface");

    // State: which chart to display
    let mut idx = 0usize;
    let mut size = window.inner_size();
    let mut dragging = false;
    let mut last_drag_pos: Option<(f64, f64)> = None;

    // View state (world ranges) per active chart
    let view = compute_extents(&charts[idx]);
    let view_share = Arc::new(Mutex::new(view));
    let cursor_share: Arc<Mutex<Option<(f64, f64)>>> = Arc::new(Mutex::new(None));

    let mut draw = move |charts: &mut Vec<Chart>| {
        let w = size.width.max(1);
        let h = size.height.max(1);
        surface.resize(NonZeroU32::new(w).unwrap(), NonZeroU32::new(h).unwrap()).ok();

        let mut opts = RenderOptions::default();
        opts.width = w as i32;
        opts.height = h as i32;
        // Labels on for demo
        opts.draw_labels = true;

        // Apply current view to the active chart
        let current_view = *view_share.lock().unwrap();
        {
            let ch = &mut charts[idx];
            ch.x_axis.min = current_view.x_min;
            ch.x_axis.max = current_view.x_max;
            ch.y_axis.min = current_view.y_min;
            ch.y_axis.max = current_view.y_max;
        }

        // Render to RGBA and convert to BGRA u32 for softbuffer
        let (rgba, _, _, _) = charts[idx].render_to_rgba8(&opts).expect("render rgba");
        let mut frame = surface.buffer_mut().expect("frame");
        let max_px = frame.len().min(rgba.len() / 4);
        for (i, px) in rgba.chunks_exact(4).take(max_px).enumerate() {
            let r = px[0] as u32;
            let g = px[1] as u32;
            let b = px[2] as u32;
            let a = px[3] as u32;
            // Softbuffer expects ARGB or BGRA depending on platform; BGRA is common.
            frame[i] = (a << 24) | (r << 16) | (g << 8) | b; // ARGB with R in high byte — adjust if needed
        }

        // Draw crosshair overlay if cursor present
        if let Some((cx, cy)) = *cursor_share.lock().unwrap() {
            let ix = cx.round().clamp(0.0, (w as f64 - 1.0)) as i32;
            let iy = cy.round().clamp(0.0, (h as f64 - 1.0)) as i32;
            let color: u32 = (0xFF << 24) | (255 << 16) | (230 << 8) | 70; // ARGB yellow
            // Horizontal
            let row = iy.max(0).min(h as i32 - 1) as usize;
            for x in 0..(w as usize) { frame[row * (w as usize) + x] = color; }
            // Vertical
            let col = ix.max(0).min(w as i32 - 1) as usize;
            for y in 0..(h as usize) { frame[y * (w as usize) + col] = color; }
        }
        if let Err(e) = frame.present() { eprintln!("present error: {e:?}"); }
    };

    let mut control_flow = ControlFlow::Wait;
    // Keep a copy of opts' insets/size for event mapping
    let mut map_width = RenderOptions::default().width;
    let mut map_height = RenderOptions::default().height;
    let insets = RenderOptions::default().insets;

    event_loop.run(move |event, _, cf| {
        *cf = control_flow;
        match event {
            Event::WindowEvent { event, window_id: _ } => match event {
                WindowEvent::CloseRequested => {
                    *cf = ControlFlow::Exit;
                }
                WindowEvent::Resized(new_size) => {
                    size = new_size;
                }
                WindowEvent::CursorMoved { position, .. } => {
                    *cursor_share.lock().unwrap() = Some((position.x, position.y));
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    if button == winit::event::MouseButton::Left {
                        dragging = state == winit::event::ElementState::Pressed;
                        last_drag_pos = None;
                    }
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    // Zoom around cursor position
                    if let Some((cx, cy)) = *cursor_share.lock().unwrap() {
                        // Use current window size and default insets for mapping
                        let w = size.width as i32; let h = size.height as i32;
                        map_width = w; map_height = h;
                        let (l, rpx, t, bpx) = (
                            insets.left as f64,
                            (w - insets.right) as f64,
                            insets.top as f64,
                            (h - insets.bottom) as f64,
                        );
                        let plot_w = (rpx - l).max(1.0);
                        let plot_h = (bpx - t).max(1.0);
                        let cx_clamped = cx.clamp(l, rpx);
                        let cy_clamped = cy.clamp(t, bpx);
                        let v = *view_share.lock().unwrap();
                        let x_span = v.x_max - v.x_min;
                        let y_span = v.y_max - v.y_min;
                        let wx = v.x_min + (cx_clamped - l) / plot_w * x_span;
                        let wy = v.y_max - (cy_clamped - t) / plot_h * y_span;
                        let scroll = match delta {
                            winit::event::MouseScrollDelta::LineDelta(_, y) => y as f64 * 0.1,
                            winit::event::MouseScrollDelta::PixelDelta(p) => (p.y as f64) / 240.0,
                        };
                        let factor = (1.0 - scroll).clamp(0.1, 10.0);
                        // New spans
                        let nx = x_span * factor;
                        let ny = y_span * factor;
                        // Keep wx/wy fixed in view
                        let mut vmut = view_share.lock().unwrap();
                        let rx = (wx - vmut.x_min) / x_span;
                        let ry = (vmut.y_max - wy) / y_span; // from top
                        vmut.x_min = wx - rx * nx;
                        vmut.x_max = vmut.x_min + nx;
                        vmut.y_max = wy + ry * ny;
                        vmut.y_min = vmut.y_max - ny;
                    }
                }
                WindowEvent::KeyboardInput { .. } => {
                    idx = (idx + 1) % charts.len();
                    // Reset view to new chart extents
                    *view_share.lock().unwrap() = compute_extents(&charts[idx]);
                }
                _ => {}
            },
            Event::DeviceEvent { event: winit::event::DeviceEvent::MouseMotion { delta }, .. } => {
                if dragging {
                    // Pan by delta pixels mapped to world
                    let (dx, dy) = delta;
                    let w = size.width as i32; let h = size.height as i32;
                    let plot_w = ((w - insets.right - insets.left) as f64).max(1.0);
                    let plot_h = ((h - insets.bottom - insets.top) as f64).max(1.0);
                    let mut v = view_share.lock().unwrap();
                    let x_span = v.x_max - v.x_min;
                    let y_span = v.y_max - v.y_min;
                    let wx = -dx as f64 / plot_w * x_span;
                    let wy = dy as f64 / plot_h * y_span; // screen y down -> world up
                    v.x_min += wx; v.x_max += wx;
                    v.y_min += wy; v.y_max += wy;
                }
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::RedrawRequested(_) => { draw(&mut charts); }
            _ => {}
        }
    });
}

fn build_charts(candles: &[Candle]) -> Vec<Chart> {
    let n = candles.len();
    let (min_p, max_p) = minmax_price(candles);

    // 1) Candles
    let mut c1 = Chart::new();
    c1.x_axis = Axis::new("Time", 0.0, (n - 1) as f64);
    c1.y_axis = Axis::new("Price", min_p, max_p * 1.02);
    c1.add_series(Series::from_candles(candles.to_vec()));

    // 2) Bars
    let mut c2 = Chart::new();
    c2.x_axis = Axis::new("Time", 0.0, (n - 1) as f64);
    c2.y_axis = Axis::new("Price", min_p, max_p * 1.02);
    c2.add_series(Series::from_candles_as(SeriesType::Bar, candles.to_vec()));

    // 3) Histogram of close-open
    let xy_diff: Vec<(f64, f64)> = candles
        .iter()
        .enumerate()
        .map(|(i, c)| (i as f64, c.c - c.o))
        .collect();
    let (min_h, max_h) = minmax_xy(&xy_diff);
    let mut c3 = Chart::new();
    c3.x_axis = Axis::new("Index", 0.0, (n - 1) as f64);
    c3.y_axis = Axis::new(" Delta Close-Open\, min_h.min(0.0), max_h.max(0.0));
    c3.add_series(Series::with_data(SeriesType::Histogram, xy_diff).with_baseline(0.0));

    // 4) Baseline of closes vs average
    let xy_close: Vec<(f64, f64)> = candles
        .iter()
        .enumerate()
        .map(|(i, c)| (i as f64, c.c))
        .collect();
    let avg_close = candles.iter().map(|c| c.c).sum::<f64>() / (n as f64);
    let (min_c, max_c) = minmax_xy(&xy_close);
    let mut c4 = Chart::new();
    c4.x_axis = Axis::new("Index", 0.0, (n - 1) as f64);
    c4.y_axis = Axis::new("Close", min_c, max_c);
    c4.add_series(Series::with_data(SeriesType::Baseline, xy_close).with_baseline(avg_close));

    vec![c1, c2, c3, c4]
}

#[derive(Clone, Copy)]
struct View { x_min: f64, x_max: f64, y_min: f64, y_max: f64 }

fn compute_extents(chart: &Chart) -> View {
    let mut x_min = f64::INFINITY;
    let mut x_max = f64::NEG_INFINITY;
    let mut y_min = f64::INFINITY;
    let mut y_max = f64::NEG_INFINITY;
    for s in &chart.series {
        match s.series_type {
            SeriesType::Line | SeriesType::Histogram | SeriesType::Baseline => {
                for &(x, y) in &s.data_xy {
                    x_min = x_min.min(x); x_max = x_max.max(x);
                    y_min = y_min.min(y); y_max = y_max.max(y);
                }
                if let Some(b) = s.baseline { y_min = y_min.min(b); y_max = y_max.max(b); }
            }
            SeriesType::Candlestick | SeriesType::Bar => {
                for c in &s.data_ohlc {
                    x_min = x_min.min(c.t); x_max = x_max.max(c.t);
                    y_min = y_min.min(c.l); y_max = y_max.max(c.h);
                }
            }
        }
    }
    if !x_min.is_finite() || !x_max.is_finite() || !y_min.is_finite() || !y_max.is_finite() {
        return View { x_min: 0.0, x_max: 1.0, y_min: 0.0, y_max: 1.0 };
    }
    if (x_max - x_min).abs() < 1e-9 { x_max = x_min + 1.0; }
    if (y_max - y_min).abs() < 1e-9 { y_max = y_min + 1.0; }
    // add small margin
    let ym = (y_max - y_min) * 0.02;
    View { x_min, x_max, y_min: y_min - ym, y_max: y_max + ym }
}

/// Resolve path, trying .csv/.cvs swap if needed.
fn resolve_path_simple(raw: &str) -> (PathBuf, bool) {
    let p = Path::new(raw);
    if p.exists() { return (p.to_path_buf(), false); }
    if let Some(alt) = swap_ext(p) {
        if alt.exists() { return (alt, true); }
    }
    (p.to_path_buf(), false)
}

fn swap_ext(p: &Path) -> Option<std::path::PathBuf> {
    let mut alt = p.to_path_buf();
    let ext = p.extension()?.to_string_lossy().to_lowercase();
    match ext.as_str() { "cvs" => { alt.set_extension("csv"); Some(alt) }, "csv" => { alt.set_extension("cvs"); Some(alt) }, _ => None }
}

fn load_ohlc_csv(path: &Path) -> Vec<Candle> {
    let mut rdr = csv::ReaderBuilder::new().has_headers(true).from_path(path).expect("open csv");
    let headers = rdr.headers().expect("headers").iter().map(|h| h.to_lowercase()).collect::<Vec<_>>();
    let idx = |names: &[&str]| -> Option<usize> {
        for (i, h) in headers.iter().enumerate() { for want in names { if h == want { return Some(i); } } } None
    };
    let i_time = idx(&["time","timestamp","open_time","date","datetime"]);
    let i_open = idx(&["open","o"]);
    let i_high = idx(&["high","h"]);
    let i_low  = idx(&["low","l"]);
    let i_close= idx(&["close","c","adj_close","close_price"]);

    let mut out = Vec::new();
    let mut row_index = 0_f64;
    for rec in rdr.records() {
        let rec = rec.expect("record");
        let parse = |i: Option<usize>| -> Option<f64> { i.and_then(|ix| rec.get(ix)).and_then(|s| s.trim().parse::<f64>().ok()) };
        let t = if let Some(ix) = i_time { rec.get(ix).and_then(parse_time_to_f64).unwrap_or_else(|| { let v=row_index; row_index+=1.0; v }) } else { let v=row_index; row_index+=1.0; v };
        let (o, h, l, c) = (parse(i_open), parse(i_high), parse(i_low), parse(i_close));
        if let (Some(o), Some(h), Some(l), Some(c)) = (o, h, l, c) { out.push(Candle { t, o, h, l, c }); }
    }
    out
}

fn parse_time_to_f64(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() { return None; }
    if let Ok(n) = s.parse::<i64>() {
        if n > 10_i64.pow(12) { return Some(n as f64 / 1000.0); }
        if n > 10_i64.pow(9)  { return Some(n as f64); }
        return Some(n as f64);
    }
    None
}

fn minmax_price(c: &[Candle]) -> (f64, f64) {
    let mut min_p = f64::INFINITY;
    let mut max_p = f64::NEG_INFINITY;
    for k in c { min_p = min_p.min(k.l); max_p = max_p.max(k.h); }
    (min_p, max_p)
}

fn minmax_xy(v: &[(f64, f64)]) -> (f64, f64) {
    let mut min_v = f64::INFINITY; let mut max_v = f64::NEG_INFINITY;
    for &(_, y) in v { min_v = min_v.min(y); max_v = max_v.max(y); }
    (min_v, max_v)
}
