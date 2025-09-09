// File: crates/chart-core/src/series.rs
// Summary: Series model for line, candlestick, bar, histogram, and baseline data.
// Notes:
// - This file intentionally keeps the original `Series` layout to maintain
//   compatibility with existing rendering code. New, safer constructors and
//   helpers are provided to tighten invariants without breaking callers.

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

impl Candle {
    /// Try to construct a candle enforcing OHLC invariants:
    /// l <= min(o,c) and h >= max(o,c), and l <= h.
    pub fn try_new(t: f64, o: f64, h: f64, l: f64, c: f64) -> Result<Self, &'static str> {
        let lo = o.min(c);
        let hi = o.max(c);
        if l > lo { return Err("low above min(open,close)"); }
        if h < hi { return Err("high below max(open,close)"); }
        if l > h { return Err("low above high"); }
        Ok(Self { t, o, h, l, c })
    }
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

    /// Get baseline value or default (0.0) when not set.
    pub fn baseline_value(&self) -> f64 { self.baseline.unwrap_or(0.0) }
}
