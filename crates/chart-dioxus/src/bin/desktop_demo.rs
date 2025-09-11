// File: crates/chart-dioxus/src/bin/desktop_demo.rs
// Purpose: Minimal launcher for the Dioxus desktop ChartCanvas demo.

#[cfg(feature = "desktop")]
fn main() {
    if let Err(e) = chart_dioxus::ui::run_demo_ui() {
        eprintln!("chart-dioxus demo error: {e}");
    }
}

#[cfg(not(feature = "desktop"))]
fn main() {
    eprintln!("This demo requires --features desktop");
}

