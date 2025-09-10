#![cfg(feature = "gpu-gl-demo")]

// Minimal, feature-gated GPU demo scaffolding using glutin + skia-safe GL interface.
// Note: This is a stub entry; full rendering integration to replace CPU path will follow.

use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextApi, ContextAttributesBuilder, Version};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::{SurfaceAttributesBuilder, WindowSurface, Surface};
use raw_window_handle::HasRawWindowHandle;
use winit::{dpi::LogicalSize, event::Event, event_loop::EventLoop, window::WindowBuilder};
use std::num::NonZeroU32;

use chart_render_skia::SkiaRenderer;
use chart_core::{Chart, Axis, Series, RenderOptions, SeriesType, ViewState, Theme};
use chart_core::series::Candle;
use chart_core::lttb;
use csv;

pub fn run() -> Result<(), String> {
    // Event loop
    let event_loop = EventLoop::new();

    // GL display & config
    let wb = glutin_winit::DisplayBuilder::new();
    let template = ConfigTemplateBuilder::new();
    let (maybe_window, gl_config) = wb
        .with_window_builder(Some(WindowBuilder::new().with_title("Constellation Chart - GPU (GL stub)").with_inner_size(LogicalSize::new(1024.0, 640.0))))
        .build(&event_loop, template, |mut configs| {
            // Pick first for now
            configs.next().unwrap()
        })
        .map_err(|e| e.to_string())?;
    let window = maybe_window.expect("failed to create winit window");

    let raw_handle = window.raw_window_handle();
    let gl_display = gl_config.display();

    // GL context
    let context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
        .build(Some(raw_handle));
    let not_current = unsafe { gl_display.create_context(&gl_config, &context_attributes) }
        .map_err(|e| format!("create_context: {e}"))?;

    // Surface
    let size = window.inner_size();
    let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
        window.raw_window_handle(),
        NonZeroU32::new(size.width.max(1)).unwrap(),
        NonZeroU32::new(size.height.max(1)).unwrap(),
    );
    let gl_surface: Surface<WindowSurface> = unsafe { gl_display.create_window_surface(&gl_config, &attrs) }
        .map_err(|e| format!("create_window_surface: {e}"))?;
    let gl_context = not_current.make_current(&gl_surface).map_err(|e| e.to_string())?;

    // Load GL and create Skia interface
    let interface = skia_safe::gpu::gl::Interface::new_load_with(|s| gl_display.get_proc_address(&std::ffi::CString::new(s).unwrap()) as _)
        .ok_or_else(|| "Skia GL Interface creation failed".to_string())?;

    let mut direct = skia_safe::gpu::direct_contexts::make_gl(interface, None)
        .ok_or_else(|| "Skia DirectContext creation failed".to_string())?;

    // Backend target (framebuffer 0)
    let fb_info = skia_safe::gpu::gl::FramebufferInfo { fboid: 0, format: skia_safe::gpu::gl::Format::RGBA8.into(), protected: skia_safe::gpu::Protected::No };
    let size = window.inner_size();
    let backend_rt = skia_safe::gpu::backend_render_targets::make_gl(
        (size.width as i32, size.height as i32),
        None,
        8,
        fb_info,
    );

    let mut surface = skia_safe::gpu::surfaces::wrap_backend_render_target(
        &mut direct,
        &backend_rt,
        skia_safe::gpu::SurfaceOrigin::BottomLeft,
        skia_safe::ColorType::RGBA8888,
        None,
        None,
    )
    .ok_or_else(|| "Skia GPU surface creation failed".to_string())?;

    let _renderer = SkiaRenderer::new();

    // Load CSV OHLC similar to CPU demo; ignore option-like args
    let raw = std::env::args().nth(1).filter(|a| !a.starts_with('-'))
        .unwrap_or_else(|| "CRVUSDT_6h.csv".to_string());
    let (mut path, _used_alt) = resolve_path_simple(&raw);
    if !path.exists() {
        for cand in ["CRVUSDT_6h.csv", "BTCUSDT_1m_100.csv", "ETHUSDT_1m_500.csv"] {
            let p = std::path::PathBuf::from(cand);
            if p.exists() { path = p; break; }
        }
    }
    let candles = load_ohlc_csv(&path);
    if candles.is_empty() {
        return Err("no candles loaded: provide a CSV path or place a sample like CRVUSDT_6h.csv in project root".to_string());
    }

    // Prepare charts and simple view state
    let mut downsample = true;
    let mut charts = build_charts(&candles, downsample, window.inner_size().width as usize);
    let mut idx: usize = 0;
    let mut view = ViewState::from_chart(&charts[idx]);
    let mut opts = RenderOptions::default();
    opts.draw_labels = true;
    opts.show_tooltip = true;
    opts.dpr = window.scale_factor() as f32;
    let themes = chart_core::theme::presets();
    let mut theme_idx: usize = 0;
    let mut cursor_pos: Option<(f64, f64)> = None;
    let mut dragging = false;

    // Initial title with theme + downsampling status
    window.set_title(&format!(
        "Constellation Chart - {} | {}{}",
        series_title(idx),
        themes.get(theme_idx % themes.len()).map(|t| t.name).unwrap_or("dark"),
        if downsample { " | DS:on" } else { " | DS:off" }
    ));

    event_loop.run(move |event, _, control_flow| {
        *control_flow = winit::event_loop::ControlFlow::Poll;
        match event {
            Event::RedrawRequested(_) | Event::MainEventsCleared => {
                let size = window.inner_size();
                opts.width = size.width as i32;
                opts.height = size.height as i32;
                opts.dpr = window.scale_factor() as f32;

                let canvas = surface.canvas();
                let ch = &mut charts[idx];
                view.apply_to_chart(ch);
                // Crosshair + theme
                if let Some((cx, cy)) = cursor_pos { opts.crosshair = Some((cx as f32, cy as f32)); } else { opts.crosshair = None; }
                opts.theme = themes.get(theme_idx % themes.len()).copied().unwrap_or(Theme::dark());
                ch.draw_onto_canvas(canvas, &opts);
                direct.flush_and_submit();
                let _ = gl_surface.swap_buffers(&gl_context);
                window.request_redraw();
            }
            Event::WindowEvent { event, .. } => match event {
                winit::event::WindowEvent::CloseRequested => {
                    *control_flow = winit::event_loop::ControlFlow::Exit;
                }
                winit::event::WindowEvent::CursorMoved { position, .. } => {
                    cursor_pos = Some((position.x, position.y));
                }
                winit::event::WindowEvent::MouseInput { state, button, .. } => {
                    if button == winit::event::MouseButton::Left {
                        dragging = state == winit::event::ElementState::Pressed;
                    }
                }
                winit::event::WindowEvent::MouseWheel { delta, .. } => {
                    if let Some((cx, cy)) = cursor_pos {
                        let insets = RenderOptions::default().insets;
                        let w = window.inner_size().width as i32;
                        let h = window.inner_size().height as i32;
                        let scroll = match delta {
                            winit::event::MouseScrollDelta::LineDelta(_, y) => y as f64 * 0.1,
                            winit::event::MouseScrollDelta::PixelDelta(p) => (p.y as f64) / 240.0,
                        };
                        view.zoom_at_pixel(scroll, cx, cy, w, h, &insets);
                    }
                }
                winit::event::WindowEvent::Resized(new_size) => {
                    // Recreate render target and surface to match new size
                    let fb_info = skia_safe::gpu::gl::FramebufferInfo { fboid: 0, format: skia_safe::gpu::gl::Format::RGBA8.into(), protected: skia_safe::gpu::Protected::No };
                    let backend_rt = skia_safe::gpu::backend_render_targets::make_gl(
                        (new_size.width as i32, new_size.height as i32),
                        None,
                        8,
                        fb_info,
                    );
                    surface = skia_safe::gpu::surfaces::wrap_backend_render_target(
                        &mut direct,
                        &backend_rt,
                        skia_safe::gpu::SurfaceOrigin::BottomLeft,
                        skia_safe::ColorType::RGBA8888,
                        None,
                        None,
                    )
                    .expect("Skia GPU surface recreate failed");
                    if downsample {
                        charts = build_charts(&candles, downsample, new_size.width as usize);
                        view = ViewState::from_chart(&charts[idx]);
                    }
                }
                winit::event::WindowEvent::KeyboardInput { input, .. } => {
                    if input.state != winit::event::ElementState::Pressed { return; }
                    match input.virtual_keycode {
                        Some(winit::event::VirtualKeyCode::Key1) | Some(winit::event::VirtualKeyCode::Numpad1) => { idx = 0; view = ViewState::from_chart(&charts[idx]); }
                        Some(winit::event::VirtualKeyCode::Key2) | Some(winit::event::VirtualKeyCode::Numpad2) => { idx = 1; view = ViewState::from_chart(&charts[idx]); }
                        Some(winit::event::VirtualKeyCode::Key3) | Some(winit::event::VirtualKeyCode::Numpad3) => { idx = 2; view = ViewState::from_chart(&charts[idx]); }
                        Some(winit::event::VirtualKeyCode::Key4) | Some(winit::event::VirtualKeyCode::Numpad4) => { idx = 3; view = ViewState::from_chart(&charts[idx]); }
                        Some(winit::event::VirtualKeyCode::A) => { view = ViewState::from_chart(&charts[idx]); }
                        Some(winit::event::VirtualKeyCode::Y) => { let _ = view.autoscale_y_visible(&charts[idx]); }
                        Some(winit::event::VirtualKeyCode::D) => { downsample = !downsample; charts = build_charts(&candles, downsample, window.inner_size().width as usize); view = ViewState::from_chart(&charts[idx]); window.set_title(&format!("Constellation Chart - {} | {}{}", series_title(idx), themes.get(theme_idx % themes.len()).map(|t| t.name).unwrap_or("dark"), if downsample { " | DS:on" } else { " | DS:off" })); }
                        Some(winit::event::VirtualKeyCode::T) => { theme_idx = (theme_idx + 1) % themes.len(); window.set_title(&format!("Constellation Chart - {} | {}{}", series_title(idx), themes.get(theme_idx % themes.len()).map(|t| t.name).unwrap_or("dark"), if downsample { " | DS:on" } else { " | DS:off" })); }
                        Some(winit::event::VirtualKeyCode::Escape) => { *control_flow = winit::event_loop::ControlFlow::Exit; }
                        _ => {}
                    }
                    // Update title on chart switch as well
                    window.set_title(&format!(
                        "Constellation Chart - {} | {}{}",
                        series_title(idx),
                        themes.get(theme_idx % themes.len()).map(|t| t.name).unwrap_or("dark"),
                        if downsample { " | DS:on" } else { " | DS:off" }
                    ));
                }
                _ => {}
            },
            Event::DeviceEvent { event: winit::event::DeviceEvent::MouseMotion { delta }, .. } => {
                if dragging {
                    let (dx, dy) = delta;
                    let insets = RenderOptions::default().insets;
                    let w = window.inner_size().width as i32;
                    let h = window.inner_size().height as i32;
                    view.pan_by_pixels(dx as f64, dy as f64, w, h, &insets);
                }
            }
            _ => {}
        }
    });
}

