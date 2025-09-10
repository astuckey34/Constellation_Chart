// File: crates/window-demo/src/main.rs
// Windowed demo: shows chart-core in a window with crosshair, pan, and zoom.

use chart_core::{Axis, Chart, RenderOptions, Series, ViewState, Theme};
use chart_core::series::{Candle, SeriesType};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use winit::event::{Event, MouseButton, WindowEvent, ElementState, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

#[cfg(feature = "gpu-gl-demo")]
pub mod gpu_gl_demo;

fn main() {
    #[cfg(feature = "gpu-gl-demo")]
    if std::env::args().any(|a| a == "--gpu") {
        // Run the GPU GL demo path (experimental)
        match crate::gpu_gl_demo::run() {
            Ok(()) => return,
            Err(e) => {
                eprintln!("[gpu] failed to start GPU demo: {e}");
                // Fall back to CPU path below
            }
        }
    }

    // Arg: CSV path (supports .csv/.cvs swap). Ignore option-like args (e.g., --gpu)
    let raw = std::env::args().nth(1).filter(|a| !a.starts_with('-'))
        .unwrap_or_else(|| "CRVUSDT_6h.csv".to_string());
    let (mut path, _used_alt) = resolve_path_simple(&raw);
    if !path.exists() {
        // Fallback to known sample files
        for cand in ["CRVUSDT_6h.csv", "BTCUSDT_1m_100.csv", "ETHUSDT_1m_500.csv"] {
            let p = std::path::PathBuf::from(cand);
            if p.exists() { path = p; break; }
        }
    }

    // Load data
    let candles = load_ohlc_csv(&path);
    if candles.is_empty() {
        eprintln!("No candles loaded. Provide a CSV path or place a sample like CRVUSDT_6h.csv in the project root.");
        return;
    }

    // Prepare charts for multiple series types with optional downsampling
    let mut downsample = true; // toggle with 'D'
    let mut charts = build_charts(&candles, downsample, 1024);

    // Window + softbuffer
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Constellation Chart - Candlesticks")
        .with_inner_size(winit::dpi::LogicalSize::new(1024.0, 640.0))
        .build(&event_loop)
        .expect("build window");

    let context = unsafe { softbuffer::Context::new(&window) }.expect("softbuffer context");
    let mut surface = unsafe { softbuffer::Surface::new(&context, &window) }.expect("softbuffer surface");

    // State
    let mut idx: usize = 0; // which chart
    let mut size = window.inner_size();
    let cursor_pos: Arc<Mutex<Option<(f64, f64)>>> = Arc::new(Mutex::new(None));
    let cursor_pos_draw = Arc::clone(&cursor_pos);
    let view0 = ViewState::from_chart(&charts[idx]);
    let view: Arc<Mutex<ViewState>> = Arc::new(Mutex::new(view0));
    let view_draw = Arc::clone(&view);
    let mut dragging = false;
    let themes: Arc<Vec<Theme>> = Arc::new(chart_core::theme::presets());
    let theme_idx: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
    let theme_idx_draw = Arc::clone(&theme_idx);
    let themes_draw = Arc::clone(&themes);

    // Drawing closure
    let dpr: Arc<Mutex<f32>> = Arc::new(Mutex::new(window.scale_factor() as f32));
    let dpr_draw = Arc::clone(&dpr);
    let mut draw = move |charts: &mut [Chart]| {
        let w = size.width.max(1);
        let h = size.height.max(1);
        surface
            .resize(NonZeroU32::new(w).unwrap(), NonZeroU32::new(h).unwrap())
            .ok();

        let mut opts = RenderOptions::default();
        opts.width = w as i32;
        opts.height = h as i32;
        opts.dpr = *dpr_draw.lock().unwrap();
        opts.draw_labels = true;
        opts.show_tooltip = true;
        if let Some((cx, cy)) = *cursor_pos_draw.lock().unwrap() {
            opts.crosshair = Some((cx as f32, cy as f32));
        } else {
            opts.crosshair = None;
        }
        // Theme selection
        let idx_theme = *theme_idx_draw.lock().unwrap();
        let t = themes_draw.get(idx_theme % themes_draw.len()).copied().unwrap_or(Theme::dark());
        opts.theme = t;

        // Apply current view to active chart
        let v = *view_draw.lock().unwrap();
        {
            let ch = &mut charts[idx];
            v.apply_to_chart(ch);
        }

        // Render and blit
        let (rgba, _, _, _) = charts[idx]
            .render_to_rgba8(&opts)
            .expect("render rgba");
        let mut frame = surface.buffer_mut().expect("frame");
        let max_px = frame.len().min(rgba.len() / 4);
        for (i, px) in rgba.chunks_exact(4).take(max_px).enumerate() {
            let r = px[0] as u32;
            let g = px[1] as u32;
            let b = px[2] as u32;
            let a = px[3] as u32;
            frame[i] = (a << 24) | (r << 16) | (g << 8) | b; // ARGB
        }

        // Crosshair drawn by chart-core using opts.crosshair

        if let Err(e) = frame.present() {
            eprintln!("present error: {e:?}");
        }
    };

    // Event loop
    let control_flow = ControlFlow::Wait;
    event_loop.run(move |event, _, cf| {
        *cf = control_flow;
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *cf = ControlFlow::Exit;
                }
                WindowEvent::Resized(new_size) => {
                    size = new_size;
                    if downsample {
                        charts = build_charts(&candles, downsample, size.width as usize);
                        *view.lock().unwrap() = ViewState::from_chart(&charts[idx]);
                    }
                    *dpr.lock().unwrap() = window.scale_factor() as f32;
                }
                WindowEvent::CursorMoved { position, .. } => {
                    *cursor_pos.lock().unwrap() = Some((position.x, position.y));
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    if button == MouseButton::Left {
                        dragging = state == winit::event::ElementState::Pressed;
                    }
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    if let Some((cx, cy)) = *cursor_pos.lock().unwrap() {
                        // Map cursor to world and zoom around it
                        let insets = RenderOptions::default().insets;
                        let w = size.width as i32;
                        let h = size.height as i32;
                        let scroll = match delta {
                            winit::event::MouseScrollDelta::LineDelta(_, y) => y as f64 * 0.1,
                            winit::event::MouseScrollDelta::PixelDelta(p) => (p.y as f64) / 240.0,
                        };
                        let mut vmut = view.lock().unwrap();
                        vmut.zoom_at_pixel(scroll, cx, cy, w, h, &insets);
                    }
                }
                WindowEvent::KeyboardInput { input, .. } => {
                    if input.state != ElementState::Pressed {
                        return;
                    }
                    let set_to = match input.virtual_keycode {
                        Some(VirtualKeyCode::Key1) | Some(VirtualKeyCode::Numpad1) => Some(0),
                        Some(VirtualKeyCode::Key2) | Some(VirtualKeyCode::Numpad2) => Some(1),
                        Some(VirtualKeyCode::Key3) | Some(VirtualKeyCode::Numpad3) => Some(2),
                        Some(VirtualKeyCode::Key4) | Some(VirtualKeyCode::Numpad4) => Some(3),
                        // Autoscale: A = full extents both axes; Y = autoscale Y on visible X range
                        Some(VirtualKeyCode::A) => {
                            *view.lock().unwrap() = ViewState::from_chart(&charts[idx]);
                            let ti = *theme_idx.lock().unwrap();
                            window.set_title(&format!(
                                "Constellation Chart - {} | {}{}",
                                series_title(idx),
                                themes.get(ti % themes.len()).map(|t| t.name).unwrap_or("dark"),
                                if downsample { " | DS:on" } else { " | DS:off" }
                            ));
                            None
                        }
                        Some(VirtualKeyCode::L) => {
                            // Toggle Y-axis scale (Linear <-> Log10)
                            use chart_core::axis::ScaleKind;
                            let ch = &mut charts[idx];
                            ch.y_axis.kind = if ch.y_axis.kind == ScaleKind::Linear { ScaleKind::Log10 } else { ScaleKind::Linear };
                            // Ensure positive range for log
                            if ch.y_axis.kind == ScaleKind::Log10 {
                                if ch.y_axis.min <= 0.0 { ch.y_axis.min = 1e-6; }
                            }
                            *view.lock().unwrap() = ViewState::from_chart(&charts[idx]);
                            None
                        }
                        Some(VirtualKeyCode::Y) => {
                            let mut vmut = view.lock().unwrap();
                            vmut.autoscale_y_visible(&charts[idx]);
                            None
                        }
                        Some(VirtualKeyCode::D) => {
                            // Toggle downsampling and rebuild to match current width
                            downsample = !downsample;
                            charts = build_charts(&candles, downsample, size.width as usize);
                            *view.lock().unwrap() = ViewState::from_chart(&charts[idx]);
                            let ti = *theme_idx.lock().unwrap();
                            window.set_title(&format!(
                                "Constellation Chart - {} | {}{}",
                                series_title(idx),
                                themes.get(ti % themes.len()).map(|t| t.name).unwrap_or("dark"),
                                if downsample { " | DS:on" } else { " | DS:off" }
                            ));
                            None
                        }
                        Some(VirtualKeyCode::T) => {
                            let mut ti = theme_idx.lock().unwrap();
                            *ti = (*ti + 1) % themes.len();
                            window.set_title(&format!(
                                "Constellation Chart - {} | {}{}",
                                series_title(idx),
                                themes.get(*ti % themes.len()).map(|t| t.name).unwrap_or("dark"),
                                if downsample { " | DS:on" } else { " | DS:off" }
                            ));
                            None
                        }
                        Some(VirtualKeyCode::Escape) => {
                            *cf = ControlFlow::Exit;
                            None
                        }
                        _ => None,
                    };
                    if let Some(new_idx) = set_to {
                        if new_idx < charts.len() {
                            idx = new_idx;
                            *view.lock().unwrap() = ViewState::from_chart(&charts[idx]);
                            let ti = *theme_idx.lock().unwrap();
                            window.set_title(&format!(
                                "Constellation Chart - {} | {}{}",
                                series_title(idx),
                                themes.get(ti % themes.len()).map(|t| t.name).unwrap_or("dark"),
                                if downsample { " | DS:on" } else { " | DS:off" }
                            ));
                        }
                    }
                }
                _ => {}
            },
            Event::DeviceEvent {
                event: winit::event::DeviceEvent::MouseMotion { delta },
                ..
            } => {
                if dragging {
                    let (dx, dy) = delta;
                    let insets = RenderOptions::default().insets;
                    let w = size.width as i32;
                    let h = size.height as i32;
                    view.lock().unwrap().pan_by_pixels(dx as f64, dy as f64, w, h, &insets);
                }
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                draw(&mut charts);
            }
            _ => {}
        }
    });
}

