//! Additional and customizable behavior for rate limiters.
//!
//! Rate-limiting middleware follows the principle that basic
//! rate-limiting should be very cheap, and unless users desire more
//! behavior, they should not pay any extra price.
//!
//! However, those who wish to get additional features should find the
//! means to do so. The middleware module attempts to make this
//! possible.
// TODO: More docs.
//
use core::fmt;

use crate::{clock, nanos::Nanos, NotUntil, Quota};

/// Information about the rate-limiting state used to reach a decision.
#[derive(Clone, PartialEq, Debug)]
pub struct StateSnapshot {
    /// The "weight" of a single packet in units of time.
    t: Nanos,

    /// The "burst capacity" of the bucket.
    tau: Nanos,

    /// The next time a cell is expected to arrive
    tat: Option<Nanos>,
}

impl StateSnapshot {
    #[inline]
    pub(crate) fn new(t: Nanos, tau: Nanos, tat: Option<Nanos>) -> Self {
        Self { t, tau, tat }
    }

    /// Returns the quota used to make the rate limiting decision.
    pub fn quota(&self) -> Quota {
        Quota::from_gcra_parameters(self.t, self.tau)
    }

    /// Returns the number of cells that can be let through in
    /// addition to a (possible) positive outcome.
    ///
    /// Returns None if the rate limiting decision was not positive.
    pub fn remaining_burst_capacity(&self) -> Option<u32> {
        self.tat.map(|tat| {
            // at this point we know that we're `tat` nanos after the
            // earliest arrival time, and so are using up some "burst
            // capacity".
            //
            // As one cell has already been used by the positive
            // decision, we're relying on the "round down" behavior of
            // unsigned integer division.
            self.quota().burst_size().get() + 1 - (tat / self.t) as u32
        })
    }
}

/// Implements additional behavior when rate-limiting decisions are made.
///
/// Besides altering the return value in the positive outcome,
/// middleware is not able to affect the decisions of the rate-limiter
/// in any way: A rate-limiting decision will always be `Ok(...)` or
/// `Err(NotUntil{...})`, but middleware can be set up to alter the
/// return value in the Ok() case.
pub trait RateLimitingMiddleware: fmt::Debug + PartialEq {
    /// The type that's returned by the rate limiter when a cell is allowed.
    ///
    /// By default, rate limiters return `Ok(())`, which does not give
    /// much information. By using custom middleware, users can obtain
    /// more information about the rate limiter state that was used to
    /// come to a decision. That state can then be used to pass
    /// information downstream about, e.g. how much burst capacity is
    /// remaining.
    type PositiveOutcome: Sized;

    /// Called when a positive rate-limiting decision is made.
    ///
    /// This function is able to affect the return type of
    /// [RateLimiter.check](../struct.RateLimiter.html#method.check)
    /// (and others) in the Ok case: Whatever is returned here is the
    /// value of the Ok result returned from the check functions.
    ///
    /// The function is passed a snapshot of the rate-limiting state
    /// updated to *after* the decision was reached: E.g., if there
    /// was one cell left in the burst capacity before the decision
    /// was reached, the [`StateSnapshot::remaining_burst_capacity`]
    /// method will return 0.
    fn allow<K>(key: &K, state: StateSnapshot) -> Self::PositiveOutcome;

    /// Called when a negative rate-limiting decision is made (the
    /// "not allowed but OK" case).
    ///
    /// This method does not affect anything the rate limiter returns
    /// to user code, but can be used to track counts for
    /// rate-limiting outcomes on a key-by-key basis.
    fn disallow<K, P>(key: &K, state: StateSnapshot, not_until: &NotUntil<P, Self>)
    where
        Self: Sized,
        P: clock::Reference;
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
    fn allow<K>(_key: &K, _state: StateSnapshot) -> Self::PositiveOutcome {}

    #[inline]
    /// Does nothing.
    fn disallow<K, P: clock::Reference>(
        _key: &K,
        _state: StateSnapshot,
        _not_until: &NotUntil<P, Self>,
    ) where
        Self: Sized,
    {
    }
}

/// Middleware that returns the state of the rate limiter if a
/// positive decision is reached.
#[derive(PartialEq, Debug)]
pub struct StateInformationMiddleware {}

impl RateLimitingMiddleware for StateInformationMiddleware {
    /// The state snapshot returned from the limiter.
    type PositiveOutcome = StateSnapshot;

    fn allow<K>(_key: &K, state: StateSnapshot) -> Self::PositiveOutcome {
        state
    }

    fn disallow<K, P>(_key: &K, _state: StateSnapshot, _not_until: &NotUntil<P, Self>)
    where
        Self: Sized,
        P: clock::Reference,
    {
    }
}
