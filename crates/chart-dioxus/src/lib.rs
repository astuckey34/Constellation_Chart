// File: crates/chart-dioxus/src/lib.rs
// Summary: Dioxus UI scaffolding for a native ChartCanvas component (desktop only).
// Notes:
// - This crate keeps UI deps behind the `desktop` feature, so the workspace builds
//   without fetching Dioxus unless explicitly enabled.
// - The initial scaffold does NOT create a real native drawing surface yet; it wires
//   props, state, and input handlers to prepare for softbuffer/Skia integration.

use chart_core::{Chart, RenderOptions, Series, Theme, ViewState};

#[cfg(feature = "desktop")]
pub mod ui {
    use super::*;
    use dioxus::prelude::*;
    use dioxus_desktop::use_window;
    use base64::Engine as _;

    #[derive(Props, Clone)]
    pub struct ChartCanvasProps {
        pub series: Vec<Series>,
        #[props(default = Theme::dark())]
        pub theme: Theme,
        #[props(default)]
        pub initial_view: Option<ViewState>,
        /// Called when the ViewState changes (pan/zoom/autoscale)
        #[props(default)]
        pub on_view_change: Option<EventHandler<ViewState>>, // optional
        /// Whether to show the tooltip HUD under crosshair
        #[props(default = true)]
        pub show_tooltip: bool,
        /// Initial/rendered width in pixels (temporary until native surface sizing)
        #[props(default = 1024)]
        pub width_px: i32,
        /// Initial/rendered height in pixels
        #[props(default = 640)]
        pub height_px: i32,
    }

    impl PartialEq for ChartCanvasProps {
        fn eq(&self, _other: &Self) -> bool { false }
    }

    /// Minimal ChartCanvas component scaffold. Renders a placeholder container and wires
    /// mouse/keyboard events into a ViewState. A subsequent patch will add a native
    /// softbuffer surface and blit `Chart::render_to_rgba8` into it.
    #[component]
    pub fn ChartCanvas(props: ChartCanvasProps) -> Element {
        // Chart and view state (shared across events)
        let mut chart = use_signal(|| {
            let mut c = Chart::new();
            for s in &props.series { c.add_series(s.clone()); }
            c
        });
        let mut view = use_signal(|| {
            props
                .initial_view
                .unwrap_or_else(|| ViewState::from_chart(&chart.read()))
        });
        let crosshair = use_signal(|| Option::<(f32, f32)>::None);

        // Window size state (placeholder; in the next iteration we’ll read actual node size)
        let width = use_signal(|| props.width_px);
        let height = use_signal(|| props.height_px);

        // Notify on view change helper
        let _notify_view = |v: ViewState| {
            if let Some(cb) = &props.on_view_change { cb.call(v); }
        };

        // Interaction wiring will be added in the native-surface iteration.

        // PNG data-URL buffer for fallback rendering
        let img_src = use_signal(|| Option::<String>::None);

        let mut render_now = {
            to_owned![chart, view, crosshair, width, height, img_src];
            move || {
                let mut opts = RenderOptions::default();
                opts.width = *width.read();
                opts.height = *height.read();
                opts.dpr = 1.0;
                opts.draw_labels = true;
                opts.show_tooltip = true;
                opts.theme = props.theme;
                if let Some((cx, cy)) = *crosshair.read() { opts.crosshair = Some((cx, cy)); } else { opts.crosshair = None; }

                // Apply current view to a temp chart snapshot and render
                let current = chart.read();
                let mut c = Chart::new();
                c.x_axis = current.x_axis.clone();
                c.y_axis = current.y_axis.clone();
                for s in &current.series { c.add_series(s.clone()); }
                (*view.read()).apply_to_chart(&mut c);
                // Try native RGBA + softbuffer first
                if let Ok((rgba, _w, _h, _row)) = c.render_to_rgba8(&opts) {
                    let win = use_window();
                    let winit_win: &dioxus_desktop::tao::window::Window = &win.window;
                    // SAFETY: We only use the handles within this call and immediately drop them
                    if let Ok(ctx) = unsafe { softbuffer::Context::new(winit_win) } {
                        if let Ok(mut surface) = unsafe { softbuffer::Surface::new(&ctx, winit_win) } {
                            let w = (opts.width.max(1)) as u32;
                            let h = (opts.height.max(1)) as u32;
                            let _ = surface.resize(w.try_into().unwrap(), h.try_into().unwrap());
                            if let Ok(mut frame) = surface.buffer_mut() {
                                let max_px = frame.len().min(rgba.len() / 4);
                                for (i, px) in rgba.chunks_exact(4).take(max_px).enumerate() {
                                    let r = px[0] as u32; let g = px[1] as u32; let b = px[2] as u32; let a = px[3] as u32;
                                    frame[i] = (a << 24) | (r << 16) | (g << 8) | b;
                                }
                                let _ = frame.present();
                                // Native blit succeeded; clear PNG fallback
                                if img_src.read().is_some() { img_src.set(None); }
                                return;
                            }
                        }
                    }
                }
                // Fallback to PNG <img> path if softbuffer not available
                if let Ok(bytes) = c.render_to_png_bytes(&opts) {
                    let b64 = base64::engine::general_purpose::STANDARD_NO_PAD.encode(bytes);
                    img_src.set(Some(format!("data:image/png;base64,{}", b64)));
                }
            }
        };

        // Initial render
        render_now();

        // Root container with a simple toolbar for interactivity (PNG rerender).
        rsx! {
            div {
                tabindex: 0,
                style: format!("position:relative; width:{}px; height:{}px; outline:none; background:#121214; color:#ddd;", *width.read(), *height.read()),
                // Toolbar
                div { style: "position:absolute; left:8px; top:8px; z-index:10; display:flex; gap:6px;",
                    // Pan controls
                    button { onclick: move |_| {
                            let mut v = *view.read();
                            v.pan_by_pixels(-50.0, 0.0, *width.read(), *height.read(), &RenderOptions::default().insets);
                            view.set(v);
                            render_now();
                        }, "Pan \u{2190}" }
                    button { onclick: move |_| {
                            let mut v = *view.read();
                            v.pan_by_pixels(50.0, 0.0, *width.read(), *height.read(), &RenderOptions::default().insets);
                            view.set(v);
                            render_now();
                        }, "Pan \u{2192}" }
                    button { onclick: move |_| {
                            let mut v = *view.read();
                            v.pan_by_pixels(0.0, -30.0, *width.read(), *height.read(), &RenderOptions::default().insets);
                            view.set(v);
                            render_now();
                        }, "Pan \u{2191}" }
                    button { onclick: move |_| {
                            let mut v = *view.read();
                            v.pan_by_pixels(0.0, 30.0, *width.read(), *height.read(), &RenderOptions::default().insets);
                            view.set(v);
                            render_now();
                        }, "Pan \u{2193}" }
                    button { onclick: move |_| {
                            let mut v = *view.read();
                            let (cx, cy) = ((*width.read() as f64)/2.0, (*height.read() as f64)/2.0);
                            v.zoom_at_pixel( 0.2, cx, cy, *width.read(), *height.read(), &RenderOptions::default().insets);
                            view.set(v);
                            render_now();
                        }, "Zoom +" }
                    button { onclick: move |_| {
                            let mut v = *view.read();
                            let (cx, cy) = ((*width.read() as f64)/2.0, (*height.read() as f64)/2.0);
                            v.zoom_at_pixel(-0.2, cx, cy, *width.read(), *height.read(), &RenderOptions::default().insets);
                            view.set(v);
                            render_now();
                        }, "Zoom -" }
                    button { onclick: move |_| {
                            view.set(ViewState::from_chart(&chart.read()));
                            render_now();
                        }, "Reset" }
                    button { onclick: move |_| {
                            let mut v = *view.read();
                            v.autoscale_y_visible(&chart.read());
                            view.set(v);
                            render_now();
                        }, "Autoscale Y" }
                    button { onclick: move |_| {
                            let mut c = chart.write();
                            use chart_core::axis::ScaleKind;
                            c.y_axis.kind = if c.y_axis.kind == ScaleKind::Linear { ScaleKind::Log10 } else { ScaleKind::Linear };
                            if c.y_axis.kind == ScaleKind::Log10 && c.y_axis.min <= 0.0 { c.y_axis.min = 1e-6; }
                            view.set(ViewState::from_chart(&c));
                            render_now();
                        }, "Toggle Log" }
                }
                // Image displaying the rendered chart (fallback when native blit unavailable)
                if let Some(src) = &*img_src.read() {
                    img { style: "position:absolute; inset:0; width:100%; height:100%; object-fit:contain; image-rendering:pixelated;", src: src.clone() }
                }
            }
        }
    }

