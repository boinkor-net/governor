#![cfg(feature = "std")]

use futures::executor::block_on;
use futures::SinkExt;
use governor::{prelude::*, Jitter, Quota, RateLimiter};
use nonzero_ext::*;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[test]
fn sink() {
    let i = Instant::now();
    let lim = Arc::new(RateLimiter::direct(Quota::per_second(nonzero!(10u32))));
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

    let result = sink.get_ref();
    assert_eq!(result.len(), 12);
    assert!(result.into_iter().all(|&elt| elt == ()));
}

#[test]
fn auxilliary_sink_methods() {
    let lim = Arc::new(RateLimiter::direct(Quota::per_second(nonzero!(10u32))));
    // TODO: use futures_ringbuf to build a Sink-that-is-a-Stream and
    // use it as both
    // (https://github.com/najamelan/futures_ringbuf/issues/5)
    let mut sink = Vec::<u8>::new().ratelimit_sink(&lim);

    // Closes the underlying sink:
    assert!(block_on(sink.close()).is_ok());
}

#[cfg_attr(feature = "jitter", test)]
fn sink_with_jitter() {
    let i = Instant::now();
    let lim = Arc::new(RateLimiter::direct(Quota::per_second(nonzero!(10u32))));
    let mut sink =
        Vec::new().ratelimit_sink_with_jitter(&lim, Jitter::up_to(Duration::from_nanos(1)));

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

    let result = sink.into_inner();
    assert_eq!(result.len(), 12);
    assert!(result.into_iter().all(|elt| elt == ()));
}
