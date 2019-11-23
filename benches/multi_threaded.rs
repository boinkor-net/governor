use criterion::{black_box, BenchmarkId, Criterion, Throughput};
use governor::state::keyed::{DashMapStateStore, HashMapStateStore, KeyedStateStore};
use governor::{clock, Quota, RateLimiter};
use nonzero_ext::*;
use std::any::type_name;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

pub fn bench_all(c: &mut Criterion) {
    bench_direct(c);
    bench_keyed::<HashMapStateStore<u32>>(c);
    bench_keyed::<DashMapStateStore<u32>>(c);
}

const THREADS: u32 = 20;

fn bench_direct(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_threaded");
    group.throughput(Throughput::Elements(1));
    group.bench_function("direct", |b| {
        let ms = Duration::from_millis(1);
        let clock = clock::FakeRelativeClock::default();
        let lim = Arc::new(RateLimiter::direct_with_clock(
            Quota::per_second(nonzero!(50u32)),
            &clock,
        ));

        b.iter_custom(|iters| {
            let mut children = vec![];
            let start = Instant::now();
            for _i in 0..THREADS {
                let lim = lim.clone();
                let clock = clock.clone();
                children.push(thread::spawn(move || {
                    for _i in 0..iters {
                        clock.advance(ms);
                        black_box(lim.check().is_ok());
                    }
                }));
            }
            for child in children {
                child.join().unwrap()
            }
            start.elapsed()
        })
    });
    group.finish();
}

fn bench_keyed<M: KeyedStateStore<u32> + Default + Send + Sync + 'static>(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_threaded");

    // We perform 3 checks per thread per iter:
    group.throughput(Throughput::Elements(3));

    group.bench_function(BenchmarkId::new("keyed", type_name::<M>()), |b| {
        let ms = Duration::from_millis(1);
        let clock = clock::FakeRelativeClock::default();
        let state: M = Default::default();
        let lim = Arc::new(RateLimiter::new(
            Quota::per_second(nonzero!(50u32)),
            state,
            &clock,
        ));

        b.iter_custom(|iters| {
            let mut children = vec![];
            let start = Instant::now();
            for _i in 0..THREADS {
                let lim = lim.clone();
                let clock = clock.clone();
                children.push(thread::spawn(move || {
                    for _i in 0..iters {
                        clock.advance(ms);
                        black_box(lim.check_key(&1u32).is_ok());
                        black_box(lim.check_key(&2u32).is_ok());
                        black_box(lim.check_key(&3u32).is_ok());
                    }
                }));
            }
            for child in children {
                child.join().unwrap()
            }
            start.elapsed()
        })
    });
    group.finish();
}
