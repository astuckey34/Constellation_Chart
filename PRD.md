
---

# 📊 PRD: Constellation Chart vs TradingView Lightweight-Charts

## 🔄 Feature Comparison

| Category                       | TradingView Lightweight-Charts (JS)                  | Constellation Chart (Rust + Dioxus + Skia)                                                         |
| ------------------------------ | ---------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| **Language / Runtime**         | JavaScript/TypeScript in browser; Canvas2D API       | Pure Rust, compiled native; Dioxus for UI; Skia GPU backend                                        |
| **Platform Targets**           | Browser only (WebAssembly via JS bindings possible)  | Desktop native (Windows, macOS, Linux) with optional WASM fallback                                 |
| **Rendering Backend**          | Canvas2D immediate-mode, raster at DPR               | Skia GPU/CPU accelerated vector engine, sRGB consistent                                            |
| **HiDPI Support**              | DPR scaling handled manually by lib                  | Native DPR/HiDPI aware via Skia surfaces; half-pixel crisping                                      |
| **Series Types**               | Line, Area, Candlestick, Bar, Histogram, Baseline    | Same (Line, Area, Candlestick, Bar, Histogram, Baseline)                                           |
| **Downsampling / Aggregation** | Basic subsampling; chart slows with >100k points     | LTTB downsampling for lines, OHLC bucket aggregation, stride skipping; target 1M+ points at 60 FPS |
| **Scales**                     | Time & Price axes, autoscale, padding                | Time & Price axes, autoscale, padding; log scale (planned)                                         |
| **Interactivity**              | Crosshair, drag pan, scroll zoom, hover tooltips     | Same features, plus plugin hooks for custom overlays/gestures                                      |
| **Text Rendering**             | Browser font stack; Canvas2D measureText (imprecise) | Skia text shaping & metrics; kerning, tabular numbers, subpixel AA                                 |
| **Plugin / Extensibility**     | Limited (overlays via app code, no plugin API)       | Formal plugin system: overlays, indicators, annotations, custom drawing                            |
| **Testing**                    | Visual/manual regression tests                       | Golden snapshot tests (Skia CPU → PNG), unit tests, property tests                                 |
| **Color / Theme**              | CSS colors, alpha fills, gradients                   | Rust `Color` type; sRGB pipeline; theming system planned (dark/light/custom)                       |
| **Performance**                | Smooth up to \~50k–100k points, drops beyond         | Target: 100k–1M points @ 60 FPS (native GPU batching)                                              |
| **Export**                     | Screenshots via browser                              | PNG/SVG export via Skia surfaces (headless render)                                                 |
| **Dependencies**               | JS runtime + browser DOM                             | Pure Rust binary; Skia via `skia-safe`; no JS runtime                                              |

---

## 🚀 Summary

* **Parity**: All core features (candles, bars, line, area, histogram, baseline, autoscaling, crosshair, grid, tooltips) are matched 1:1.
* **Advantage**: Constellation Chart has **native text fidelity, GPU acceleration, plugin system, golden testing, and no JS runtime overhead**.
* **Stretch Features**: Log scale, rich theming, and 1M+ points performance headroom go **beyond TradingView**.

---

## 🎯 User Value Proposition

I’m building **Constellation Chart** because I want my charting to live natively in the same Rust ecosystem I already use for quant work. TradingView lightweight-charts is solid, but it’s JavaScript — and that’s not where I want my codebase or runtime dependencies to live.

Here’s why this matters to me:

1. **Rust-Native Workflow**

   * No cross-language overhead. Charts belong directly in my Rust projects.

2. **Performance Headroom**

   * Designed for millions of points at 60 FPS — necessary for backtests and intraday analysis.

3. **Fidelity & Precision**

   * Skia-powered rendering gives me crystal-clear numbers and chart visuals I can trust.

4. **Extensibility**

   * A plugin system lets me evolve workflows without hacking the core.

5. **Parity with TradingView, but in Rust**

   * Same features, same API philosophy — but aligned with Rust’s safety and performance model.

---

## 🛠 Roadmap (Priority-Stacked)

### MVP (Minimum Viable Demo)

* Render a chart surface with Skia backend.
* Draw static series: candles, lines, grid.
* Basic time/price axes.
* Validate rendering fidelity.

✅ Ensures chart appearance works before adding interactivity.

