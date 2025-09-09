// File: crates/chart-core/src/series.rs
// Summary: Series model for line, candlestick, bar, histogram, and baseline data.

#[derive(Clone, Copy, Debug)]
pub enum SeriesType {
    Line,
    Candlestick,
    Bar,         // OHLC bar (no filled body)
    Histogram,   // (x, y) bars from baseline (0.0)
    Baseline,    // area relative to baseline value (default 0.0)
}

#[derive(Clone, Copy, Debug)]
pub struct Candle {
    pub t: f64,  // time/index (displayed on X)
    pub o: f64,
    pub h: f64,
    pub l: f64,
    pub c: f64,
}

pub struct Series {
    pub series_type: SeriesType,
    pub data_xy: Vec<(f64, f64)>,     // used by Line/Histogram/Baseline
    pub data_ohlc: Vec<Candle>,       // used by Candlestick/Bar
    pub baseline: Option<f64>,        // used by Baseline/Histogram (origin)
}

impl Series {
    pub fn new(series_type: SeriesType) -> Self {
        Self { series_type, data_xy: Vec::new(), data_ohlc: Vec::new(), baseline: None }
    }

    pub fn with_data(series_type: SeriesType, data: Vec<(f64, f64)>) -> Self {
        Self { series_type, data_xy: data, data_ohlc: Vec::new(), baseline: None }
    }

    pub fn from_candles(candles: Vec<Candle>) -> Self {
        Self { series_type: SeriesType::Candlestick, data_xy: Vec::new(), data_ohlc: candles, baseline: None }
    }

    pub fn from_candles_as(series_type: SeriesType, candles: Vec<Candle>) -> Self {
        Self { series_type, data_xy: Vec::new(), data_ohlc: candles, baseline: None }
    }

    pub fn with_baseline(mut self, baseline: f64) -> Self {
        self.baseline = Some(baseline);
        self
    }
}
