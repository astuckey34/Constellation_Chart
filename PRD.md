<!-- See docs/CONSULTANT_ADDENDUM.md for expert architecture & guidance. -->
---

# PRD: Constellation Chart vs TradingView Lightweight-Charts

## Feature Comparison

| Category                   | TradingView Lightweight-Charts (JS)              | Constellation Chart (Rust + Dioxus + Skia)                                                         |
| -------------------------- | ------------------------------------------------ | -------------------------------------------------------------------------------------------------- |
| Language / Runtime         | JavaScript/TypeScript in browser; Canvas2D API   | Pure Rust, compiled native; Dioxus for UI; Skia GPU backend                                        |
| Platform Targets           | Browser only (WASM via JS bindings possible)     | Desktop native (Windows, macOS, Linux) with optional WASM fallback                                 |
| Rendering Backend          | Canvas2D immediate-mode, raster at DPR           | Skia GPU/CPU accelerated vector engine, sRGB consistent                                            |
| HiDPI Support              | DPR scaling handled manually by lib              | Native DPR/HiDPI aware via Skia surfaces; half-pixel crisping                                      |
| Series Types               | Line, Area, Candlestick, Bar, Histogram, Baseline| Same (Line, Area, Candlestick, Bar, Histogram, Baseline)                                           |
| Downsampling / Aggregation | Basic subsampling; slows >100k points            | LTTB downsampling for lines, OHLC bucket aggregation, stride skipping; target 1M+ points at 60 FPS |
| Scales                     | Time & Price axes, autoscale, padding            | Time & Price axes, autoscale, padding; log scale (implemented)                                     |
| Interactivity              | Crosshair, drag pan, scroll zoom, tooltips       | Same, plus plugin hooks for custom overlays/gestures                                               |
| Text Rendering             | Browser font stack; measureText (imprecise)      | Skia text shaping & metrics; kerning, tabular numbers, subpixel AA                                 |
| Plugin / Extensibility     | Limited (no formal plugin API)                   | Formal plugin system: overlays, indicators, annotations, custom drawing                            |
| Testing                    | Visual/manual regression tests                   | Golden snapshot tests (Skia CPU + PNG), unit tests, property tests                                 |
| Color / Theme              | CSS colors, alpha fills, gradients               | Rust `Color`; sRGB pipeline; theming presets (dark/light/solarized/high-contrast); custom themes planned |
| Performance                | Smooth up to ~50k–100k points                    | Target: 100k–1M points @ 60 FPS (native GPU batching)                                              |
| Export                     | Screenshots via browser                          | PNG export + vector SVG export (implemented)                                                       |
| Dependencies               | JS runtime + browser DOM                         | Pure Rust binary; Skia via `skia-safe`; no JS runtime                                              |

---

## Summary

- Parity: All core features (candles, bars, line, area, histogram, baseline, autoscaling, crosshair, grid, tooltips) are matched 1:1.
- Advantage: Constellation Chart has native text fidelity, GPU acceleration, plugin system, golden testing, and no JS runtime overhead.
- Stretch: Log scale, rich theming, and 1M+ points performance headroom go beyond TradingView.

---

## Status

- Implemented:
  - Series: Line, Candlestick, Bar, Histogram, Baseline
  - Autoscale + ViewState; pan/zoom; crosshair + tooltips
  - Theme presets; crisp grid/axes; log scale (Y)
  - PNG export + vector SVG export
  - Golden snapshot tests and examples
- Partial:
  - Plugin system: core traits defined; rendering hooks & reference overlays pending
  - GPU path: feature-gated GL demo available; not default
  - Text fidelity: Skia textlayout in place; numeric/tabular polish ongoing
- Planned:
  - Dioxus app shell + (optional) Tauri packaging
  - WASM/Web fallback demo
  - Python interop via PyO3
  - Performance measurement/benchmarks toward 1M @ 60 FPS

---

## User Value Proposition

I'm building Constellation Chart because I want charting to live natively in the same Rust ecosystem I already use for quant work. TradingView Lightweight-Charts is solid, but it's JavaScript — and that's not where I want my codebase or runtime dependencies to live.

Why this matters:

1. Rust-Native Workflow
   - No cross-language overhead. Charts belong directly in my Rust projects.
2. Performance Headroom
   - Designed for millions of points at 60 FPS — necessary for backtests and intraday analysis.
3. Fidelity & Precision
   - Skia-powered rendering gives crystal-clear numbers and chart visuals I can trust.
4. Extensibility
   - A plugin system lets me evolve workflows without hacking the core.
5. Parity with TradingView, but in Rust
   - Same features and API philosophy, aligned with Rust's safety and performance model.

---

## Roadmap (Priority-Stacked)

### MVP (Minimum Viable Demo)

- Render a chart surface with Skia backend.
- Draw static series: candles, lines, grid.
- Basic time/price axes.
- Validate rendering fidelity.

Note: Ensures chart appearance works before adding interactivity.

### Parity Layer (Match TradingView Basics)

- Pan & Zoom.
- Crosshair + tooltips.
- Autoscale axes.
- Multiple series types (candlestick, bar, histogram, baseline).
- Light/dark theming.

### Differentiation Layer (Beyond TradingView)

