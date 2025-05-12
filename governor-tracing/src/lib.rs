use std::{fmt::Debug, marker::PhantomData};

use governor::{clock, middleware::RateLimitingMiddleware};
use tracing::instrument;

/// Middleware that emits `TRACE` level events whenever a measurement and outcome on the ratelimiter happens.
#[derive(Debug)]
pub struct TracingMiddleware<K: Debug, P: clock::Reference, I: RateLimitingMiddleware<P, Key = K>> {
    _phantom: PhantomData<(K, I, P)>,
}

impl<K: Debug, P: clock::Reference, I: RateLimitingMiddleware<P, Key = K>> RateLimitingMiddleware<P>
    for TracingMiddleware<K, P, I>
where
    I::PositiveOutcome: Debug,
    I::NegativeOutcome: Debug,
{
    type PositiveOutcome = I::PositiveOutcome;

    type NegativeOutcome = I::NegativeOutcome;

    type Key = K;

    #[instrument(ret, skip(state))]
    fn allow(
        key: &Self::Key,
        state: impl Into<governor::middleware::StateSnapshot>,
    ) -> Self::PositiveOutcome {
        I::allow(key, state)
    }

    #[instrument(ret, skip(limiter))]
    fn disallow(
        key: &Self::Key,
        limiter: impl Into<governor::middleware::StateSnapshot>,
        start_time: P,
    ) -> Self::NegativeOutcome {
        I::disallow(key, limiter, start_time)
    }
}
