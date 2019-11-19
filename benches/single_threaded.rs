use criterion::{black_box, Benchmark, Criterion, Throughput};
use governor::state::keyed::{DashMapStateStore, HashMapStateStore, KeyedStateStore};
use governor::{clock, Quota, RateLimiter};
use nonzero_ext::*;

pub fn bench_all(c: &mut Criterion) {
    bench_direct(c);
    bench_keyed::<HashMapStateStore<u32>>(c, "hashmap");
    bench_keyed::<DashMapStateStore<u32>>(c, "dashmap");
}

fn bench_direct(c: &mut Criterion) {
    let id = "single_threaded/direct";
    let bm = Benchmark::new(id, |b| {
        let rl = RateLimiter::direct(Quota::per_second(nonzero!(1_000_000u32)));
        b.iter(|| {
            black_box(rl.check().is_ok());
        });
    })
    .throughput(Throughput::Elements(1));
    c.bench(id, bm);
}

fn bench_keyed<M: KeyedStateStore<u32> + Default + Send + Sync + 'static>(
    c: &mut Criterion,
    name: &str,
) {
    let id = format!("single_threaded/keyed/{}", name);
    let bm = Benchmark::new(&id, |b| {
        let state: M = Default::default();
        let clock = clock::MonotonicClock::default();
        let rl = RateLimiter::new(Quota::per_second(nonzero!(1_000_000u32)), state, &clock);
        b.iter(|| {
            black_box(rl.check_key(&1u32).is_ok());
        });
    })
    .throughput(Throughput::Elements(1));
    c.bench(&id, bm);
}
