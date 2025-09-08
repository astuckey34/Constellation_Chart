// File: crates/chart-core/src/grid.rs
// Summary: Simple grid/tick layout helpers.

pub fn linspace(start: f64, end: f64, steps: usize) -> Vec<f64> {
    if steps < 2 { return vec![start, end]; }
    let step = (end - start) / (steps as f64 - 1.0);
    (0..steps).map(|i| start + step * i as f64).collect()
}
