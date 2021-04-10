use core::fmt;

use crate::{clock, nanos::Nanos, NotUntil};

/// Implements additional behavior when rate-limiting decisions are made.
pub trait RateLimitingMiddleware: fmt::Debug + PartialEq {
    type PositiveOutcome: Sized;

    /// Called when a positive rate-limiting decision is made.
    ///
    /// This function is able to affect the return type of `test` (and
    /// others) in the Ok case: Whatever is returned here is the value
    /// of the Ok result returned from the test functions.
    fn allow<K, P: clock::Reference>(
        key: &K,
        decision_at: P,
        next_tat: Nanos,
    ) -> Self::PositiveOutcome;

    /// Called when a negative rate-limiting decision is made (the
    /// "not allowed but OK" case).
    fn disallow<K, P: clock::Reference>(key: &K, decision_at: P, not_until: &NotUntil<P, Self>)
    where
        Self: Sized;
}

#[derive(PartialEq, Debug)]
/// A middleware that does nothing and returns `()` in the positive outcome.
pub struct NoOpMiddleware {}

impl RateLimitingMiddleware for NoOpMiddleware {
    /// By default, rate limiters return nothing other than an
    /// indicator that the element should be let through.
    type PositiveOutcome = ();

    #[inline]
    /// Returns `()` and has no side-effects.
    fn allow<K, P: clock::Reference>(
        _key: &K,
        _decision_at: P,
        _next_tat: Nanos,
    ) -> Self::PositiveOutcome {
    }

    #[inline]
    /// Does nothing.
    fn disallow<K, P: clock::Reference>(_key: &K, _decision_at: P, _not_until: &NotUntil<P, Self>)
    where
        Self: Sized,
    {
    }
}
