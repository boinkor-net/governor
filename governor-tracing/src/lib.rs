use std::{fmt::Debug, marker::PhantomData};

use governor::{clock, middleware::RateLimitingMiddleware};
use tracing::{Level, event, span};

/// Middleware that emits `TRACE` level events whenever a measurement and outcome on the ratelimiter happens.
pub struct TracingMiddleware<K, P: clock::Reference, I: RateLimitingMiddleware<P, Key = K>> {
    _phantom: PhantomData<(K, I, P)>,
}

impl<K, P: clock::Reference, I: RateLimitingMiddleware<P, Key = K>> core::fmt::Debug
    for TracingMiddleware<K, P, I>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TracingMiddleware").finish()
    }
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

    fn allow(
        key: &Self::Key,
        state: impl Into<governor::middleware::StateSnapshot>,
    ) -> Self::PositiveOutcome {
        let state = state.into();
        let span = span!(Level::TRACE, "allow", ?key, ?state);
        let _enter = span.enter();
        let result = I::allow(key, state);
        event!(Level::TRACE, ?result);
        result
    }

    fn disallow(
        key: &Self::Key,
        limiter: impl Into<governor::middleware::StateSnapshot>,
        start_time: P,
    ) -> Self::NegativeOutcome {
        let limiter = limiter.into();
        let span = span!(Level::TRACE, "disallow", ?key);
        let _enter = span.enter();
        let result = I::disallow(key, limiter, start_time);
        event!(Level::TRACE, ?result);
        result
    }
}
