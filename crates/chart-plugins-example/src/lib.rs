// File: crates/chart-plugins-example/src/lib.rs
// Summary: Example plugin(s) and trait shape (stub).

pub trait Plugin {
    fn name(&self) -> &'static str;
}

pub struct SmaOverlay;

impl Plugin for SmaOverlay {
    fn name(&self) -> &'static str { "SMA" }
}

