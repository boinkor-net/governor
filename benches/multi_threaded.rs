use criterion::{black_box, Benchmark, Criterion, Throughput};
use governor::clock::FakeRelativeClock;
use governor::{DirectRateLimiter, Quota};
use nonzero_ext::*;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub fn bench_all(c: &mut Criterion) {
    bench_direct(c);
}

fn bench_direct(c: &mut Criterion) {
    let id = "multi_threaded/direct";
    let bm = Benchmark::new(id, |b| {
        let mut clock = FakeRelativeClock::new();
        let ms = Duration::from_millis(20);
        let mut children = vec![];
        let lim = Arc::new(DirectRateLimiter::new_with_clock(
            Quota::per_second(nonzero!(5u32)),
            &clock,
        ));

        for _i in 0..19 {
            let lim = lim.clone();
            let mut clock = clock.clone();
            let mut b = *b;
            children.push(thread::spawn(move || {
                b.iter(|| {
                    clock.advance(ms);
                    black_box(lim.check().is_ok());
                });
            }));
        }
        b.iter(|| {
            clock.advance(ms);
            black_box(lim.check().is_ok());
        });
        for child in children {
            child.join().unwrap();
        }
    })
    .throughput(Throughput::Elements(1));
    c.bench(id, bm);
}
