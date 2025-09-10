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

pub use chart::{Chart, RenderOptions};
pub use series::{Series, SeriesType};
pub use axis::Axis;
pub use view::ViewState;
pub use theme::Theme;
