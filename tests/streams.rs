#![cfg(feature = "std")]

use futures::{stream, StreamExt};
use governor::{prelude::*, Quota, RateLimiter};
use instant::Instant;
use nonzero_ext::*;
use std::sync::Arc;
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use futures::executor::block_on;

#[cfg(target_arch = "wasm32")]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[macro_use]
mod macros;

tests! {
    fn stream() {
        let lim = Arc::new(RateLimiter::direct(Quota::per_second(nonzero!(10u32))));
        let mut stream = stream::repeat(()).ratelimit_stream(&lim);
        let i = Instant::now();

        for _ in 0..10 {
            wait!(stream.next());
        }
        assert!(i.elapsed() <= Duration::from_millis(100));

        wait!(stream.next());
        assert!(i.elapsed() >= Duration::from_millis(100));
        assert!(i.elapsed() <= Duration::from_millis(200));

        wait!(stream.next());
        assert!(i.elapsed() >= Duration::from_millis(200));
        assert!(i.elapsed() <= Duration::from_millis(300));
    }
}
