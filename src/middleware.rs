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
pub struct StateSnapshot<P: clock::Reference> {
    start: P,

    /// The "weight" of a single packet in units of time.
    t: Nanos,

    /// The "burst capacity" of the bucket.
    tau: Nanos,

    /// The next time a cell is expected to arrive
    tat: Option<Nanos>,
}

impl<P: clock::Reference> StateSnapshot<P> {
    #[inline]
    pub(crate) fn new(start: P, t: Nanos, tau: Nanos, tat: Option<Nanos>) -> Self {
        Self { start, t, tau, tat }
    }

    /// Returns the quota used to make the rate limiting decision.
    pub fn quota(&self) -> Quota {
        Quota::from_gcra_parameters(self.t, self.tau)
    }

    /// Returns the number of cells that can be let through in
    /// addition to a (possible) positive outcome.
    ///
    /// Returns None if the rate limiting decision was not positive.
    pub fn remaining_burst_capacity(&self) -> Option<u64> {
        self.tat.map(|tat| {
            // at this point we know that we're `tat` nanos after the
            // earliest arrival time, and so are using up some "burst
            // capacity".
            //
            // As one cell has already been used by the positive
            // decision, we're relying on the "round down" behavior of
            // unsigned integer division.
            tat / self.t
        })
    }
}

/// Implements additional behavior when rate-limiting decisions are made.
pub trait RateLimitingMiddleware: fmt::Debug + PartialEq {
    type PositiveOutcome: Sized;

    /// Called when a positive rate-limiting decision is made.
    ///
    /// This function is able to affect the return type of `test` (and
    /// others) in the Ok case: Whatever is returned here is the value
    /// of the Ok result returned from the test functions.
    ///
    /// As arguments, it takes a `key` (the item we were testing for)
    /// and a `when` closure that, if called, returns the time at
    /// which the next cell is expected.
    fn allow<K, P>(key: &K, state: StateSnapshot<P>) -> Self::PositiveOutcome
    where
        P: clock::Reference;

    /// Called when a negative rate-limiting decision is made (the
    /// "not allowed but OK" case).
    fn disallow<K, P: clock::Reference>(
        key: &K,
        state: StateSnapshot<P>,
        not_until: &NotUntil<P, Self>,
    ) where
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
    fn allow<K, P>(_key: &K, _state: StateSnapshot<P>) -> Self::PositiveOutcome
    where
        P: clock::Reference,
    {
    }

    #[inline]
    /// Does nothing.
    fn disallow<K, P: clock::Reference>(
        _key: &K,
        _state: StateSnapshot<P>,
        _not_until: &NotUntil<P, Self>,
    ) where
        Self: Sized,
    {
    }
}
