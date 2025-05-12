//! Additional, customizable behavior for rate limiters.
//!
//! Rate-limiting middleware follows the principle that basic
//! rate-limiting should be very cheap, and unless users desire more
//! behavior, they should not pay any extra price.
//!
//! However, if you do desire more information about what the
//! rate-limiter does (or the ability to install hooks in the
//! decision-making process), you can. The [`RateLimitingMiddleware`]
//! trait in this module allows you to customize:
//!
//! * Any additional code that gets run when a rate-limiting decision is made.
//! * What value is returned in the positive or negative case.
//!
//! Writing middleware does **not** let you override rate-limiting
//! decisions: They remain either positive (returning `Ok`) or negative
//! (returning `Err`). However, you can override the values returned
//! inside the Result for either decision.
//!
//! This crate ships two middlewares (named after their behavior in the
//! positive outcome):
//!
//! * The cheapest still-useful one, [`NoOpMiddleware`], named after its
//!   behavior in the positive case. In the positive case it returns
//!   `Ok(())`; in the negative case, `Err(`[`NotUntil`]`)`.
//!
//! * A more informative middleware, [`StateInformationMiddleware`], which
//!   returns `Ok(`[`StateSnapshot`]`)`, or
//!   `Err(`[`NotUntil`]`)`.
//!
//! ## Using a custom middleware
//!
//! Middlewares are attached to the
//! [`RateLimiter`][crate::RateLimiter] at construction time using
//! [`RateLimiter::with_middleware`][crate::RateLimiter::with_middleware]:
//!
//! ```rust
//! # #[cfg(feature = "std")]
//! # fn main () {
//! # use nonzero_ext::nonzero;
//! use governor::{RateLimiter, Quota, middleware::StateInformationMiddleware};
//! let lim = RateLimiter::direct(Quota::per_hour(nonzero!(1_u32)))
//!     .with_middleware::<StateInformationMiddleware<_>>();
//!
//! // A positive outcome with additional information:
//! assert!(
//!     lim.check()
//!         // Here we receive an Ok(StateSnapshot):
//!         .map(|outcome| assert_eq!(outcome.remaining_burst_capacity(), 0))
//!         .is_ok()
//! );
//!
//! // The negative case:
//! assert!(
//!     lim.check()
//!         // Here we receive Err(NotUntil):
//!         .map_err(|outcome| assert_eq!(outcome.quota().burst_size().get(), 1))
//!         .is_err()
//! );
//! # }
//! # #[cfg(not(feature = "std"))]
//! # fn main() {}
//! ```
//!
//! You can define your own middleware by `impl`ing [`RateLimitingMiddleware`].
use core::fmt;
use core::{cmp, marker::PhantomData};

use crate::{clock, nanos::Nanos, NotUntil, Quota};

/// Information about the rate-limiting state used to reach a decision.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StateSnapshot {
    /// The "weight" of a single packet in units of time.
    t: Nanos,

    /// The "tolerance" of the bucket.
    ///
    /// The total "burst capacity" of the bucket is `t + tau`.
    tau: Nanos,

    /// The time at which the measurement was taken.
    pub(crate) time_of_measurement: Nanos,

    /// The next time a cell is expected to arrive
    pub(crate) tat: Nanos,
}

impl StateSnapshot {
    #[inline]
    pub(crate) fn new(t: Nanos, tau: Nanos, time_of_measurement: Nanos, tat: Nanos) -> Self {
        Self {
            t,
            tau,
            time_of_measurement,
            tat,
        }
    }

    /// Returns the quota used to make the rate limiting decision.
    pub fn quota(&self) -> Quota {
        Quota::from_gcra_parameters(self.t, self.tau)
    }

    /// Returns the number of cells that can be let through in
    /// addition to a (possible) positive outcome.
    ///
    /// If this state snapshot is based on a negative rate limiting
    /// outcome, this method returns 0.
    pub fn remaining_burst_capacity(&self) -> u32 {
        let t0 = self.time_of_measurement;
        (cmp::min(
            (t0 + self.tau + self.t).saturating_sub(self.tat).as_u64(),
            (self.tau + self.t).as_u64(),
        ) / self.t.as_u64()) as u32
    }
}

