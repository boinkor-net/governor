#![cfg(feature = "std")]

use all_asserts::*;
use futures::SinkExt;
use governor::{prelude::*, Quota, RateLimiter};
use instant::Instant;
use nonzero_ext::*;
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "jitter")]
use governor::Jitter;

#[cfg(not(target_arch = "wasm32"))]
use futures::executor::block_on;

#[cfg(target_arch = "wasm32")]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[macro_use]
mod macros;

tests! {
    fn sink() {
        let lim = Arc::new(RateLimiter::direct(Quota::per_second(nonzero!(10u32))));
        let mut sink = Vec::new().ratelimit_sink(&lim);
        let i = Instant::now();

        for _ in 0..10 {
            wait!(sink.send(())).unwrap();
        }
        assert_lt!(i.elapsed(), Duration::from_millis(100));

        wait!(sink.send(())).unwrap();
        assert_range!((100..=200), i.elapsed().as_millis());

        wait!(sink.send(())).unwrap();
        assert_range!((200..=300), i.elapsed().as_millis());

        let result = sink.get_ref();
        assert_eq!(result.len(), 12);
        assert!(result.into_iter().all(|&elt| elt == ()));
    }

    fn auxilliary_sink_methods() {
        let lim = Arc::new(RateLimiter::direct(Quota::per_second(nonzero!(10u32))));
        // TODO: use futures_ringbuf to build a Sink-that-is-a-Stream and
        // use it as both
        // (https://github.com/najamelan/futures_ringbuf/issues/5)
        let mut sink = Vec::<u8>::new().ratelimit_sink(&lim);

        // Closes the underlying sink:
        assert!(wait!(sink.close()).is_ok());
    }

    #[cfg(feature = "jitter")]
    fn sink_with_jitter() {
        let lim = Arc::new(RateLimiter::direct(Quota::per_second(nonzero!(10u32))));
        let mut sink =
            Vec::new().ratelimit_sink_with_jitter(&lim, Jitter::up_to(Duration::from_nanos(1)));
        let i = Instant::now();

        for _ in 0..10 {
            wait!(sink.send(())).unwrap();
        }
        assert_le!(i.elapsed(), Duration::from_millis(100),);

        wait!(sink.send(())).unwrap();
        assert_range!((100..=200), i.elapsed().as_millis());

        wait!(sink.send(())).unwrap();
        assert_range!((200..=300), i.elapsed().as_millis());

        let result = sink.into_inner();
        assert_eq!(result.len(), 12);
        assert!(result.into_iter().all(|elt| elt == ()));
    }
}
