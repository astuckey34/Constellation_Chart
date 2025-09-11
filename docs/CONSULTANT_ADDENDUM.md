# Constellation Chart — Consultant Addendum

This document captures the consulting plan to evolve Constellation Chart into a TradingView‑grade, desktop‑only charting engine using Rust + Dioxus (no Tauri). It consolidates architecture guidance, design principles, UX standards, and practical next steps mapped to the current codebase.

## Executive Summary

- Goal: Build a high‑performance, HiDPI‑aware, TradingView‑style charting engine with overlays, indicators, precise zoom/pan, and extensible UI, hosted by Dioxus Desktop.
- Current strengths: Solid core rendering via Skia CPU surfaces, clean axis/tick/tooltip logic, overlays/indicators API, downsampling (LTTB), and a window demo with crosshair + pan/zoom.
- Strategy: Keep Skia as the primary renderer (CPU now, GPU later), maintain a thin renderer boundary, and introduce a Dioxus shell that embeds a native chart surface.

## Architecture

- Core Engine (existing)
  - Modules: series, axes/scales, grid, theme, text shaping, overlays/indicators, view state.
  - Rendering path: Skia CPU raster today; prepared for GPU Skia or future wgpu.
  - Key files:
    - `crates/chart-core/src/chart.rs`
    - `crates/chart-core/src/plugin.rs`
    - `crates/chart-core/src/view.rs`
    - `crates/chart-core/src/scale.rs`

- Renderer Backends
  - Primary: Skia CPU (now), Skia GPU (GL/Metal/Vulkan) later, optional wgpu backend.
  - Crate: `crates/chart-render-skia/src/lib.rs` (CPU surface + GPU scaffolding behind feature).

- Host Shells
  - Winit/Softbuffer demo: Functional reference with pan/zoom/crosshair/overlays.
    - `crates/window-demo/src/main.rs`
  - Dioxus Component Shell (to implement): UI, events, layout; embeds a native chart surface.
    - `crates/chart-dioxus/src/lib.rs` (currently a stub)

ASCII overview:

```
App (Dioxus Desktop)
  UI state, panels, menus
  ChartCanvas (native surface)
    Renderer (Skia CPU/GPU)
      ChartCore (model, view, overlays)
        DataStore (series/indicators cache)
        ViewState (pan/zoom/autoscale)
        Plugin system (overlays/indicators)
```

## Design Principles (TradingView‑level)

- Separation of concerns: core data + transforms in `chart-core`; rendering via a thin canvas boundary; UI in Dioxus/Tauri.
- Pixel accuracy: half‑pixel alignment for hairlines; integer‑aligned grids/ticks; subpixel AA for text only.
- HiDPI correctness: DPR‑aware layout, paddings, tick lengths, and hit testing.
- Performance first: downsample by viewport; batch by paint state; avoid unnecessary re‑layout; prefer cheap translations where viable.
- Extensibility: plugins for overlays/indicators with world‑space events; multiple axes and scale groups.

## MVP Feature Scope

- Series: Candles, Bars, Line, Histogram, Baseline (present).
- Interactions: Pan, zoom at cursor, crosshair with tooltip (present), Y autoscale, log scale toggle (present).
- Overlays: SMA, guide lines (present); next: EMA, VWAP.
- Export: PNG/SVG (present). Later: copy to clipboard, PDF.
- UI: Dioxus toolbar (period, symbol, theme, overlays), status bar (OHLC under cursor), theme switch.

## Renderer Boundary Plan

- Near‑term: Keep Skia as the single drawing API. Hosts provide CPU raster or GPU surface; call `Chart::draw_onto_canvas`.
- Later: Introduce a `Renderer` trait when adding a second backend (e.g., wgpu) to avoid premature abstraction.

## Dioxus Integration

- Target: Dioxus UI with a native chart surface in the same window; no browser canvas.
- Approach: Dioxus Desktop (wry/tao). Obtain raw window handle; create `softbuffer::Surface` for the chart region; drive redraws from the host event loop. Communicate UI controls to the chart via shared state or channels.

## Event Model

- Centralize pan/zoom/crosshair in `ViewState` (already implemented and used in window demo).
- Map device px → world via `TimeScale`/`ValueScale` within plot insets.
- Pointer capture + drag state for panning; wheel zoom around cursor; keyboard accelerators (A=reset, Y=autoscale, L=log scale).
- Overlay events dispatched in world coordinates using `OverlayEvent` (in `plugin.rs`).

## HiDPI and Crispness

- Maintain `opts.dpr` and scale text, paddings, tick lengths; align 1px strokes to half‑pixels when `crisp_lines`.
- Ensure consistency across grid, axes, ticks, series outlines, SVG output.

## Performance Strategy

- CPU raster path
  - Downsample (LTTB) when points >> pixels; choose target samples based on viewport width.
  - Batch geometry by paint state (already used for candles/bars).
  - Avoid reconstructing paths on pure pan; prefer cheap recompute of transforms; consider translate‑only re‑paint later.
  - HUD/crosshair as separate pass (future) for minimal redraw.
- GPU path (later)
  - Skia GPU: swap to GPU surface; reuse `draw_onto_canvas` unchanged.
  - Optional wgpu: instanced rectangles (candles), line strips, dynamic uniforms; SDF text atlas.
