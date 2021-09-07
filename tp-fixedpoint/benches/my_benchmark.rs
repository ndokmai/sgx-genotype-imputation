use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use tp_fixedpoint::TpFixed64;

fn bench_fibs(c: &mut Criterion) {
    let nrounds = 16;
    let a = ndarray::Array1::from_shape_fn(nrounds, |i| {
        TpFixed64::<20>::leaky_from_f32(1. / (i + 2) as f32)
    });
    let mut group = c.benchmark_group("LogLtOne");
    group.bench_function(BenchmarkId::new("Single", 0), |b| {
        b.iter(|| {
            let mut results = Vec::with_capacity(nrounds);
            for i in &a {
                results.push(i.log_lt_one());
            }
        })
    });
    group.bench_function(BenchmarkId::new("Batch", 0), |b| {
        b.iter(|| TpFixed64::log_lt_one_batch(a.to_owned()))
    });
    group.finish();
}

criterion_group!(benches, bench_fibs);
criterion_main!(benches);
