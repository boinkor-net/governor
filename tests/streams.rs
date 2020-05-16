#![cfg(feature = "std")]

use futures::executor::block_on;
use futures::{stream, StreamExt};
use governor::{clock, prelude::*, Quota, RateLimiter};
use nonzero_ext::*;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[test]
fn stream() {
    clock::calibrate_quanta_clock();
    let i = Instant::now();
    let lim = Arc::new(RateLimiter::direct(Quota::per_second(nonzero!(10u32))));
    let mut stream = stream::repeat(()).ratelimit_stream(&lim);

    for _ in 0..10 {
        block_on(stream.next());
    }
    assert!(i.elapsed() <= Duration::from_millis(100));

    block_on(stream.next());
    assert!(i.elapsed() > Duration::from_millis(100));
    assert!(i.elapsed() <= Duration::from_millis(200));

    block_on(stream.next());
    assert!(i.elapsed() > Duration::from_millis(200));
    assert!(i.elapsed() <= Duration::from_millis(300));
}
