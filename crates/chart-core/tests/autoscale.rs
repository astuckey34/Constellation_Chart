// File: crates/chart-core/tests/autoscale.rs
// Purpose: Validate autoscale over mixed series types.

use chart_core::{Axis, Chart, Series};
use chart_core::series::{Candle, SeriesType};

#[test]
fn autoscale_mixed_series() {
    let mut chart = Chart::new();

    // XY series
    chart.add_series(Series::with_data(SeriesType::Line, vec![(0.0, 1.0), (5.0, 3.0)]));

    // Candles
    chart.add_series(Series::from_candles(vec![
        Candle { t: 2.0, o: 2.0, h: 6.0, l: 1.5, c: 4.0 },
        Candle { t: 3.0, o: 4.0, h: 5.5, l: 2.0, c: 2.5 },
    ]));

    chart.autoscale_axes(0.0);

    // X spans 0..5 from line vs 2..3 from candles => expect ~0..5
    assert!(chart.x_axis.min <= 0.0 + 1e-9);
    assert!(chart.x_axis.max >= 5.0 - 1e-9);

    // Y min uses candle low (1.5) vs line min 1.0 => expect <= 1.0
    assert!(chart.y_axis.min <= 1.0 + 1e-9);
    // Y max uses candle high 6.0 or line 3.0 => expect >= 6.0
    assert!(chart.y_axis.max >= 6.0 - 1e-9);
}