### Parity Layer (Match TradingView Basics)

* Pan & Zoom.
* Crosshair + tooltips.
* Autoscale axes.
* Multiple series types (candlestick, bar, histogram, baseline).
* Light/dark theming.

### Differentiation Layer (Beyond TradingView)

* GPU batching & downsampling (target 1M+ points @ 60 FPS).
* High-fidelity text rendering (kerning, tabular numbers).
* Formal plugin system (indicators, overlays, annotations).
* PNG/SVG export.
* Pure Rust, no JS runtime.

### Stretch Layer (Nice-to-Haves)

* Logarithmic scale.
* Rich custom theming system.
* Advanced axis formatting.
* WASM/Web fallback demo.
* Multi-monitor / HiDPI optimizations.

---

## 🧩 Adoption & Ecosystem Strategy

1. **Rust-First Core**

   * Rust crate (`constellation-chart`).
   * Demo app for CSV/JSON data → chart rendering + export.
   * Golden snapshot testing to ensure stability.

2. **Python Interop (Stretch Goal)**

   * PyO3 bindings for quants prototyping in Python.
   * `pip install constellation-chart` → DataFrame to chart pipeline.

3. **Migration Path**

   * API mirrors TradingView lightweight-charts (`addLineSeries`, `addCandlestickSeries`, etc.).
   * Easy porting from JS codebases.

4. **Ecosystem Growth**

   * Docs + tutorials.
   * Examples repo (dashboards, TA overlays).
   * Plugin ecosystem for shared indicators/overlays.

---

## ⚠️ Risks & Mitigation

1. **Cross-Platform Rendering Variance**

   * Risk: GPU driver differences.
   * Mitigation: Golden snapshot tests, multi-OS validation, CPU fallback.

2. **Performance Ceiling**

   * Risk: 1M points @ 60 FPS may not hold everywhere.
   * Mitigation: Downsampling, stride skipping, aggregation, benchmarks.

3. **Plugin Ecosystem Bootstrapping**

   * Risk: Empty ecosystem at launch.
   * Mitigation: Provide reference plugins, stable API.

4. **Adoption Friction (Rust vs JS)**

   * Risk: Community inertia around JS.
   * Mitigation: API parity, migration guides, eventual PyO3 bindings.

5. **Community & Maintenance Risk**

   * Risk: If open-sourced, sustaining momentum.
   * Mitigation: Small core, plugin model, CI/CD testing, encourage contributions.

---

# 📁 Workspace Layout

## Progress Update (engineering status)

- MVP:
  - [x] Skia raster surface PNG rendering (headless) implemented (see `crates/chart-core/src/chart.rs:1`).
  - [x] Static series rendering for line and candlestick; grid lines drawn (see `crates/chart-core/src/chart.rs:1`).
  - [x] Basic time/price axes model and drawing (see `crates/chart-core/src/axis.rs:1`).
- [~] Rendering fidelity validation: demo produces PNGs; golden snapshot tests are placeholders (see `tests/rendering.rs:1`).
- [x] Rendering fidelity validation: crisp 1px grid/axes (half-pixel alignment), deterministic snapshots (labels off in tests), and golden images for all series types (see `crates/chart-core/tests/snapshot*.rs:1`).

- Parity Layer:
  - [x] Pan & zoom (windowed demo: mouse wheel zoom, drag pan).
  - [~] Crosshair (windowed demo: crosshair lines at cursor; tooltips pending).
  - [ ] Autoscale axes (ranges derived in demo only; no autoscale yet).
- [x] Multiple series types: Line, Candlestick, Bar, Histogram, Baseline implemented (see `crates/chart-core/src/series.rs:1`, `crates/chart-core/src/chart.rs:1`).
  - [ ] Light/dark theming (not started).

- Differentiation:
  - [ ] GPU batching & downsampling (not started).
  - [ ] High‑fidelity text shaping/metrics (not started).
  - [ ] Formal plugin system (scaffold crate exists, API not defined).
  - [~] Export: PNG implemented; SVG pending (see `crates/chart-core/src/chart.rs:1`).
  - [x] Pure Rust, no JS runtime.

- Stretch:
  - [ ] Logarithmic scale (not started).
  - [ ] Theming system (not started).
  - [ ] Advanced axis formatting (not started).
  - [ ] WASM/Web fallback demo (not started).
  - [ ] HiDPI optimizations (not started).

