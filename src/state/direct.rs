use crate::gcra::{NotUntil, Tat, GCRA};
use crate::lib::*;
use crate::{clock, NegativeMultiDecision, Quota};

/// A trait for state stores that only keep one rate limiting state.
///
/// This is blanket-implemented by all [`StateStore`]s with `()` key associated types.
pub trait DirectStateStore: StateStore<Key = ()> {}

impl<T> DirectStateStore for T where T: StateStore<Key = ()> {}

/// An in-memory rate limiter that makes direct (un-keyed)
/// rate-limiting decisions. Direct rate limiters can be used to
/// e.g. regulate the transmission of packets on a single connection,
/// or to ensure that an API client stays within a service's rate
/// limit.
#[derive(Debug)]
#[deprecated]
pub struct DirectRateLimiter<C: clock::Clock = clock::DefaultClock> {
    state: Tat,
    gcra: GCRA,
    clock: C,
    start: C::Instant,
}

/// The default constructor in `std` mode.
#[cfg(feature = "std")]
impl DirectRateLimiter<clock::DefaultClock> {
    /// Construct a new direct rate limiter for a quota with the default clock.
    pub fn new(quota: Quota) -> Self {
        let clock = clock::DefaultClock::default();
        DirectRateLimiter::new_with_clock(quota, &clock)
    }
}

/// The default constructor in `std` mode.
#[cfg(feature = "std")]
impl<S> RateLimiter<S::Key, S, clock::DefaultClock>
where
    S: DirectStateStore,
{
    //    /// Construct a new direct rate limiter for a quota with the default clock.
    //    pub fn direct(quota: Quota) -> Self {
    //        let clock = clock::DefaultClock::default();
    //        let state = S::new(gcra.starting_state(clock.now()));
    //        RateLimiter::new_with_clock(quota, state, &clock)
    //    }
}

/// Manually checking cells against direct rate limiters
///
/// These are available on the [`DirectRateLimiter2`][crate::state::DirectRateLimiter2] type.
impl<S, C> RateLimiter<S::Key, S, C>
where
    S: DirectStateStore,
    C: clock::Clock,
{
    /// Allow a single cell through the rate limiter.
    ///
    /// If the rate limit is reached, `check` returns information about the earliest
    /// time that a cell might be allowed through again.
    pub fn check(&self) -> Result<(), NotUntil<C::Instant>> {
        self.gcra
            .test_and_update_state(self.start, (), &self.state, self.clock.now())
    }

    /// Allow *only all* `n` cells through the rate limiter.
    ///
    /// This method can succeed in only one way and fail in two ways:
    /// * Success: If all `n` cells can be accommodated, it returns `Ok(())`.
    /// * Failure (but ok): Not all cells can make it through at the current time.
    ///   The result is `Err(NegativeMultiDecision::BatchNonConforming(NotUntil))`, which can
    ///   be interrogated about when the batch might next conform.
    /// * Failure (the batch can never go through): The rate limit is too low for the given number
    ///   of cells.
    ///
    /// # Performance
    /// This method diverges a little from the GCRA algorithm, using
    /// multiplication to determine the next theoretical arrival time, and so
    /// is not as fast as checking a single cell.  
    pub fn check_all(
        &self,
        n: NonZeroU32,
    ) -> Result<(), NegativeMultiDecision<NotUntil<C::Instant>>> {
        self.gcra
            .test_n_all_and_update_state(self.start, (), n, &self.state, self.clock.now())
    }
}

/// Manually checking cells against a rate limit.
impl<C: clock::Clock> DirectRateLimiter<C> {
    /// Allow a single cell through the rate limiter.
    ///
    /// If the rate limit is reached, `check` returns information about the earliest
    /// time that a cell might be allowed through again.  
    pub fn check(&self) -> Result<(), NotUntil<C::Instant>> {
        self.gcra
            .test_and_update(self.start, &self.state, self.clock.now())
    }

    /// Allow *only all* `n` cells through the rate limiter.
    ///
    /// This method can succeed in only one way and fail in two ways:
    /// * Success: If all `n` cells can be accommodated, it returns `Ok(())`.
    /// * Failure (but ok): Not all cells can make it through at the current time.
    ///   The result is `Err(NegativeMultiDecision::BatchNonConforming(NotUntil))`, which can
    ///   be interrogated about when the batch might next conform.
    /// * Failure (the batch can never go through): The rate limit is too low for the given number
    ///   of cells.
    ///
    /// # Performance
    /// This method diverges a little from the GCRA algorithm, using
    /// multiplication to determine the next theoretical arrival time, and so
    /// is not as fast as checking a single cell.  
    pub fn check_all(
        &self,
        n: NonZeroU32,
    ) -> Result<(), NegativeMultiDecision<NotUntil<C::Instant>>> {
        self.gcra
            .test_n_all_and_update(self.start, n, &self.state, self.clock.now())
    }
}

impl<C: clock::Clock> DirectRateLimiter<C> {
    /// Construct a new direct rate limiter with a custom clock.
    pub fn new_with_clock(quota: Quota, clock: &C) -> DirectRateLimiter<C> {
        let start = clock.now();
        let gcra: GCRA = GCRA::new(quota);
        let clock = clock.clone();
        let state = Tat::new(gcra.starting_state(clock.now(), start));
        DirectRateLimiter {
            state,
            clock,
            gcra,
            start,
        }
    }

    /// Returns a reference to the rate limiter's clock.
    pub fn get_clock(&self) -> &C {
        &self.clock
    }
}

#[cfg(feature = "std")]
mod future;
#[cfg(feature = "std")]
pub use future::*;

#[cfg(feature = "std")]
mod sinks;
#[cfg(feature = "std")]
pub use sinks::*;

#[cfg(feature = "std")]
mod streams;

use crate::state::{RateLimiter, StateStore};
#[cfg(feature = "std")]
pub use streams::*;
