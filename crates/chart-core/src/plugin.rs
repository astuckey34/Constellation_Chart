// File: crates/chart-core/src/plugin.rs
// Summary: Plugin trait definitions (overlay & indicator) with minimal, renderer-agnostic API.

use crate::series::{Series, SeriesType};
use crate::Chart;
use std::cell::RefCell;

#[derive(Clone, Copy, Debug)]
pub struct IndicatorParams {
    pub period: usize,
}

impl Default for IndicatorParams {
    fn default() -> Self { Self { period: 14 } }
}

/// Indicator transforms input series into a derived series (typically a Line series).
pub trait Indicator {
    fn id(&self) -> &'static str;
    fn compute(&self, input: &Series, params: &IndicatorParams) -> Series;
}

/// Overlay can later render custom visuals on top of a chart. Backend wiring is deferred.
pub trait Overlay {
    fn id(&self) -> &'static str;
    /// Compute one or more series to render as overlays on top of the chart.
    fn compute(&self, chart: &Chart) -> Vec<Series>;
    /// Handle user interaction in world space (chart coordinates).
    fn handle_event(&self, _evt: &OverlayEvent, _chart: &Chart) {}
}

/// Helper: simple SMA over (x, y) pairs, returns averaged (x, yavg) points.
pub fn sma_xy(data: &[(f64, f64)], period: usize) -> Vec<(f64, f64)> {
    if period == 0 || data.is_empty() { return Vec::new(); }
    let p = period as usize;
    if data.len() < p { return Vec::new(); }
    let mut out = Vec::with_capacity(data.len() - p + 1);
    let mut sum = 0.0f64;
    for i in 0..data.len() {
        sum += data[i].1;
        if i + 1 >= p {
            if i + 1 > p { sum -= data[i - p].1; }
            let x = data[i].0;
            out.push((x, sum / (p as f64)));
        }
    }
    out
}

/// Helper: SMA over candle closes, returns (index or time, avg_close).
pub fn sma_candles(data: &[(f64, f64, f64, f64, f64)], period: usize) -> Vec<(f64, f64)> {
    // tuple: (t, o, h, l, c)
    if period == 0 || data.is_empty() { return Vec::new(); }
    let p = period as usize;
    if data.len() < p { return Vec::new(); }
    let mut out = Vec::with_capacity(data.len() - p + 1);
    let mut sum = 0.0f64;
    for i in 0..data.len() {
        sum += data[i].4;
        if i + 1 >= p {
            if i + 1 > p { sum -= data[i - p].4; }
            let x = data[i].0;
            out.push((x, sum / (p as f64)));
        }
    }
    out
}

/// Default SMA indicator implementation for convenience.
pub struct SmaIndicator;

impl Indicator for SmaIndicator {
    fn id(&self) -> &'static str { "sma" }

    fn compute(&self, input: &Series, params: &IndicatorParams) -> Series {
        match input.series_type {
            SeriesType::Line | SeriesType::Histogram | SeriesType::Baseline => {
                let xy = sma_xy(&input.data_xy, params.period);
                Series::with_data(SeriesType::Line, xy)
            }
            SeriesType::Candlestick | SeriesType::Bar => {
                let oh = input.data_ohlc.iter().map(|c| (c.t, c.o, c.h, c.l, c.c)).collect::<Vec<_>>();
                let xy = sma_candles(&oh, params.period);
                Series::with_data(SeriesType::Line, xy)
            }
        }
    }
}

/// Simple SMA overlay that computes a moving average over the first series in the chart.
pub struct SmaOverlay {
    pub period: usize,
}

impl Overlay for SmaOverlay {
    fn id(&self) -> &'static str { "sma_overlay" }

    fn compute(&self, chart: &Chart) -> Vec<Series> {
        let p = if self.period.max(1) == 0 { 14 } else { self.period.max(1) };
        let params = IndicatorParams { period: p };
        let sma = SmaIndicator;
        // Prefer XY input; if none, derive from candles close.
        if let Some(s) = chart.series.iter().find(|s| matches!(s.series_type, SeriesType::Line | SeriesType::Histogram | SeriesType::Baseline)) {
            return vec![sma.compute(s, &params)];
        }
        if let Some(s) = chart.series.iter().find(|s| matches!(s.series_type, SeriesType::Candlestick | SeriesType::Bar)) {
            return vec![sma.compute(s, &params)];
        }
        Vec::new()
    }
}

/// Overlay event in world coordinates (x/y are chart values, not pixels).
pub enum OverlayEvent {
    PointerDown { x: f64, y: f64 },
    PointerUp { x: f64, y: f64 },
    PointerMove { x: f64, y: f64 },
}

/// Horizontal guide line overlay; click sets Y, renders a line across current X range.
pub struct HvLineOverlay {
    y: RefCell<Option<f64>>,
}

impl HvLineOverlay {
    pub fn new() -> Self { Self { y: RefCell::new(None) } }
}

impl Overlay for HvLineOverlay {
    fn id(&self) -> &'static str { "hv_line" }

    fn compute(&self, chart: &Chart) -> Vec<Series> {
        if let Some(y) = *self.y.borrow() {
            let x0 = chart.x_axis.min;
            let x1 = chart.x_axis.max;
            return vec![Series::with_data(SeriesType::Line, vec![(x0, y), (x1, y)])];
        }
        Vec::new()
    }

    fn handle_event(&self, evt: &OverlayEvent, _chart: &Chart) {
        match *evt {
            OverlayEvent::PointerDown { y, .. } => { *self.y.borrow_mut() = Some(y); }
            _ => {}
        }
    }
}

// Allow sharing a single overlay instance across frames
impl Overlay for std::sync::Arc<HvLineOverlay> {
    fn id(&self) -> &'static str { HvLineOverlay::id(self) }
    fn compute(&self, chart: &Chart) -> Vec<Series> { HvLineOverlay::compute(self, chart) }
    fn handle_event(&self, evt: &OverlayEvent, chart: &Chart) { HvLineOverlay::handle_event(self, evt, chart) }
}