fn series_title(idx: usize) -> &'static str {
    match idx { 0 => "Candlesticks", 1 => "Bars", 2 => "Histogram", 3 => "Baseline", _ => "Series" }
}

fn resolve_path_simple(raw: &str) -> (std::path::PathBuf, bool) {
    let p = std::path::Path::new(raw);
    if p.exists() { return (p.to_path_buf(), false); }
    if let Some(alt) = swap_ext(p) { if alt.exists() { return (alt, true); } }
    (p.to_path_buf(), false)
}

fn swap_ext(p: &std::path::Path) -> Option<std::path::PathBuf> {
    let mut alt = p.to_path_buf();
    let ext = p.extension()?.to_string_lossy().to_lowercase();
    match ext.as_str() {
        "cvs" => { alt.set_extension("csv"); Some(alt) }
        "csv" => { alt.set_extension("cvs"); Some(alt) }
        _ => None,
    }
}

fn load_ohlc_csv(path: &std::path::Path) -> Vec<Candle> {
    let mut rdr = match csv::ReaderBuilder::new().has_headers(true).from_path(path) { Ok(r) => r, Err(_) => return Vec::new() };
    let headers = match rdr.headers() { Ok(h) => h.iter().map(|h| h.to_lowercase()).collect::<Vec<_>>(), Err(_) => return Vec::new() };
    let idx = |names: &[&str]| -> Option<usize> { for (i, h) in headers.iter().enumerate() { for want in names { if h == want { return Some(i); } } } None };
    let i_time = idx(&["time", "timestamp", "open_time", "date", "datetime"]);
    let i_open = idx(&["open", "o"]);
    let i_high = idx(&["high", "h"]);
    let i_low = idx(&["low", "l"]);
    let i_close = idx(&["close", "c", "adj_close", "close_price"]);
    let mut out = Vec::new();
    let mut row_index = 0_f64;
    for rec in rdr.records() {
        let rec = if let Ok(r) = rec { r } else { continue };
        let parse = |i: Option<usize>| -> Option<f64> { i.and_then(|ix| rec.get(ix)).and_then(|s| s.trim().parse::<f64>().ok()) };
        let t = if let Some(ix) = i_time { rec.get(ix).and_then(parse_time_to_f64).unwrap_or_else(|| { let v=row_index; row_index+=1.0; v }) } else { let v=row_index; row_index+=1.0; v };
        let (o, h, l, c) = (parse(i_open), parse(i_high), parse(i_low), parse(i_close));
        if let (Some(o), Some(h), Some(l), Some(c)) = (o, h, l, c) { out.push(Candle { t, o, h, l, c }); }
    }
    out
}