- Adoption & Ecosystem:
  - [x] Rust workspace and core crate established (see `Cargo.toml:1`).
  - [~] Demo app: CSV → chart → PNG path implemented; JSON not yet (see `crates/demo/src/main.rs:1`).
- [x] Golden snapshot tests: harness active with blessed baseline at `crates/chart-core/tests/__snapshots__/basic_chart.png:1` (update via `UPDATE_SNAPSHOTS=1`).
  - [ ] Python interop (stub crate only; see `crates/chart-python/src/lib.rs:1`).
  - [ ] API parity/migration helpers (not started).
  - [ ] Docs/examples/plugins (examples stub only; see `crates/chart-examples/src/bin/lines.rs:1`).

Notes:
- Generated PNGs found under `target/out/` confirm end‑to‑end render (see `target/out/chart.png:1`).
- Minor housekeeping: a test placeholder resides at `crates/chart-core/src/rendering.rs:1` and likely belongs under `tests/`.

```
constellation-chart/
├─ Cargo.toml                  # Workspace manifest
├─ rust-toolchain.toml         # (optional) pin toolchain
├─ Makefile                    # common dev tasks (fmt, clippy, test, demo)
├─ README.md
├─ .gitignore
├─ .github/
│  └─ workflows/
│     └─ ci.yml               # build, clippy, tests, golden-snapshots
├─ assets/
│  ├─ fonts/                  # fallback font(s) for consistent text metrics
│  └─ data/                   # sample CSV/JSON OHLC for demo & tests
├─ snapshots/                  # golden PNGs for render regression
│  ├─ linux/
│  ├─ macos/
│  └─ windows/
└─ crates/
   ├─ chart-core/             # core data types & API (no UI deps)
   │  ├─ src/
   │  │  ├─ lib.rs
   │  │  ├─ chart.rs
   │  │  ├─ series/
   │  │  │  ├─ mod.rs
   │  │  │  ├─ line.rs
   │  │  │  ├─ candlestick.rs
   │  │  │  └─ histogram.rs
   │  │  ├─ axes/
   │  │  │  ├─ mod.rs
   │  │  │  ├─ time_axis.rs
   │  │  │  └─ price_axis.rs
   │  │  ├─ layout.rs
   │  │  ├─ color.rs
   │  │  ├─ theme.rs
   │  │  ├─ downsample.rs     # LTTB, bucket aggregation, stride
   │  │  └─ plugin.rs         # trait definitions for overlays/indicators
   │  └─ Cargo.toml
   │
   ├─ chart-render-skia/      # Skia-backed renderer (GPU/CPU)
   │  ├─ src/
   │  │  ├─ lib.rs
   │  │  ├─ renderer.rs       # draws core primitives using skia-safe
   │  │  ├─ text.rs           # shaping/metrics, tabular nums
   │  │  ├─ surfaces.rs       # GPU vs CPU surfaces; PNG/SVG export
   │  │  └─ snapshot.rs       # headless CPU render → PNG (golden tests)
   │  └─ Cargo.toml
   │
   ├─ chart-dioxus/           # Dioxus component glue (UI shell, events)
   │  ├─ src/
   │  │  ├─ lib.rs
   │  │  ├─ components/
   │  │  │  └─ chart_canvas.rs
   │  │  ├─ input.rs          # (later) pan/zoom, crosshair
   │  │  └─ state.rs          # view state (scale, ranges) separate from model
   │  └─ Cargo.toml
   │
   ├─ chart-demo/             # minimal MVP app (no interactivity first)
   │  ├─ src/
   │  │  └─ main.rs
   │  ├─ Cargo.toml
   │  └─ README.md
   │
   ├─ chart-examples/         # small binaries showing features
   │  ├─ src/
   │  │  ├─ lines.rs
   │  │  ├─ candles.rs
   │  │  └─ export_png.rs
   │  └─ Cargo.toml
   │
   ├─ chart-plugins-example/  # reference plugin(s) for API shape
   │  ├─ src/
   │  │  ├─ lib.rs            # e.g., SMA overlay implementing Plugin
   │  │  └─ sma.rs
   │  └─ Cargo.toml
   │
   └─ chart-python/           # (future) PyO3 binding crate
      ├─ src/
      │  └─ lib.rs
      └─ Cargo.toml

