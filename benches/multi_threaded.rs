use criterion::{black_box, Benchmark, Criterion, Throughput};
use governor::state::keyed::{DashMapStateStore, HashMapStateStore, KeyedStateStore};
use governor::{clock, Quota, RateLimiter};
use nonzero_ext::*;
use std::sync::Arc;
use std::thread;

pub fn bench_all(c: &mut Criterion) {
    bench_direct(c);
    bench_keyed::<HashMapStateStore<u32>>(c, "hashmap");
    bench_keyed::<DashMapStateStore<u32>>(c, "dashmap");
}

fn bench_direct(c: &mut Criterion) {
    let id = "multi_threaded/direct";
    let bm = Benchmark::new(id, |b| {
        let mut children = vec![];
        let lim = Arc::new(RateLimiter::direct(Quota::per_second(nonzero!(
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

fn bench_keyed<M: KeyedStateStore<u32> + Default + Send + Sync + 'static>(
    c: &mut Criterion,
    name: &str,
) {
    let id = format!("multi_threaded/keyed/{}", name);
    let bm = Benchmark::new(&id, |b| {
        let mut children = vec![];
        let state: M = Default::default();
        let clock = clock::MonotonicClock::default();
        let lim = Arc::new(RateLimiter::new(
            Quota::per_second(nonzero!(1_000_000u32)),
            state,
            &clock,
        ));

        for _i in 0..19 {
            let lim = lim.clone();
            let mut b = *b;
            children.push(thread::spawn(move || {
                b.iter(|| {
                    black_box(lim.check_key(&1u32).is_ok());
                    black_box(lim.check_key(&2u32).is_ok());
                    black_box(lim.check_key(&3u32).is_ok());
                });
            }));
        }
        b.iter(|| {
            black_box(lim.check_key(&1u32).is_ok());
            black_box(lim.check_key(&2u32).is_ok());
            black_box(lim.check_key(&3u32).is_ok());
        });
        for child in children {
            child.join().unwrap();
        }
    })
    .throughput(Throughput::Elements(3));
    c.bench(&id, bm);
}
