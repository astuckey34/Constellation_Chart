// File: crates/chart-core/src/downsample.rs
// Summary: Downsampling utilities (LTTB for XY; OHLC bucket aggregation).

use crate::series::Candle;

/// Largest-Triangle-Three-Buckets downsampling for XY series.
/// Returns up to `threshold` points preserving overall shape.
pub fn lttb(points: &[(f64, f64)], threshold: usize) -> Vec<(f64, f64)> {
    let n = points.len();
    if threshold == 0 || n == 0 { return Vec::new(); }
    if threshold >= n || n <= 2 { return points.to_vec(); }
    if threshold == 1 { return vec![points[0]]; }

    let bucket_size = (n - 2) as f64 / (threshold - 2) as f64;
    let mut sampled = Vec::with_capacity(threshold);
    // Always include first
    sampled.push(points[0]);

    let mut a = 0usize; // a is the index of the selected point from previous bucket

    for i in 0..(threshold - 2) {
        let start = (1.0 + (i as f64) * bucket_size).floor() as usize;
        let end = (1.0 + ((i + 1) as f64) * bucket_size).floor().min((n - 1) as f64) as usize;

        // Compute average of next bucket (end..next_end)
        let next_start = end;
        let next_end = (1.0 + ((i + 2) as f64) * bucket_size).floor().min(n as f64 - 1.0) as usize;
        let mut avg_x = 0.0f64;
        let mut avg_y = 0.0f64;
        let mut avg_count = 0usize;
        let rs = next_start.max(1);
        let re = next_end.max(rs + 1);
        for k in rs..re {
            avg_x += points[k].0;
            avg_y += points[k].1;
            avg_count += 1;
        }
        if avg_count == 0 { avg_x = points[end].0; avg_y = points[end].1; avg_count = 1; }
        avg_x /= avg_count as f64;
        avg_y /= avg_count as f64;

        // Select point in current bucket that maximizes triangle area with
        // previous selected point (a) and next bucket average.
        let a_x = points[a].0;
        let a_y = points[a].1;
        let mut max_area = -1.0f64;
        let mut max_idx = start;
        let se = end.max(start + 1);
        for k in start..se {
            // Triangle area via cross product magnitude
            let area = ((a_x - points[k].0) * (avg_y - a_y) - (a_x - avg_x) * (points[k].1 - a_y)).abs();
            if area > max_area {
                max_area = area;
                max_idx = k;
            }
        }
        sampled.push(points[max_idx]);
        a = max_idx;
    }

    // Always include last
    sampled.push(points[n - 1]);
    sampled
}

/// Aggregate OHLC candles into fixed-size buckets of `bucket` width.
/// For each bucket: open=first.open, close=last.close, high=max high, low=min low, t=first.t
pub fn aggregate_ohlc_buckets(data: &[Candle], bucket: usize) -> Vec<Candle> {
    if bucket <= 1 || data.len() <= 2 { return data.to_vec(); }
    let mut out: Vec<Candle> = Vec::new();
    let mut i = 0usize;
    let n = data.len();
    while i < n {
        let j = (i + bucket).min(n);
        let first = data[i];
        let last = data[j - 1];
        let mut low = first.l;
        let mut high = first.h;
        for k in (i + 1)..j {
            low = low.min(data[k].l);
            high = high.max(data[k].h);
        }
        out.push(Candle { t: first.t, o: first.o, h: high, l: low, c: last.c });
        i = j;
    }
    out
}