- Text
  - Cache font metrics and tick layouts; re‑shape only when tick set changes.

## Data Model and Scales

- Support index‑based and time‑based X; keep time tick heuristics (`detect_time_like`, `format_time_tick`).
- Multiple Y‑axes with scale groups
  - Introduce `ScaleGroup` IDs; aggregate min/max per group; render left/right axes.
  - API addition: `Series::with_scale_group(id)` or a setter.

## UI/UX Guidelines

- Mouse/keys
  - Left‑drag pan; wheel zoom around cursor; Shift/Ctrl to constrain zoom; double‑click or A for autoscale; Y for autoscale‑Y on visible range; L to toggle log.
- Crosshair
  - Add axis boxes showing X (time) and Y (price) near the hairlines; current tooltip is a good start.
- Grid/labels
  - Dynamic tick density; avoid overlaps by measuring label widths with the text shaper.
- Overlays/Indicators
  - Toggleable from a side panel; consistent color coding; quick parameter controls (periods).
- Export
  - PNG/SVG now; add copy‑to‑clipboard and “Export selection” later.

## Concrete Next Steps

1) Implement `ChartCanvas` in `crates/chart-dioxus`
   - Props: `series`, `overlays`, `initial_view`, `theme`, `on_view_change`.
   - Internals: create native `softbuffer::Surface`, render loop using `Chart::render_to_rgba8`, wire mouse/keyboard to `ViewState` and `OverlayEvent`.
   - File: `crates/chart-dioxus/src/lib.rs`.

2) Extract a reusable softbuffer helper
   - Utility to own `softbuffer::Context`/`Surface`, handle `resize`, `present_rgba`.
   - Use from `window-demo` and `chart-dioxus`.
   - Files: new small module under `crates/window-demo` or a shared crate.

3) Crosshair axis value boxes and legend row
   - Extend after crosshair render to draw small value boxes at axes and a minimal legend.
   - File: `crates/chart-core/src/chart.rs` (around crosshair/tooltip drawing).

4) Prepare Skia GPU behind a feature
   - Wire GL/Metal/Vulkan via `skia-safe` features; keep optional.
   - File: `crates/chart-render-skia/src/lib.rs` (GPU scaffolding already stubbed).

5) Multiple Y scales (design + staged impl)
   - Add `ScaleGroup`; per‑group autoscale; render left/right axes; per‑series assignment.

## Code Sketches (Illustrative)

### Dioxus `ChartCanvas` skeleton (CPU path; simplified)

```rust
use dioxus::prelude::*;
use chart_core::{Chart, RenderOptions, ViewState, Theme, Series};
use std::sync::{Arc, Mutex};

pub struct ChartProps {
    pub series: Vec<Series>,
    pub theme: Theme,
    pub on_view_change: Option<EventHandler<ViewState>>, // optional callback
}

pub fn ChartCanvas(cx: Scope<ChartProps>) -> Element {
    let chart = use_ref(cx, || {
        let mut c = Chart::new();
        for s in &cx.props.series { c.add_series(s.clone()); }
        c
    });
    let view = use_ref(cx, || ViewState::from_chart(&chart.read()));
    let dpr = use_state(cx, || 1.0f32);

    // TODO: obtain raw window handle from host, construct softbuffer::Surface
    // TODO: wire mouse/keyboard → update ViewState → request redraw
    // TODO: on redraw, build RenderOptions, apply view, render_to_rgba8, blit to surface

    cx.render(rsx! {
        div { /* Native chart region host; attach event handlers */ }
    })
}
```

### Softbuffer helper (reuse from window demo)

- Encapsulate `Context`/`Surface` creation, `resize(w,h)`, `present_rgba(&[u8])`.
- Use in `crates/window-demo/src/main.rs` and `chart-dioxus` component.

## Challenges & Decisions

- Dioxus Desktop as the host: use wry/tao windowing; avoid additional app shells.
- Event loop ownership: Dioxus host drives redraws; avoid a second loop.
- Renderer abstraction: defer trait extraction until a second backend exists to avoid premature complexity.

## Quality Bar Checklist

- Performance: Smooth pan/zoom at 60 FPS for 10–50k visible points; downsample beyond.
- HiDPI: Crisp hairlines/text; correct scaling.
- UX: Crosshair axis boxes; consistent theme; accessible shortcuts.
- Extensibility: Plugins independent/composable; world‑space event routing.

## Repo‑Specific Notes

- Good patterns:
  - RGBA blit path: `crates/window-demo/src/main.rs`.
  - Overlay/indicator system: `crates/chart-core/src/plugin.rs`.
  - HiDPI + crispness via `RenderOptions::dpr` and half‑pixel alignment in SVG.
- Improvement opportunities:
  - Cache tick label layouts to avoid re‑shaping every frame.
  - Add axis value boxes to complement the tooltip HUD.

## Next Actions (Choose 1–3 to implement next)

1) Scaffold `ChartCanvas` in `crates/chart-dioxus` with CPU softbuffer path and full input wiring.
2) Add crosshair axis value boxes and a small legend row in `chart-core`.
3) Extract a reusable softbuffer helper and refactor `window-demo` to use it.
