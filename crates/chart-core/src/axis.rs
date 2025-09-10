// File: crates/chart-core/src/axis.rs
// Summary: Axis model with labels and ranges.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScaleKind {
    Linear,
    Log10,
}

#[derive(Clone)]
pub struct Axis {
    pub label: String,
    pub min: f64,
    pub max: f64,
    pub kind: ScaleKind,
}

impl Axis {
    pub fn new(label: impl Into<String>, min: f64, max: f64) -> Self {
        Self { label: label.into(), min, max, kind: ScaleKind::Linear }
    }

    pub fn default_x() -> Self {
        let mut a = Self::new("Time", 0.0, 10.0);
        a.kind = ScaleKind::Linear;
        a
    }

    pub fn default_y() -> Self {
        let mut a = Self::new("Price", 0.0, 100.0);
        a.kind = ScaleKind::Linear;
        a
    }
}
