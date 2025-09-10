// File: crates/demo/src/main.rs
// Summary: Demo loads OHLC CSV and renders multiple series types (candles, bars, histogram, baseline) to PNGs.

use anyhow::{Context, Result};
use chart_core::{Axis, Chart, RenderOptions, Series};
use chart_core::series::{Candle, SeriesType};
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    // Accept path from CLI or fall back to sample filename (supports .csv/.cvs swap)
    let raw = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "binanceus_CRVUSDT_6h_2023-09-13_to_2025-01-21.cvs".to_string());

    let (path, used_alt) = resolve_path(&raw)?;
    println!("Using input file: {}", path.display());
    if used_alt {
        println!("  (extension swapped between .csv/.cvs)");
    }

    let candles = load_ohlc_csv(&path)
        .with_context(|| format!("failed to load CSV '{}'", path.display()))?;
    println!("Loaded {} candles", candles.len());

    if candles.is_empty() {
        anyhow::bail!("no candles loaded — check headers/delimiter.");
    }

    // Derive axis ranges
    let n = candles.len();
    let (min_p, max_p) = minmax_price(&candles);
    println!("Price range: [{:.4}, {:.4}] across {} rows", min_p, max_p, n);

    let opts = RenderOptions::default();

    // Optional downsampling/aggregation for large datasets
    let target_points = 1500usize;
    let bucket = if n > target_points { ((n as f64) / (target_points as f64)).ceil() as usize } else { 1 };

    // 1) Candlesticks
    let mut chart_c = Chart::new();
    chart_c.x_axis = Axis::new("Time (index/epoch)", 0.0, (n - 1) as f64);
    chart_c.y_axis = Axis::new("Price", min_p, max_p * 1.02);
    let series_c = Series::from_candles(candles.clone()).aggregate_ohlc(bucket);
    chart_c.add_series(series_c);
    let out_c = out_name_with(&path, "candles");
    chart_c.render_to_png(&opts, &out_c)?;
    let out_c_svg = out_c.with_extension("svg");
    chart_c.render_to_svg(&opts, &out_c_svg)?;
    println!("Wrote {}", out_c.display());

    // 2) OHLC Bars
    let mut chart_bars = Chart::new();
    chart_bars.x_axis = Axis::new("Time (index/epoch)", 0.0, (n - 1) as f64);
    chart_bars.y_axis = Axis::new("Price", min_p, max_p * 1.02);
    chart_bars.add_series(Series::from_candles_as(SeriesType::Bar, candles.clone()).aggregate_ohlc(bucket));
    let out_bars = out_name_with(&path, "bars");
    chart_bars.render_to_png(&opts, &out_bars)?;
    let out_bars_svg = out_bars.with_extension("svg");
    chart_bars.render_to_svg(&opts, &out_bars_svg)?;
    println!("Wrote {}", out_bars.display());

    // Prepare derived series for Histogram and Baseline
    let xy_diff_full: Vec<(f64, f64)> = candles
        .iter()
        .enumerate()
        .map(|(i, c)| (i as f64, c.c - c.o))
        .collect();
    let xy_diff = if n > target_points { chart_core::lttb(&xy_diff_full, target_points) } else { xy_diff_full };
    let xy_close_full: Vec<(f64, f64)> = candles
        .iter()
        .enumerate()
        .map(|(i, c)| (i as f64, c.c))
        .collect();
    let xy_close = if n > target_points { chart_core::lttb(&xy_close_full, target_points) } else { xy_close_full };

    // 3) Histogram of close-open relative to baseline 0.0
    let (min_h, max_h) = minmax_xy(&xy_diff);
    let mut chart_hist = Chart::new();
    chart_hist.x_axis = Axis::new("Index", 0.0, (n - 1) as f64);
    chart_hist.y_axis = Axis::new("Delta Close-Open", min_h.min(0.0), max_h.max(0.0));
    chart_hist.add_series(Series::with_data(SeriesType::Histogram, xy_diff).with_baseline(0.0));
    let out_hist = out_name_with(&path, "hist");
    chart_hist.render_to_png(&opts, &out_hist)?;
    let out_hist_svg = out_hist.with_extension("svg");
    chart_hist.render_to_svg(&opts, &out_hist_svg)?;
    println!("Wrote {}", out_hist.display());

    // 4) Baseline area of closes around average close
    let avg_close = candles.iter().map(|c| c.c).sum::<f64>() / n as f64;
    let (min_c, max_c) = minmax_xy(&xy_close);
    let mut chart_base = Chart::new();
    chart_base.x_axis = Axis::new("Index", 0.0, (n - 1) as f64);
    chart_base.y_axis = Axis::new("Close", min_c, max_c);
    chart_base.add_series(Series::with_data(SeriesType::Baseline, xy_close).with_baseline(avg_close));
    let out_base = out_name_with(&path, "baseline");
    chart_base.render_to_png(&opts, &out_base)?;
    let out_base_svg = out_base.with_extension("svg");
    chart_base.render_to_svg(&opts, &out_base_svg)?;
    println!("Wrote {}", out_base.display());

    Ok(())
}

/// Resolve path, trying .csv/.cvs swap if needed.
/// Returns (actual_path, used_alt)
fn resolve_path(raw: &str) -> Result<(PathBuf, bool)> {
    let p = Path::new(raw);
    if p.exists() {
        return Ok((p.to_path_buf(), false));
    }
    if let Some(alt) = swap_ext(p) {
        if alt.exists() {
            return Ok((alt, true));
        }
    }
    anyhow::bail!("file not found: {}", p.display());
}

/// Produce output file name like target/out/chart_<stem>_<suffix>.png
fn out_name_with(input: &Path, suffix: &str) -> PathBuf {
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("chart");
    let short = stem.split('_').take(3).collect::<Vec<_>>().join("_");
    let mut out = PathBuf::from("target/out");
    std::fs::create_dir_all(&out).ok();
    if short.is_empty() {
        out.push(format!("chart_{}.png", suffix));
    } else {
        out.push(format!("chart_{}_{}.png", short, suffix));
    }
    out
}

/// Load Binance-like OHLC CSV into Candle vec.
fn load_ohlc_csv(path: &Path) -> Result<Vec<Candle>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)
        .with_context(|| format!("opening {}", path.display()))?;

    // Inspect headers (log them)
    let headers = rdr
        .headers()?
        .iter()
        .map(|h| h.to_lowercase())
        .collect::<Vec<_>>();
    println!("Headers: {:?}", headers);

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

    // Common Binance headers
    let i_time = idx(&["time", "timestamp", "open_time", "date", "datetime"]);
    let i_open = idx(&["open", "o"]);
    let i_high = idx(&["high", "h"]);
    let i_low = idx(&["low", "l"]);
    let i_close = idx(&["close", "c", "adj_close", "close_price"]);

    if i_open.is_none() || i_high.is_none() || i_low.is_none() || i_close.is_none() {
        println!("Warning: Could not find one of open/high/low/close columns.");
    }

    let mut out = Vec::new();
    let mut row_index = 0_f64;

    for rec in rdr.records() {
        let rec = rec?;
        let parse = |i: Option<usize>| -> Option<f64> { i.and_then(|ix| rec.get(ix)).and_then(|s| s.trim().parse::<f64>().ok()) };

        // x-value
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
    Ok(out)
}

fn parse_time_to_f64(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Ok(n) = s.parse::<i64>() {
        if n > 10_i64.pow(12) {
            return Some(n as f64 / 1000.0);
        } // epoch ms -> sec
        if n > 10_i64.pow(9) {
            return Some(n as f64);
        } // epoch sec
        return Some(n as f64);
    }
    None
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
