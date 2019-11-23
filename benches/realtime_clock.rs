//! Benchmarks to determine the performance of measuring against the default real-time clock.
//!
//! The two functions in here measure the throughput against a rate-limiter that mostly allows
//! (allowing max_value of `u32` per nanosecond), and one that mostly denies (allowing only one
//! per hour).

use criterion::{black_box, Benchmark, Criterion, Throughput};
use governor::{Quota, RateLimiter};
use nonzero_ext::*;
use std::time::Duration;

pub fn bench_all(c: &mut Criterion) {
    bench_mostly_allow(c);
    bench_mostly_deny(c);
}

fn bench_mostly_allow(c: &mut Criterion) {
    let id = "realtime_clock/mostly_allow";
    let bm = Benchmark::new(id, |b| {
        let rl = RateLimiter::direct(
            Quota::new(nonzero!(u32::max_value()), Duration::from_nanos(1)).unwrap(),
        );
        b.iter(|| {
            black_box(rl.check().is_ok());
        });
    })
    .throughput(Throughput::Elements(1));
    c.bench(id, bm);
}

fn bench_mostly_deny(c: &mut Criterion) {
    let id = "realtime_clock/mostly_deny";
    let bm = Benchmark::new(id, |b| {
        let rl = RateLimiter::direct(Quota::per_hour(nonzero!(1u32)));
        b.iter(|| {
            black_box(rl.check().is_ok());
        });
    })
    .throughput(Throughput::Elements(1));
    c.bench(id, bm);
}
