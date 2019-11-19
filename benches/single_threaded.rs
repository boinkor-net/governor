use criterion::{black_box, Benchmark, Criterion, Throughput};
use governor::state::keyed::{DashMapStateStore, HashMapStateStore, KeyedStateStore};
use governor::{clock, Quota, RateLimiter};
use nonzero_ext::*;
use std::time::Duration;

pub fn bench_all(c: &mut Criterion) {
    bench_direct(c);
    bench_keyed::<HashMapStateStore<u32>>(c, "hashmap");
    bench_keyed::<DashMapStateStore<u32>>(c, "dashmap");
}

fn bench_direct(c: &mut Criterion) {
    let id = "single_threaded/direct";
    let bm = Benchmark::new(id, |b| {
        let clock = clock::FakeRelativeClock::default();
        let step = Duration::from_millis(20);
        let rl = RateLimiter::direct_with_clock(Quota::per_second(nonzero!(50u32)), &clock);
        b.iter(|| {
            clock.advance(step);
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
        let clock = clock::FakeRelativeClock::default();
        let step = Duration::from_millis(20);
        let rl = RateLimiter::new(Quota::per_second(nonzero!(50u32)), state, &clock);
        b.iter(|| {
            clock.advance(step);
            black_box(rl.check_key(&1u32).is_ok());
        });
    })
    .throughput(Throughput::Elements(1));
    c.bench(&id, bm);
}
