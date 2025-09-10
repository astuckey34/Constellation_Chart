use chart_core::downsample::lttb;
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, black_box};

fn gen_xy(n: usize) -> Vec<(f64, f64)> {
    let mut v = Vec::with_capacity(n);
    let mut x = 0.0f64;
    let mut y = 0.0f64;
    for i in 0..n {
        x += 1.0;
        // simple waveform with drift
        y = (i as f64 * 0.01).sin() * 10.0 + (i as f64 * 0.0001);
        v.push((x, y));
    }
    v
}

fn bench_lttb(c: &mut Criterion) {
    let mut group = c.benchmark_group("lttb");
    for &n in &[50_000usize, 100_000usize] {
        let data = gen_xy(n);
        for &target in &[1_000usize, 2_000usize, 5_000usize] {
            group.bench_with_input(BenchmarkId::from_parameter(format!("n{n}_t{target}")), &target, |b, &t| {
                b.iter_batched(
                    || data.clone(),
                    |d| { let _ = black_box(lttb(&d, t)); },
                    BatchSize::SmallInput,
                );
            });
        }
    }
    group.finish();
}

criterion_group!(benches, bench_lttb);
criterion_main!(benches);

