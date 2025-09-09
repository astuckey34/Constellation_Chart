# Constellation Chart — Demo Quickstart

This workspace includes a headless PNG demo and minimal examples.

Prereqs: Rust toolchain installed (1.70+ recommended). Skia is vendored via `skia-safe`.

## Run the OHLC Demo

- Using the included CSV:

```
cargo run -p constellation-demo -- binanceus_CRVUSDT_6h_2023-09-13_to_2025-01-21.csv
```

Outputs (written under `target/out/`):

- `chart_binanceus_CRVUSDT_6h_candles.png` — candlestick series
- `chart_binanceus_CRVUSDT_6h_bars.png` — OHLC bar series
- `chart_binanceus_CRVUSDT_6h_hist.png` — histogram of (close − open) around 0.0
- `chart_binanceus_CRVUSDT_6h_baseline.png` — baseline area of close vs average close

 Notes:
 - The demo accepts either `.csv` or `.cvs` and will auto-swap the extension if the file isn’t found.
 - Logs print detected headers, row count, and price range.
 - Series types implemented: Line, Candlestick, Bar, Histogram, Baseline.

## Windowed Demo (interactive)

```
cargo run -p constellation-window-demo -- binanceus_CRVUSDT_6h_2023-09-13_to_2025-01-21.csv
```

Controls:
- Any key: cycle between Candles, Bars, Histogram, Baseline
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
