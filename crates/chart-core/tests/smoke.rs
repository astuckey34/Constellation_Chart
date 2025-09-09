// File: crates/chart-core/tests/smoke.rs
// Purpose: Basic end-to-end render smoke test writing a PNG.

use chart_core::{Chart, RenderOptions, Axis, Series};

#[test]
fn render_smoke_png() {
    // Minimal data: tiny line series
    let mut chart = Chart::new();
    chart.x_axis = Axis::new("X", 0.0, 4.0);
    chart.y_axis = Axis::new("Y", 0.0, 4.0);
    chart.add_series(Series::with_data(
        chart_core::SeriesType::Line,
        vec![(0.0, 0.0), (1.0, 2.0), (2.0, 1.0), (3.0, 3.5), (4.0, 2.5)],
    ));

    let opts = RenderOptions::default();
    let out = std::path::PathBuf::from("target/test_out/smoke.png");
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();

    chart.render_to_png(&opts, &out).expect("render should succeed");
    let meta = std::fs::metadata(&out).expect("output exists");
    assert!(meta.len() > 0, "png should be non-empty");

    // Also verify in-memory API works
    let bytes = chart.render_to_png_bytes(&opts).expect("render bytes");
    assert!(bytes.starts_with(&[137, 80, 78, 71]), "should be PNG header");
}
