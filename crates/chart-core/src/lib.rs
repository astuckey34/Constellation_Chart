// File: crates/chart-core/src/lib.rs
// Summary: Core library entry point; exports public API for chart construction and rendering.

pub mod chart;
pub mod series;
pub mod axis;
pub mod grid;
pub mod types;
pub mod geometry;
pub mod scale;
pub mod view;
pub mod theme;
pub mod text;
pub mod downsample;
pub mod plugin;

pub use chart::{Chart, RenderOptions};
pub use series::{Series, SeriesType};
pub use axis::Axis;
pub use view::ViewState;
pub use theme::Theme;
pub use text::TextShaper;
pub use downsample::{lttb, aggregate_ohlc_buckets};
pub use plugin::{Indicator, IndicatorParams, Overlay, SmaOverlay, OverlayEvent, HvLineOverlay};
