# Constellation Chart — Demo Quickstart

This workspace includes a headless PNG demo and minimal examples.

Prereqs: Rust toolchain installed (1.70+ recommended). Skia is vendored via `skia-safe`.

## Run the OHLC Demo

- Using the included CSV (repo root contains `CRVUSDT_6h.csv`, `BTCUSDT_1m_100.csv`, `ETHUSDT_1m_500.csv`):

```
cargo run -p constellation-demo -- CRVUSDT_6h.csv
```

Outputs (written under `target/out/`):

- `chart_CRVUSDT_6h_candles.png` — candlestick series
- `chart_CRVUSDT_6h_bars.png` — OHLC bar series
- `chart_CRVUSDT_6h_hist.png` — histogram of (close - open) around 0.0
- `chart_CRVUSDT_6h_baseline.png` — baseline area of close vs average close

Additionally, matching `.svg` vector files are generated for each image.

Notes:
- The demo accepts either `.csv` or `.cvs` and will auto-swap the extension if the file isn't found.
- Logs print detected headers, row count, and price range.
- Series types implemented: Line, Candlestick, Bar, Histogram, Baseline.

## Windowed Demo (interactive)

```
cargo run -p constellation-window-demo -- CRVUSDT_6h.csv
```

Controls:
- Keys 1-4: switch to Candlesticks, Bars, Histogram, Baseline
- Key A: autoscale both axes to full data
- Key Y: autoscale Y to the visible X-range
- Key O: toggle SMA overlay on/off
- Key E: export current view to PNG + SVG (target/out)
- Mouse wheel: zoom at cursor (both axes)
- Left-drag: pan
- Crosshair: follows mouse cursor

## Run the Lines Example

```
cargo run -p chart-examples --bin example-lines
```

Output is written to `target/out/example_lines.png`.

## Tests and Golden Snapshot

- Run tests:

```
cargo test -p chart-core
```

- Update (bless) golden snapshot:

```
UPDATE_SNAPSHOTS=1 cargo test -p chart-core --test snapshot
```

Snapshot file: `crates/chart-core/tests/__snapshots__/basic_chart.png`.