- GPU batching & downsampling (target 1M+ points @ 60 FPS).
- High-fidelity text rendering (kerning, tabular numbers).
- Formal plugin system (indicators, overlays, annotations).
- PNG/SVG export.
- Pure Rust, no JS runtime.

### Stretch Layer (Nice-to-Haves)

- Logarithmic scale.
- Rich custom theming system.
- Advanced axis formatting.
- WASM/Web fallback demo.
- Multi-monitor / HiDPI optimizations.

---

## Adoption & Ecosystem Strategy

1. Rust-First Core
   - Rust crate (`chart-core`).
   - Demo app for CSV/JSON data + chart rendering + export.
   - Golden snapshot testing to ensure stability.

2. Python Interop (Stretch Goal)
   - PyO3 bindings for quants prototyping in Python.
   - `pip install` target; DataFrame to chart pipeline.

3. Migration Path
   - API mirrors TradingView Lightweight-Charts (`addLineSeries`, `addCandlestickSeries`, etc.).
   - Easy porting from JS codebases.

4. Ecosystem Growth
   - Docs + tutorials.
   - Examples repo (dashboards, TA overlays).
   - Plugin ecosystem for shared indicators/overlays.

---

## Risks & Mitigation

1. Cross-Platform Rendering Variance
   - Risk: GPU driver differences.
   - Mitigation: Golden snapshot tests, multi-OS validation, CPU fallback.

2. Performance Ceiling
   - Risk: 1M points @ 60 FPS may not hold everywhere.
   - Mitigation: Downsampling, stride skipping, aggregation, benchmarks.

3. Plugin Ecosystem Bootstrapping
   - Risk: Empty ecosystem at launch.
   - Mitigation: Provide reference plugins, stable API.

4. Adoption Friction (Rust vs JS)
   - Risk: Community inertia around JS.
   - Mitigation: API parity, migration guides, eventual PyO3 bindings.

5. Community & Maintenance Risk
   - Risk: If open-sourced, sustaining momentum.
   - Mitigation: Small core, plugin model, CI/CD testing, encourage contributions.

---

## Progress Addendum: Runtime UX (Window Demo)

- Theme toggle: press `T` to switch light/dark (dark default).
- Downsampling toggle: press `D` to enable/disable LTTB/bucket aggregation based on window width.
- Autoscale: `A` resets to full extents; `Y` autoscale Y over the current visible X range.
- CSV input handling: CPU and GPU demos ignore option-like args and fall back to sample CSVs in repo root (`CRVUSDT_6h.csv`, `BTCUSDT_1m_100.csv`, `ETHUSDT_1m_500.csv`) when a path isn't provided.

---

## Progress Addendum: GPU Demo (OpenGL)

- Feature-gated GPU demo: `constellation-window-demo` supports an OpenGL path when built with `--features gpu-gl-demo` and launched with `--gpu`.
  - Example: `cargo run -p constellation-window-demo --features gpu-gl-demo -- --gpu`
- Data: Loads the same CSV OHLC input and renders Candlesticks, Bars, Histogram (close–open), and Baseline (close vs avg) using Skia GPU surfaces.
- Interactions: Crosshair + tooltips, pan (left-drag), zoom (mouse wheel), autoscale (A), Y-autoscale visible (Y).
- Toggles: Theme (T) light/dark, downsampling (D) on/off; window title reflects current theme and DS status.
- Resize: Recreates the GPU render target at new size and reapplies downsampling for width-based target points.
- Safety: CPU remains the default; GPU demo is optional and isolated behind a feature flag.

---

## How To Run (Demos)

- CPU demo:
  - Command: `cargo run -p constellation-window-demo`
- GPU demo (OpenGL, feature-gated):
  - Command: `cargo run -p constellation-window-demo --features gpu-gl-demo -- --gpu`
- CSV input:
  - Pass a CSV path as the first non-option arg, or place a sample in repo root: `CRVUSDT_6h.csv`, `BTCUSDT_1m_100.csv`, `ETHUSDT_1m_500.csv`.
- Controls (CPU & GPU):
  - Series: `1`..`4`
  - Autoscale: `A` (full), `Y` (Y-only visible range)
  - Downsampling: `D` (toggle)
  - Theme: `T` (cycle presets)
  - Scale: `L` (toggle linear/log Y)
  - Mouse: crosshair/tooltip on move, wheel zoom at cursor, left-drag pan
  - Exit: `Esc`

---

## Workspace Layout (high-level)

```
crates/
  chart-core/             # core data types & API (no UI deps)
  chart-render-skia/      # Skia-backed renderer (GPU/CPU)
  chart-dioxus/           # Dioxus component glue (UI shell, events)
  chart-examples/         # small binaries showing features
  chart-plugins-example/  # reference plugin(s) for API shape
  chart-python/           # (future) PyO3 binding crate
  demo/                   # headless PNG demo
  window-demo/            # interactive windowed demo (softbuffer)
assets/
  fonts/                  # fallback font(s) for consistent text metrics
  data/                   # sample CSV/JSON OHLC for demo & tests
  snapshots/              # golden PNGs for render regression
workflows/
  ci.yml                  # build, clippy, tests, golden-snapshots
```

Last updated: 2025-09-10
Next review trigger: when plugin overlays render in window demo, or when WASM decision is made.
