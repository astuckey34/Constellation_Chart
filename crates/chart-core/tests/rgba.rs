// File: crates/chart-core/tests/rgba.rs
// Purpose: Validate RGBA rendering buffer shape and a few pixels.

use chart_core::{Axis, Chart, RenderOptions, Series};
use chart_core::series::SeriesType;

#[test]
fn render_rgba8_buffer() {
    let mut chart = Chart::new();
    chart.x_axis = Axis::new("X", 0.0, 4.0);
    chart.y_axis = Axis::new("Y", 0.0, 4.0);
    chart.add_series(Series::with_data(SeriesType::Line, vec![(0.0, 0.0), (4.0, 4.0)]));

    let mut opts = RenderOptions::default();
    opts.draw_labels = false; // avoid font variance
    let (px, w, h, stride) = chart.render_to_rgba8(&opts).expect("rgba render");
    assert_eq!(w as usize * h as usize * 4, px.len());
    assert_eq!(stride, (w as usize) * 4);

    // Check background alpha in top-left pixel (RGBA)
    let a = px[3];
    assert_eq!(a, 255);
}