fn series_title(idx: usize) -> &'static str {
    match idx {
        0 => "Candlesticks",
        1 => "Bars",
        2 => "Histogram",
        3 => "Baseline",
        _ => "Series",
    }
}

fn build_charts(candles: &[Candle], enable_downsample: bool, target_width_px: usize) -> Vec<Chart> {
    let n = candles.len();
    let (min_p, max_p) = minmax_price(candles);
    let insets = RenderOptions::default().insets;
    let plot_w = target_width_px.saturating_sub((insets.left + insets.right) as usize).max(400);
    let target_points = plot_w; // approx 1 point per pixel
    let bucket = if enable_downsample && n > target_points { ((n as f64) / (target_points as f64)).ceil() as usize } else { 1 };

    // 1) Candles
    let mut c1 = Chart::new();
    c1.x_axis = Axis::new("Time", 0.0, (n - 1) as f64);
    c1.y_axis = Axis::new("Price", min_p, max_p * 1.02);
    c1.add_series(Series::from_candles(candles.to_vec()).aggregate_ohlc(bucket));

    // 2) Bars
    let mut c2 = Chart::new();
    c2.x_axis = Axis::new("Time", 0.0, (n - 1) as f64);
    c2.y_axis = Axis::new("Price", min_p, max_p * 1.02);
    c2.add_series(Series::from_candles_as(SeriesType::Bar, candles.to_vec()).aggregate_ohlc(bucket));

    // 3) Histogram of close-open
    let xy_diff_full: Vec<(f64, f64)> = candles
        .iter()
        .enumerate()
        .map(|(i, c)| (i as f64, c.c - c.o))
        .collect();
    let xy_diff: Vec<(f64, f64)> = if enable_downsample && n > target_points { chart_core::lttb(&xy_diff_full, target_points) } else { xy_diff_full };
    let (min_h, max_h) = minmax_xy(&xy_diff);
    let mut c3 = Chart::new();
    c3.x_axis = Axis::new("Index", 0.0, (n - 1) as f64);
    c3.y_axis = Axis::new("Delta Close-Open", min_h.min(0.0), max_h.max(0.0));
    c3.add_series(Series::with_data(SeriesType::Histogram, xy_diff).with_baseline(0.0));

    // 4) Baseline of closes vs average
    let xy_close_full: Vec<(f64, f64)> = candles
        .iter()
        .enumerate()
        .map(|(i, c)| (i as f64, c.c))
        .collect();
    let xy_close: Vec<(f64, f64)> = if enable_downsample && n > target_points { chart_core::lttb(&xy_close_full, target_points) } else { xy_close_full };
    let avg_close = candles.iter().map(|c| c.c).sum::<f64>() / (n as f64);
    let (min_c, max_c) = minmax_xy(&xy_close);
    let mut c4 = Chart::new();
    c4.x_axis = Axis::new("Index", 0.0, (n - 1) as f64);
    c4.y_axis = Axis::new("Close", min_c, max_c);
    c4.add_series(Series::with_data(SeriesType::Baseline, xy_close).with_baseline(avg_close));

    vec![c1, c2, c3, c4]
}

