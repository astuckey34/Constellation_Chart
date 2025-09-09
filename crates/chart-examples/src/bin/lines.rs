// File: crates/chart-examples/src/bin/lines.rs
// Summary: Minimal example that renders a simple line chart to PNG.

use chart_core::{Axis, Chart, RenderOptions, Series, SeriesType};

fn main() {
    // Build a simple line series
    let data = vec![
        (0.0, 0.0),
        (1.0, 1.2),
        (2.0, 0.8),
        (3.0, 1.8),
        (4.0, 1.4),
        (5.0, 2.0),
    ];

    let mut chart = Chart::new();
    chart.x_axis = Axis::new("X", 0.0, 5.0);
    chart.y_axis = Axis::new("Y", 0.0, 2.2);
    chart.add_series(Series::with_data(SeriesType::Line, data));

    let opts = RenderOptions::default();
    let out = std::path::PathBuf::from("target/out/example_lines.png");
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
    chart.render_to_png(&opts, &out).expect("render to png");
    println!("Wrote {}", out.display());
}
