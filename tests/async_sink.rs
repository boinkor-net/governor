#![cfg(feature = "std")]

use futures::executor::block_on;
use futures::SinkExt as _;
use governor::{DirectRateLimiter, Quota, SinkExt};
use nonzero_ext::*;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[test]
fn sink() {
    let i = Instant::now();
    let lim = Arc::new(DirectRateLimiter::new(Quota::per_second(nonzero!(10u32))));
    let mut sink = Vec::new().ratelimit_sink(&lim);

    for _ in 0..10 {
        block_on(sink.send(())).unwrap();
    }
    assert!(
        i.elapsed() <= Duration::from_millis(100),
        "elapsed: {:?}",
        i.elapsed()
    );

    block_on(sink.send(())).unwrap();
    assert!(
        i.elapsed() > Duration::from_millis(100),
        "elapsed: {:?}",
        i.elapsed()
    );
    assert!(
        i.elapsed() <= Duration::from_millis(200),
        "elapsed: {:?}",
        i.elapsed()
    );

    block_on(sink.send(())).unwrap();
    assert!(i.elapsed() > Duration::from_millis(200));
    assert!(i.elapsed() <= Duration::from_millis(300));
}