fn parse_time_to_f64(s: &str) -> Option<f64> {
    let s = s.trim(); if s.is_empty() { return None; }
    if let Ok(n) = s.parse::<i64>() { if n > 1_000_000_000_000 { return Some(n as f64 / 1000.0); } if n > 1_000_000_000 { return Some(n as f64); } return Some(n as f64); }
    None
}

fn minmax_price(c: &[Candle]) -> (f64, f64) {
    let mut min_p = f64::INFINITY; let mut max_p = f64::NEG_INFINITY;
    for k in c { min_p = min_p.min(k.l); max_p = max_p.max(k.h); }
    (min_p, max_p)
}

fn minmax_xy(v: &[(f64, f64)]) -> (f64, f64) {
    let mut min_v = f64::INFINITY; let mut max_v = f64::NEG_INFINITY;
    for &(_, y) in v { min_v = min_v.min(y); max_v = max_v.max(y); }
    (min_v, max_v)
}

fn build_charts(candles: &[Candle], enable_downsample: bool, target_width_px: usize) -> Vec<Chart> {
    let n = candles.len();
    let (min_p, max_p) = minmax_price(candles);
    let insets = RenderOptions::default().insets;
    let plot_w = target_width_px.saturating_sub((insets.left + insets.right) as usize).max(400);
    let target_points = plot_w;
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
    let xy_diff_full: Vec<(f64, f64)> = candles.iter().enumerate().map(|(i, c)| (i as f64, c.c - c.o)).collect();
    let xy_diff = if enable_downsample && n > target_points { lttb(&xy_diff_full, target_points) } else { xy_diff_full };
    let (min_h, max_h) = minmax_xy(&xy_diff);
    let mut c3 = Chart::new();
    c3.x_axis = Axis::new("Index", 0.0, (n - 1) as f64);
    c3.y_axis = Axis::new("Delta Close-Open", min_h.min(0.0), max_h.max(0.0));
    c3.add_series(Series::with_data(SeriesType::Histogram, xy_diff).with_baseline(0.0));

    // 4) Baseline of closes vs average
    let xy_close_full: Vec<(f64, f64)> = candles.iter().enumerate().map(|(i, c)| (i as f64, c.c)).collect();
    let xy_close = if enable_downsample && n > target_points { lttb(&xy_close_full, target_points) } else { xy_close_full };
    let avg_close = candles.iter().map(|c| c.c).sum::<f64>() / (n as f64);
    let (min_c, max_c) = minmax_xy(&xy_close);
    let mut c4 = Chart::new();
    c4.x_axis = Axis::new("Index", 0.0, (n - 1) as f64);
    c4.y_axis = Axis::new("Close", min_c, max_c);
    c4.add_series(Series::with_data(SeriesType::Baseline, xy_close).with_baseline(avg_close));

    vec![c1, c2, c3, c4]
}
