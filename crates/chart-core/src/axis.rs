// File: crates/chart-core/src/axis.rs
// Summary: Axis model with labels and ranges.

#[derive(Clone)]
pub struct Axis {
    pub label: String,
    pub min: f64,
    pub max: f64,
}

impl Axis {
    pub fn new(label: impl Into<String>, min: f64, max: f64) -> Self {
        Self { label: label.into(), min, max }
    }

    pub fn default_x() -> Self {
        Self::new("Time", 0.0, 10.0)
    }

    pub fn default_y() -> Self {
        Self::new("Price", 0.0, 100.0)
    }
}
