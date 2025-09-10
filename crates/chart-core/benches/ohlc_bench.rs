use chart_core::downsample::aggregate_ohlc_buckets;
use chart_core::series::Candle;
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, black_box};

fn gen_ohlc(n: usize) -> Vec<Candle> {
    let mut v = Vec::with_capacity(n);
    let mut t = 0.0f64;
    let mut price = 100.0f64;
    for _ in 0..n {
        t += 1.0;
        let o = price;
        let h = o + 1.0;
        let l = o - 1.0;
        let c = o + 0.2;
        price = c;
        v.push(Candle { t, o, h, l, c });
    }
    v
}

fn bench_aggregate(c: &mut Criterion) {
    let mut group = c.benchmark_group("aggregate_ohlc");
    for &n in &[50_000usize, 100_000usize] {
        let data = gen_ohlc(n);
        for &bucket in &[5usize, 10usize, 20usize] {
            group.bench_with_input(BenchmarkId::from_parameter(format!("n{n}_b{bucket}")), &bucket, |b, &bk| {
                b.iter_batched(
                    || data.clone(),
                    |d| { let _ = black_box(aggregate_ohlc_buckets(&d, bk)); },
                    BatchSize::SmallInput,
                );
            });
        }
    }
    group.finish();
}

criterion_group!(benches, bench_aggregate);
criterion_main!(benches);

