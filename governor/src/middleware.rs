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
//!     .with_middleware::<StateInformationMiddleware>();
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
use crate::gcra::Gcra;
use crate::InsufficientCapacity;
use crate::{clock, gcra::StateSnapshot, NotUntil};
use core::fmt;
use core::marker::PhantomData;

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
/// use governor::{middleware::{RateLimitingMiddleware},
///                gcra::StateSnapshot,
///                Quota, RateLimiter, clock::Reference};
/// # #[cfg(feature = "std")]
/// # fn main () {
/// #[derive(Debug, Default)]
/// struct NullMiddleware;
///
/// impl<K, P: Reference> RateLimitingMiddleware<K, P> for NullMiddleware {
///     type PositiveOutcome = ();
///     type NegativeOutcome = ();
///
///     fn allow(_key: &K, _state: impl Into<StateSnapshot>) -> Self::PositiveOutcome {}
///     fn disallow(_: &K, _: impl Into<StateSnapshot>, _: P) -> Self::NegativeOutcome {}
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
pub trait RateLimitingMiddleware<K, P: clock::Reference>: fmt::Debug {
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
    fn allow(key: &K, state: impl Into<StateSnapshot>) -> Self::PositiveOutcome;

    /// Called when a negative rate-limiting decision is made (the
    /// "not allowed but OK" case).
    ///
    /// This method returns whatever value is returned inside the
    /// `Err` variant a [`RateLimiter`][crate::RateLimiter]'s check
    /// method returns.
    fn disallow(key: &K, limiter: impl Into<StateSnapshot>, start_time: P)
        -> Self::NegativeOutcome;

    /// Called before a rate-limiting decision is made for a given key
    ///
    /// This function is designed to allow per-key quotas.
    ///
    /// Since it makes no sense for a direct RateLimiter, it is ignored in that case.
    fn check_quota(
        &self,
        _key: &K,
        _f: &dyn Fn(&Gcra) -> Result<Self::PositiveOutcome, Self::NegativeOutcome>,
    ) -> Option<Result<Self::PositiveOutcome, Self::NegativeOutcome>> {
        None
    }

    /// Tests whether multiple cells could be accomodated
    ///
    /// This function is designed to allow per-key quotas. Since it makes no sense for a direct
    /// RateLimiter, it is ignored in that case.
    fn check_quota_n(
        &self,
        _key: &K,
        _f: &dyn Fn(
            &Gcra,
        ) -> Result<
            Result<Self::PositiveOutcome, Self::NegativeOutcome>,
            InsufficientCapacity,
        >,
    ) -> Option<Result<Result<Self::PositiveOutcome, Self::NegativeOutcome>, InsufficientCapacity>>
    {
        None
    }
}

/// A middleware that does nothing and returns `()` in the positive outcome.
pub struct NoOpMiddleware<P: clock::Reference = <clock::DefaultClock as clock::Clock>::Instant> {
    phantom: PhantomData<P>,
}

impl<P: clock::Reference> core::default::Default for NoOpMiddleware<P> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<P: clock::Reference> core::fmt::Debug for NoOpMiddleware<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NoOpMiddleware")
    }
}

impl<K, P: clock::Reference> RateLimitingMiddleware<K, P> for NoOpMiddleware<P> {
    /// By default, rate limiters return nothing other than an
    /// indicator that the element should be let through.
    type PositiveOutcome = ();

    type NegativeOutcome = NotUntil<P>;

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
#[derive(Debug, Default)]
pub struct StateInformationMiddleware;

impl<K, P: clock::Reference> RateLimitingMiddleware<K, P> for StateInformationMiddleware {
    /// The state snapshot returned from the limiter.
    type PositiveOutcome = StateSnapshot;

    type NegativeOutcome = NotUntil<P>;

    fn allow(_key: &K, state: impl Into<StateSnapshot>) -> Self::PositiveOutcome {
        state.into()
    }

    fn disallow(_key: &K, state: impl Into<StateSnapshot>, start_time: P) -> Self::NegativeOutcome {
        NotUntil::new(state.into(), start_time)
    }
}

#[cfg(all(feature = "std", test))]
mod test {
    #[cfg(all(feature = "std"))]
    use std::collections::HashMap;
    use std::time::Duration;

