use criterion::{black_box, Benchmark, Criterion, Throughput};
use governor::{DirectRateLimiter, Quota};
use nonzero_ext::*;

pub fn bench_all(c: &mut Criterion) {
    bench_direct(c);
}

fn bench_direct(c: &mut Criterion) {
    let id = "single_threaded/direct";
    let bm = Benchmark::new(id, |b| {
        let rl = DirectRateLimiter::new(Quota::per_second(nonzero!(1_000_000u32)));
        b.iter(|| {
            black_box(rl.check().is_ok());
        });
    })
    .throughput(Throughput::Elements(1));
    c.bench(id, bm);
}
