// File: crates/chart-core/tests/snapshot_series.rs
// Purpose: Golden snapshots for additional series types: candlesticks, bars, histogram, baseline.

use chart_core::{Axis, Chart, RenderOptions, Series};
use chart_core::series::{Candle, SeriesType};

fn bless_mode() -> bool {
    std::env::var("UPDATE_SNAPSHOTS").ok().map(|v| v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(false)
}

fn write_or_compare(path: &std::path::Path, bytes: &[u8]) {
    let update = bless_mode();
    if update {
        if let Some(parent) = path.parent() { std::fs::create_dir_all(parent).ok(); }
        std::fs::write(path, bytes).expect("write snapshot");
        eprintln!("[snapshot] Updated {} ({} bytes)", path.display(), bytes.len());
        return;
    }
    if path.exists() {
        let want = std::fs::read(path).expect("read snapshot");
        let got_img = image::load_from_memory(bytes).expect("decode got").to_rgba8();
        let want_img = image::load_from_memory(&want).expect("decode want").to_rgba8();
        assert_eq!(got_img.as_raw(), want_img.as_raw(), "Pixels differ: {}", path.display());
    } else {
        eprintln!("[snapshot] Missing {}; set UPDATE_SNAPSHOTS=1 to bless.", path.display());
    }
}

fn render_to_bytes<F: FnOnce(&mut Chart)>(mut build: F, x_label: &str, y_label: &str) -> Vec<u8> {
    let mut chart = Chart::new();
    chart.x_axis = Axis::new(x_label, 0.0, 9.0);
    chart.y_axis = Axis::new(y_label, -2.0, 6.0);
    build(&mut chart);

    let mut opts = RenderOptions::default();
    opts.draw_labels = false; // deterministic
    chart.render_to_png_bytes(&opts).expect("render bytes")
}

#[test]
fn golden_candlesticks() {
    let candles = vec![
        Candle { t: 0.0, o: 2.0, h: 3.0, l: 1.0, c: 2.5 },
        Candle { t: 1.0, o: 2.5, h: 3.5, l: 2.0, c: 2.0 },
        Candle { t: 2.0, o: 2.0, h: 4.0, l: 1.5, c: 3.0 },
        Candle { t: 3.0, o: 3.0, h: 3.2, l: 2.4, c: 2.6 },
        Candle { t: 4.0, o: 2.6, h: 2.9, l: 2.1, c: 2.2 },
    ];
    let bytes = render_to_bytes(|c| c.add_series(Series::from_candles(candles)), "X", "Y");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/__snapshots__/candlesticks.png");
    write_or_compare(&path, &bytes);
}

#[test]
fn golden_bars() {
    let candles = vec![
        Candle { t: 0.0, o: 2.0, h: 3.0, l: 1.0, c: 2.5 },
        Candle { t: 1.0, o: 2.5, h: 3.5, l: 2.0, c: 2.0 },
        Candle { t: 2.0, o: 2.0, h: 4.0, l: 1.5, c: 3.0 },
        Candle { t: 3.0, o: 3.0, h: 3.2, l: 2.4, c: 2.6 },
        Candle { t: 4.0, o: 2.6, h: 2.9, l: 2.1, c: 2.2 },
    ];
    let bytes = render_to_bytes(|c| c.add_series(Series::from_candles_as(SeriesType::Bar, candles)), "X", "Y");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/__snapshots__/bars.png");
    write_or_compare(&path, &bytes);
}

#[test]
fn golden_histogram() {
    let data = (0..10).map(|i| (i as f64, ((i as f64) - 4.0) * 0.4)).collect::<Vec<_>>();
    let bytes = render_to_bytes(|c| c.add_series(Series::with_data(SeriesType::Histogram, data).with_baseline(0.0)), "X", "Y");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/__snapshots__/histogram.png");
    write_or_compare(&path, &bytes);
}

#[test]
fn golden_baseline() {
    let data = vec![
        (0.0, 1.0), (1.0, 1.2), (2.0, 0.8), (3.0, 1.8), (4.0, 1.0),
    ];
    let bytes = render_to_bytes(|c| c.add_series(Series::with_data(SeriesType::Baseline, data).with_baseline(1.0)), "X", "Y");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/__snapshots__/baseline.png");
    write_or_compare(&path, &bytes);
}

