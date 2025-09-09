// File: crates/chart-core/tests/snapshot.rs
// Purpose: Golden snapshot harness with bless flow.
// Behavior:
// - Renders a deterministic small chart to PNG bytes.
// - If env UPDATE_SNAPSHOTS=1, (re)writes the snapshot file.
// - Else, if snapshot exists, compares bytes for exact match.
// - Else, logs a note and returns (skips) without failing to ease first run.

use chart_core::{Chart, RenderOptions, Axis, Series};

fn render_bytes() -> Vec<u8> {
    let mut chart = Chart::new();
    chart.x_axis = Axis::new("X", 0.0, 4.0);
    chart.y_axis = Axis::new("Y", 0.0, 4.0);
    chart.add_series(Series::with_data(
        chart_core::SeriesType::Line,
        vec![(0.0, 0.0), (1.0, 1.0), (2.0, 0.0), (3.0, 1.5), (4.0, 1.0)],
    ));

    // Match default opts used by Chart::render_to_png
    let mut opts = RenderOptions::default();
    opts.draw_labels = false; // avoid text nondeterminism across platforms
    // Render via public API to a temp file then read back
    let tmp = std::path::PathBuf::from("target/test_out/snapshot_tmp.png");
    std::fs::create_dir_all(tmp.parent().unwrap()).ok();
    Chart { series: chart.series, x_axis: chart.x_axis, y_axis: chart.y_axis }
        .render_to_png(&opts, &tmp)
        .expect("render to tmp");
    std::fs::read(tmp).expect("read tmp png")
}

#[test]
fn golden_basic_chart() {
    let bytes = render_bytes();
    let snap_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/__snapshots__");
    let snap_path = snap_dir.join("basic_chart.png");

    let update = std::env::var("UPDATE_SNAPSHOTS").ok().map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if update {
        std::fs::create_dir_all(&snap_dir).expect("create snapshots dir");
        std::fs::write(&snap_path, &bytes).expect("write snapshot");
        eprintln!("[snapshot] Updated {} ({} bytes)", snap_path.display(), bytes.len());
        return;
    }

    if snap_path.exists() {
        let want = std::fs::read(&snap_path).expect("read snapshot");
        // Compare decoded pixel buffers to avoid PNG encoder variance
        let got_img = image::load_from_memory(&bytes).expect("decode got").to_rgba8();
        let want_img = image::load_from_memory(&want).expect("decode want").to_rgba8();
        assert_eq!(got_img.as_raw(), want_img.as_raw(), "rendered pixels differ from golden snapshot: {}", snap_path.display());
    } else {
        eprintln!("[snapshot] Missing snapshot {}; set UPDATE_SNAPSHOTS=1 to bless.", snap_path.display());
        // Skip without failing on first run
    }
}
