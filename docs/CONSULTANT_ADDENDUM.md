# Expert Architecture & Guidance (Consultant Addendum)

This addendum codifies a TradingView-grade architecture and delivery plan tailored to a native Rust stack (Dioxus UI, Skia renderer, optional Tauri shell). It emphasizes maintainability, performance, deterministic rendering, and extensibility.

## Architecture Blueprint
- Model: Series (Line/Area/Candles/Bar/Histogram/Baseline), axes/scales, autoscale logic. Pure data; no UI state.
- ViewState: Visible ranges, transforms (x↔px, y↔px), crosshair, snapping, selections. No data mutation.
- Renderer: Backend-agnostic trait; Skia implementation with GPU first and CPU fallback. All overlays/crosshair rendered here for consistency.
- UI (Dioxus): Chart canvas component drives ViewState via input events, shows tooltips and panels, schedules redraws.
- Plugin: Stable traits for overlays (draw hooks) and indicators (compute hooks). Versioned params; reference plugins provided.
- Shell (Tauri): Windowing, menus, dialogs, packaging; integrates the Dioxus chart app.

## Public API (Lightweight-Charts parity, idiomatic Rust)
- Creation: `add_line_series(&[(f64,f64)])`, `add_area_series(..)`, `add_candlestick_series(&[Candle])`, `add_histogram_series(..).with_baseline(y0)`, `add_baseline_series(..).with_baseline(y0)`
- Updates: `set_data(..)`, `update_last(..)`, `push_point(..)`, `clear()`
- View: `set_visible_range(x_min, x_max)`, `autoscale_full()`, `autoscale_y_visible()`, `set_scale(Linear|Log)`
- Events: `on_crosshair_move`, `on_click`, `on_range_change`
- Theme: `set_theme(Light|Dark|Custom(Theme))`
- Export: `to_png(path)`, `to_svg(path)`
Note: Expose `ChartHandle/SeriesHandle` wrappers around `Arc<Mutex<...>>` for thread-safe UI integration.

## Renderer & Pixel-Perfect Rules
- Trait surface: `draw_chart(chart, view, surface)`, plus primitives `draw_text/line/rect/path` and `measure_text`.
- Skia GPU: Use `skia_safe::gpu::DirectContext` (GL/Vulkan decided at runtime). CPU fallback: `surfaces::raster_n32_premul`.
- 1px crisping: align strokes to N+0.5 device pixels; apply rounding post-transform for grid/axes.
- HiDPI: logical_size × scale_factor = surface size; map inputs accordingly; maintain consistent text metrics.
- Text: Use Skia textlayout with tabular numbers; cache shaped glyph runs by (font, size, string).

## Performance Plan
- LOD/Downsampling: LTTB for line/area; bucket OHLC by visible pixel columns; stride skip as last resort.
- Caching: Cache downsampled data by key (x_min, x_max, pixel_width) with hysteresis; reuse prebuilt `Path`s for candles/bars.
- Batching: Group same-style draws; avoid per-frame allocations; precompute vertices for stable data ranges.
- Text budget: Limit tick labels to a budget per axis; incremental layout; glyph atlas cache.
- Targets: 1M points @ 60 FPS (GPU); 100–200k smooth on CPU fallback.

## Interactivity & UX Guidelines
- Inputs: Left-drag pan; wheel zoom at cursor (exponential scaling base 1.2–1.4); modifiers for Y-only zoom.
- Autoscale: A = full extents, Y = autoscale Y to visible X-range; clamp to data extents if enabled.
- Crosshair: Snap to nearest x; for OHLC, snap to candle centers. Render via Skia with theme colors.
- Tooltips: Consolidated multi-series tooltip; monospace aligned columns; per-series formatter; positioned to avoid occlusion.
- Accessibility: High-contrast theme and larger font option; keyboard nav for nearest datapoints.

## Dioxus + Tauri Integration Plan
- Phase 1 (now): Dioxus Desktop + softbuffer CPU blit. Render via Skia CPU to RGBA8 and blit; deterministic tests and simple plumbing.
- Phase 2: Skia GPU path under wry/winit (GL/Vulkan). Handle resize/swapchain; preserve determinism for CPU tests.
- Phase 3: Tauri desktop shell (menus, dialogs, packaging). Embed Dioxus; expose commands for Open/Export.

## Plugin System
- Traits:
  - `Overlay { fn id(&self) -> &'static str; fn draw(&self, ctx, chart, view); fn handle_event(..) { } }`
  - `Indicator { fn id(&self) -> &'static str; fn compute(&self, input: &Series, params) -> Series }`
- Reference plugins: SMA overlay, VWAP overlay, H/V lines, measuring tool.

```rust
pub trait Overlay {
    fn id(&self) -> &'static str;
    fn draw(&self, ctx: &mut DrawCtx, chart: &Chart, view: &ViewState);
    fn handle_event(&mut self, _evt: &PointerEvent, _chart: &Chart, _view: &ViewState) {}
}

pub trait Indicator {
    fn id(&self) -> &'static str;
    fn compute(&self, input: &Series, params: &IndicatorParams) -> Series;
}
```

## Testing & Quality
- Golden snapshots: CPU raster, labels off, fixed size; per-series goldens; expand with zoomed-in/out variants.
- Property tests: Scale monotonicity and x↔px round-trip; autoscale invariants.
- Performance tests: Microbench downsampling and frame budgets for target datasets.
- CI: Multi-OS matrix; CPU goldens checked in; allow GPU path to skip in CI if needed.

## Design Tradeoffs & Calls
- CPU blit first → GPU later: Simpler, deterministic testing now; add GPU when UI boundaries are stable.
- Dioxus Desktop before Tauri: Faster iteration; wrap with Tauri once features settle.
- Per-series drawers with centralized batching/state: Keeps code readable while enabling shared perf wins.

## Immediate Issues to Address
- Tests: Fix typo in `crates/chart-core/tests/snapshot_series.rs` (histogram y-label has an extra quote; use "Y").
- Crosshair: Move overlay lines from post-blit buffer writes into Skia renderer for consistent AA, theming, and HiDPI.
- CSV time parsing: Add RFC3339 datetime fallback in `window-demo` for common CSVs beyond numeric epochs.

## Next Steps (Execution Plan)
1. Fix snapshot test label; run `cargo test -p chart-core` and bless if intended changes.
2. Introduce `scale.rs` with forward/inverse transforms and crisping helpers; adopt in chart rendering and demo.
3. Add `ViewState` to core; render crosshair via Skia using ViewState; remove manual buffer-line overlay.
4. Implement LTTB and OHLC bucketing for visible range; add simple cache keyed by view width and range.
5. Define `Theme` and thread it through renderers; keep tests with labels off for determinism.
6. Add `to_svg(path)` export using Skia vector path; wire to demo/app menu.
7. Scaffold `chart-dioxus` with softbuffer blit; wire mouse/keyboard to ViewState and existing APIs.