// visible_y_range is now in chart_core::view (via ViewState::autoscale_y_visible)

fn resolve_path_simple(raw: &str) -> (PathBuf, bool) {
    let p = Path::new(raw);
    if p.exists() {
        return (p.to_path_buf(), false);
    }
    if let Some(alt) = swap_ext(p) {
        if alt.exists() {
            return (alt, true);
        }
    }
    (p.to_path_buf(), false)
}

fn swap_ext(p: &Path) -> Option<std::path::PathBuf> {
    let mut alt = p.to_path_buf();
    let ext = p.extension()?.to_string_lossy().to_lowercase();
    match ext.as_str() {
        "cvs" => {
            alt.set_extension("csv");
            Some(alt)
        }
        "csv" => {
            alt.set_extension("cvs");
            Some(alt)
        }
        _ => None,
    }
}

fn load_ohlc_csv(path: &Path) -> Vec<Candle> {
    let mut rdr = match csv::ReaderBuilder::new().has_headers(true).from_path(path) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let headers = match rdr.headers() {
        Ok(h) => h.iter().map(|h| h.to_lowercase()).collect::<Vec<_>>(),
        Err(_) => return Vec::new(),
    };
    let idx = |names: &[&str]| -> Option<usize> {
        for (i, h) in headers.iter().enumerate() {
            for want in names {
                if h == want {
                    return Some(i);
                }
            }
        }
        None
    };
    let i_time = idx(&["time", "timestamp", "open_time", "date", "datetime"]);
    let i_open = idx(&["open", "o"]);
    let i_high = idx(&["high", "h"]);
    let i_low = idx(&["low", "l"]);
    let i_close = idx(&["close", "c", "adj_close", "close_price"]);

    let mut out = Vec::new();
    let mut row_index = 0_f64;
    for rec in rdr.records() {
        let rec = rec.expect("record");
        let parse = |i: Option<usize>| -> Option<f64> {
            i.and_then(|ix| rec.get(ix))
                .and_then(|s| s.trim().parse::<f64>().ok())
        };
        let t = if let Some(ix) = i_time {
            rec.get(ix)
                .and_then(parse_time_to_f64)
                .unwrap_or_else(|| {
                    let v = row_index;
                    row_index += 1.0;
                    v
                })
        } else {
            let v = row_index;
            row_index += 1.0;
            v
        };
        let (o, h, l, c) = (parse(i_open), parse(i_high), parse(i_low), parse(i_close));
        if let (Some(o), Some(h), Some(l), Some(c)) = (o, h, l, c) {
            out.push(Candle { t, o, h, l, c });
        }
    }
    out
}

fn parse_time_to_f64(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Ok(n) = s.parse::<i64>() {
        if n > 1_000_000_000_000 {
            return Some(n as f64 / 1000.0);
        } // epoch ms -> sec
        if n > 1_000_000_000 {
            return Some(n as f64);
        } // epoch sec
        return Some(n as f64);
    }
    None
}

fn minmax_price(c: &[Candle]) -> (f64, f64) {
    let mut min_p = f64::INFINITY;
    let mut max_p = f64::NEG_INFINITY;
    for k in c {
        min_p = min_p.min(k.l);
        max_p = max_p.max(k.h);
    }
    (min_p, max_p)
}

fn minmax_xy(v: &[(f64, f64)]) -> (f64, f64) {
    let mut min_v = f64::INFINITY;
    let mut max_v = f64::NEG_INFINITY;
    for &(_, y) in v {
        min_v = min_v.min(y);
        max_v = max_v.max(y);
    }
    (min_v, max_v)
}

// compute_extents replaced by ViewState::from_chart
