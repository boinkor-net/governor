#![cfg(feature = "std")]

use futures::executor::block_on;
use governor::{DirectRateLimiter, Quota};
use more_asserts::*;
use nonzero_ext::*;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn pauses() {
    let i = Instant::now();
    let lim = DirectRateLimiter::new(Quota::per_second(nonzero!(10u32)));

    // exhaust the limiter:
    loop {
        if lim.check().is_err() {
            break;
        }
    }

    block_on(lim.until_ready());
    assert_ge!(i.elapsed(), Duration::from_millis(100));
}

#[test]
fn proceeds() {
    let i = Instant::now();
    let lim = DirectRateLimiter::new(Quota::per_second(nonzero!(10u32)));

    block_on(lim.until_ready());
    assert_le!(i.elapsed(), Duration::from_millis(100));
}

#[test]
fn multiple() {
    let i = Instant::now();
    let lim = Arc::new(DirectRateLimiter::new(Quota::per_second(nonzero!(10u32))));
    let mut children = vec![];

    for _i in 0..20 {
        let lim = lim.clone();
        children.push(thread::spawn(move || {
            block_on(lim.until_ready());
        }));
    }
    for child in children {
        child.join().unwrap();
    }
    // by now we've waited for, on average, 10ms; but sometimes the
    // test finishes early; let's assume it takes at least 8ms:
    let elapsed = i.elapsed();
    assert_ge!(
        elapsed,
        Duration::from_millis(8),
        "Expected to wait some time, but waited: {:?}",
        elapsed
    );
}