    /// Tiny demo launcher so consumers can quickly mount the component.
    pub fn run_demo_ui() -> Result<(), String> {
        use dioxus::prelude::*;

        #[component]
        fn App() -> Element {
            // Simple demo data: a line series (sine wave)
            let mut xy = Vec::with_capacity(512);
            for i in 0..512 {
                let x = i as f64;
                let y = (x / 20.0).sin() * 10.0 + 20.0;
                xy.push((x, y));
            }
            let series: Vec<Series> = vec![Series::with_data(chart_core::SeriesType::Line, xy)];
            rsx! { super::ui::ChartCanvas { series, theme: Theme::dark(), show_tooltip: true, width_px: 1024, height_px: 640 } }
        }

        // Dioxus 0.6 desktop launcher
        // Dioxus 0.6 launch with explicit providers vec
        let providers: Vec<Box<dyn Fn() -> Box<dyn std::any::Any> + Send + Sync>> = Vec::new();
        let globals: Vec<Box<dyn std::any::Any>> = Vec::new();
        // Build transparent window config so softbuffer pixels can show through
        let cfg = dioxus_desktop::Config::new()
            .with_background_color((0, 0, 0, 0))
            .with_prerendered("<style>html,body{margin:0;height:100%;background:transparent}</style>".to_string());
        let providers: Vec<Box<dyn Fn() -> Box<dyn std::any::Any> + Send + Sync>> = Vec::new();
        let globals: Vec<Box<dyn std::any::Any>> = vec![Box::new(cfg)];
        dioxus_desktop::launch::launch(App, providers, globals);
        Ok(())
    }
}

/// Fallback when the `desktop` feature is not enabled.
#[cfg(not(feature = "desktop"))]
pub fn run_demo_ui() -> Result<(), &'static str> {
    Err("chart-dioxus built without `desktop` feature; enable features to run UI demo")
}

