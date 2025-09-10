// File: crates/chart-plugins-example/src/lib.rs
// Summary: Example plugin(s) implementing chart-core plugin traits.

use chart_core::plugin::{Indicator, IndicatorParams};
use chart_core::series::Series;

/// Simple SMA indicator using chart-core helper.
pub struct Sma;

impl Indicator for Sma {
    fn id(&self) -> &'static str { "sma" }

    fn compute(&self, input: &Series, params: &IndicatorParams) -> Series {
        chart_core::plugin::SmaIndicator.compute(input, params)
    }
}