    use super::*;

    #[test]
    fn middleware_impl_derives() {
        assert_eq!(
            format!("{StateInformationMiddleware:?}"),
            "StateInformationMiddleware"
        );
        assert_eq!(
            format!(
                "{:?}",
                NoOpMiddleware {
                    phantom: PhantomData::<Duration>,
                }
            ),
            "NoOpMiddleware"
        );
    }

    #[test]
    fn ensure_extant_middleware_gives_no_quota() {
        assert_eq!(
            NoOpMiddleware {
                phantom: PhantomData::<Duration>,
            }
            .check_quota(&111, &|_| unimplemented!()),
            None
        );
        let simw = StateInformationMiddleware;
        assert_eq!(
            <StateInformationMiddleware as RateLimitingMiddleware<i32, Duration>>::check_quota(
                &simw,
                &111,
                &|_| unimplemented!()
            ),
            None
        );
    }

    #[cfg(all(feature = "std", feature = "dashmap"))]
    #[derive(Debug)]
    pub struct KeyedMw<K: Eq + core::hash::Hash> {
        keys: HashMap<K, Gcra>,
    }

    #[cfg(all(feature = "std", feature = "dashmap"))]
    impl<K> KeyedMw<K>
    where
        K: Eq + core::hash::Hash,
    {
        pub fn new<I>(quotas: I) -> Self
        where
            I: Iterator<Item = (K, crate::Quota)>,
        {
            use std::iter::FromIterator;

            Self {
                keys: HashMap::from_iter(quotas.map(|(k, q)| (k, Gcra::new(q)))),
            }
        }
    }

    #[cfg(all(feature = "std", feature = "dashmap"))]
    impl<K, const N: usize> From<[(K, crate::Quota); N]> for KeyedMw<K>
    where
        K: Clone + Eq + core::hash::Hash,
    {
        fn from(value: [(K, crate::Quota); N]) -> Self {
            KeyedMw::<K>::new(value.iter().cloned())
        }
    }

    #[cfg(all(feature = "std", feature = "dashmap"))]
    impl<K> RateLimitingMiddleware<K, Duration> for KeyedMw<K>
    where
        K: std::fmt::Debug + Eq + core::hash::Hash,
    {
        type PositiveOutcome = ();

        type NegativeOutcome = NotUntil<Duration>;

        fn allow(_key: &K, _state: impl Into<StateSnapshot>) -> Self::PositiveOutcome {
            {}
        }

        fn disallow(
            _key: &K,
            state: impl Into<StateSnapshot>,
            start_time: Duration,
        ) -> Self::NegativeOutcome {
            NotUntil::new(state.into(), start_time)
        }

        fn check_quota(
            &self,
            key: &K,
            f: &dyn Fn(&Gcra) -> Result<Self::PositiveOutcome, Self::NegativeOutcome>,
        ) -> Option<Result<Self::PositiveOutcome, Self::NegativeOutcome>> {
            self.keys.get(key).map(f)
        }

        fn check_quota_n(
            &self,
            key: &K,
            f: &dyn Fn(
                &Gcra,
            ) -> Result<
                Result<Self::PositiveOutcome, Self::NegativeOutcome>,
                InsufficientCapacity,
            >,
        ) -> Option<
            Result<Result<Self::PositiveOutcome, Self::NegativeOutcome>, InsufficientCapacity>,
        > {
            self.keys.get(key).map(f)
        }
    }

    #[test]
    #[cfg(all(feature = "std", feature = "dashmap"))]
    fn trivial_keyed_middleware() {
        use std::time::Duration;

        use nonzero_ext::nonzero;

        use crate::Quota;

        let quota_1 = Quota {
            max_burst: nonzero!(3u32),
            replenish_1_per: Duration::from_millis(250),
        };

        let mw: KeyedMw<u32> = [(1, quota_1)].into();

        assert!(mw.check_quota(&1, &|_| Ok(())).is_some());
        assert!(mw.check_quota(&2, &|_| unimplemented!()).is_none());
    }
}
