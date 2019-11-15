use criterion::{black_box, Benchmark, Criterion, Throughput};
use governor::{DirectRateLimiter, Quota};
use nonzero_ext::*;
use std::sync::Arc;
use std::thread;

pub fn bench_all(c: &mut Criterion) {
    bench_direct(c);
}

fn bench_direct(c: &mut Criterion) {
    let id = "multi_threaded/direct";
    let bm = Benchmark::new(id, |b| {
        let mut children = vec![];
        let lim = Arc::new(DirectRateLimiter::new(Quota::per_second(nonzero!(
            1_000_000u32
        ))));

        for _i in 0..19 {
            let lim = lim.clone();
            let mut b = *b;
            children.push(thread::spawn(move || {
                b.iter(|| {
                    black_box(lim.check().is_ok());
                });
            }));
        }
        b.iter(|| {
            black_box(lim.check().is_ok());
        });
        for child in children {
            child.join().unwrap();
        }
    })
    .throughput(Throughput::Elements(1));
    c.bench(id, bm);
}