/// Defines the behavior and return values of rate limiting decisions.
///
/// While the rate limiter defines whether a decision is positive, the
/// middleware defines what additional values (other than `Ok` or `Err`)
/// are returned from the [`RateLimiter`][crate::RateLimiter]'s check methods.
///
/// The default middleware in this crate is [`NoOpMiddleware`] (which does
/// nothing in the positive case and returns [`NotUntil`] in the
/// negative) - so it does only the smallest amount of work it needs to do
/// in order to be useful to users.
///
/// Other middleware gets to adjust these trade-offs: The pre-made
/// [`StateInformationMiddleware`] returns quota and burst capacity
/// information, while custom middleware could return a set of HTTP
/// headers or increment counters per each rate limiter key's decision.
///
/// # Defining your own middleware
///
/// Here's an example of a rate limiting middleware that does no
/// computations at all on positive and negative outcomes: All the
/// information that a caller will receive is that a request should be
/// allowed or disallowed. This can allow for faster negative outcome
/// handling, and is useful if you don't need to tell users when they
/// can try again (or anything at all about their rate limiting
/// status).
///
/// ```rust
/// # use std::num::NonZeroU32;
/// # use nonzero_ext::*;
/// use governor::{middleware::{RateLimitingMiddleware, StateSnapshot},
///                Quota, RateLimiter, clock::Reference, state::direct::NotKeyed};
/// # #[cfg(feature = "std")]
/// # fn main () {
/// #[derive(Debug)]
/// struct NullMiddleware;
///
/// impl<P: Reference> RateLimitingMiddleware<P> for NullMiddleware {
///     type PositiveOutcome = ();
///     type NegativeOutcome = ();
///     type Key = NotKeyed;
///
///     fn allow(_key: &Self::Key, _state: impl Into<StateSnapshot>) -> Self::PositiveOutcome {}
///     fn disallow(_: &Self::Key, _: impl Into<StateSnapshot>, _: P) -> Self::NegativeOutcome {}
/// }
///
/// let lim = RateLimiter::direct(Quota::per_hour(nonzero!(1_u32)))
///     .with_middleware::<NullMiddleware>();
///
/// assert_eq!(lim.check(), Ok(()));
/// assert_eq!(lim.check(), Err(()));
/// # }
/// # #[cfg(not(feature = "std"))]
/// # fn main() {}
/// ```
pub trait RateLimitingMiddleware<P: clock::Reference>: fmt::Debug {
    /// The type that's returned by the rate limiter when a cell is allowed.
    ///
    /// By default, rate limiters return `Ok(())`, which does not give
    /// much information. By using custom middleware, users can obtain
    /// more information about the rate limiter state that was used to
    /// come to a decision. That state can then be used to pass
    /// information downstream about, e.g. how much burst capacity is
    /// remaining.
    type PositiveOutcome: Sized;

    /// The type that's returned by the rate limiter when a cell is *not* allowed.
    ///
    /// By default, rate limiters return `Err(NotUntil)`, which
    /// allows interrogating the minimum amount of time to wait until
    /// a client can expect to have a cell allowed again.
    type NegativeOutcome: Sized;

    /// The type of key used by the rate limiter.
    type Key: Sized;

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
    fn allow(key: &Self::Key, state: impl Into<StateSnapshot>) -> Self::PositiveOutcome;

    /// Called when a negative rate-limiting decision is made (the
    /// "not allowed but OK" case).
    ///
    /// This method returns whatever value is returned inside the
    /// `Err` variant a [`RateLimiter`][crate::RateLimiter]'s check
    /// method returns.
    fn disallow(
        key: &Self::Key,
        limiter: impl Into<StateSnapshot>,
        start_time: P,
    ) -> Self::NegativeOutcome;
}

/// A middleware that does nothing and returns `()` in the positive outcome.
pub struct NoOpMiddleware<K, P: clock::Reference = <clock::DefaultClock as clock::Clock>::Instant> {
    phantom: PhantomData<(K, P)>,
}

impl<K, P: clock::Reference> core::fmt::Debug for NoOpMiddleware<K, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NoOpMiddleware")
    }
}

impl<K, P: clock::Reference> RateLimitingMiddleware<P> for NoOpMiddleware<K, P> {
    /// By default, rate limiters return nothing other than an
    /// indicator that the element should be let through.
    type PositiveOutcome = ();

    type NegativeOutcome = NotUntil<P>;

    type Key = K;

    #[inline]
    /// Returns `()` and has no side-effects.
    fn allow(_key: &K, _state: impl Into<StateSnapshot>) -> Self::PositiveOutcome {}

    #[inline]
    /// Returns the error indicating what
    fn disallow(_key: &K, state: impl Into<StateSnapshot>, start_time: P) -> Self::NegativeOutcome {
        NotUntil::new(state.into(), start_time)
    }
}

/// Middleware that returns the state of the rate limiter if a
/// positive decision is reached.
pub struct StateInformationMiddleware<K> {
    phantom: PhantomData<K>,
}

impl<K> StateInformationMiddleware<K> {
    #[allow(dead_code)]
    pub(self) fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<K, P: clock::Reference> RateLimitingMiddleware<P> for StateInformationMiddleware<K> {
    /// The state snapshot returned from the limiter.
    type PositiveOutcome = StateSnapshot;

    type NegativeOutcome = NotUntil<P>;

    type Key = K;

    fn allow(_key: &K, state: impl Into<StateSnapshot>) -> Self::PositiveOutcome {
        state.into()
    }

    fn disallow(_key: &K, state: impl Into<StateSnapshot>, start_time: P) -> Self::NegativeOutcome {
        NotUntil::new(state.into(), start_time)
    }
}

impl<K> core::fmt::Debug for StateInformationMiddleware<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StateInformationMiddleware")
    }
}

#[cfg(all(feature = "std", test))]
mod test {
    use std::time::Duration;

    use super::*;

    #[test]
    fn middleware_impl_derives() {
        assert_eq!(
            format!("{:?}", StateInformationMiddleware::<()>::new()),
            "StateInformationMiddleware"
        );
        assert_eq!(
            format!(
                "{:?}",
                NoOpMiddleware {
                    phantom: PhantomData::<((), Duration)>,
                }
            ),
            "NoOpMiddleware"
        );
    }
}
