// File: crates/demo/src/main.rs
// Summary: Demo that loads OHLC CSV (Binance-like) and renders candlesticks to PNG, with logging.

use anyhow::{Result, Context};
use chart_core::{Chart, Series, RenderOptions, Axis};
use chart_core::series::Candle;
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    // Accept path from CLI or fall back to your sample filename (supports .csv/.cvs swap)
    let raw = std::env::args().nth(1)
        .unwrap_or_else(|| "binanceus_CRVUSDT_6h_2023-09-13_to_2025-01-21.cvs".to_string());

    let (path, used_alt) = resolve_path(&raw)?;
    println!("📄 Using input file: {}", path.display());
    if used_alt {
        println!("   (extension swapped between .csv/.cvs)");
    }

    let candles = load_ohlc_csv(&path)
        .with_context(|| format!("failed to load CSV '{}'", path.display()))?;
    println!("✅ Loaded {} candles", candles.len());

    if candles.is_empty() {
        anyhow::bail!("no candles loaded — check headers/delimiter.");
    }

    // Derive axis ranges
    let n = candles.len();
    let (min_p, max_p) = minmax_price(&candles);
    println!("📈 Price range: [{:.4}, {:.4}] across {} rows", min_p, max_p, n);

    // Build chart
    let mut chart = Chart::new();
    chart.x_axis = Axis::new("Time (index/epoch)", 0.0, (n - 1) as f64);
    chart.y_axis = Axis::new("Price", min_p, max_p * 1.02);
    chart.add_series(Series::from_candles(candles));

    // Render to a unique output so you can see changes
    let opts = RenderOptions::default();
    let out = out_name(&path);
    chart.render_to_png(&opts, &out)?;
    println!("🖼️  Wrote {}", out.display());
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

/// Produce output file name like target/out/chart_CRVUSDT_6h.png
fn out_name(input: &Path) -> PathBuf {
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("chart");
    let short = stem.split('_').take(3).collect::<Vec<_>>().join("_");
    let mut out = PathBuf::from("target/out");
    std::fs::create_dir_all(&out).ok();
    out.push(format!("chart_{}.png", if short.is_empty() { "out" } else { &short }));
    out
}

/// Load Binance-like OHLC CSV into Candle vec.
fn load_ohlc_csv(path: &Path) -> Result<Vec<Candle>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)
        .with_context(|| format!("opening {}", path.display()))?;

    // Inspect headers (log them)
    let headers = rdr.headers()?.iter().map(|h| h.to_lowercase()).collect::<Vec<_>>();
    println!("🧭 Headers: {:?}", headers);

    let idx = |names: &[&str]| -> Option<usize> {
        for (i, h) in headers.iter().enumerate() {
            for want in names {
                if h == want { return Some(i); }
            }
        }
        None
    };

    // Common Binance headers
    let i_time = idx(&["time","timestamp","open_time","date","datetime"]);
    let i_open = idx(&["open","o"]);
    let i_high = idx(&["high","h"]);
    let i_low  = idx(&["low","l"]);
    let i_close= idx(&["close","c","adj_close","close_price"]);

    if i_open.is_none() || i_high.is_none() || i_low.is_none() || i_close.is_none() {
        println!("⚠️ Could not find one of open/high/low/close columns.");
    }

    let mut out = Vec::new();
    let mut row_index = 0_f64;

    for rec in rdr.records() {
        let rec = rec?;
        let parse = |i: Option<usize>| -> Option<f64> {
            i.and_then(|ix| rec.get(ix)).and_then(|s| s.trim().parse::<f64>().ok())
        };

        // x-value
        let t = if let Some(ix) = i_time {
            rec.get(ix).and_then(parse_time_to_f64).unwrap_or_else(|| { let v=row_index; row_index+=1.0; v })
        } else {
            let v=row_index; row_index+=1.0; v
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
    if s.is_empty() { return None; }
    if let Ok(n) = s.parse::<i64>() {
        if n > 10_i64.pow(12) { return Some(n as f64 / 1000.0); } // epoch ms → sec
        if n > 10_i64.pow(9)  { return Some(n as f64); }          // epoch sec
        return Some(n as f64);                                     // index-ish ints
    }
    None
}

fn swap_ext(p: &Path) -> Option<std::path::PathBuf> {
    let mut alt = p.to_path_buf();
    let ext = p.extension()?.to_string_lossy().to_lowercase();
    match ext.as_str() {
        "cvs" => { alt.set_extension("csv"); Some(alt) }
        "csv" => { alt.set_extension("cvs"); Some(alt) }
        _ => None
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
