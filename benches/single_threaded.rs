use criterion::{black_box, Benchmark, Criterion, Throughput};
use governor::clock::FakeRelativeClock;
use governor::{DirectRateLimiter, Quota};
use nonzero_ext::*;
use std::time::Duration;

pub fn bench_all(c: &mut Criterion) {
    bench_direct(c);
}

fn bench_direct(c: &mut Criterion) {
    let id = "single_threaded/direct";
    let bm = Benchmark::new(id, |b| {
        let mut clock = FakeRelativeClock::new();
        let ms = Duration::from_millis(20);
        let rl = DirectRateLimiter::new_with_clock(Quota::per_second(nonzero!(5u32)), &clock);
        b.iter(|| {
            clock.advance(ms);
            black_box(rl.check().is_ok());
        });
    })
    .throughput(Throughput::Elements(1));
    c.bench(id, bm);
}
