use anyhow::Result;
use chart_core::{Axis, Chart, RenderOptions, Series};
use chart_core::series::SeriesType;
use criterion::{criterion_group, criterion_main, Criterion, black_box};

fn build_chart_xy(n: usize) -> Chart {
    let mut ch = Chart::new();
    let mut data = Vec::with_capacity(n);
    for i in 0..n {
        let x = i as f64;
        let y = (i as f64 * 0.01).sin() * 10.0 + (i as f64 * 0.0001);
        data.push((x, y));
    }
    ch.x_axis = Axis::new("X", 0.0, (n - 1) as f64);
    ch.y_axis = Axis::new("Y", -12.0, 12.0);
    ch.add_series(Series::with_data(SeriesType::Line, data));
    ch
}

fn bench_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("render_png_bytes");
    for &n in &[10_000usize, 50_000usize] {
        group.bench_function(format!("xy_{n}"), |b| {
            let ch = build_chart_xy(n);
            let mut opts = RenderOptions::default();
            opts.width = 800;
            opts.height = 500;
            opts.draw_labels = false;
            b.iter(|| -> Result<()> {
                let bytes = ch.render_to_png_bytes(&opts)?;
                black_box(bytes);
                Ok(())
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_render);
criterion_main!(benches);

