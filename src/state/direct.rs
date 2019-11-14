use crate::gcra::{NotUntil, Tat, GCRA};
use crate::lib::*;
use crate::{clock, NegativeMultiDecision, Quota};

/// An in-memory rate limiter that makes direct (un-keyed)
/// rate-limiting decisions. Direct rate limiters can be used to
/// e.g. regulate the transmission of packets on a single connection,
/// or to ensure that an API client stays within a service's rate
/// limit.
#[derive(Debug)]
pub struct DirectRateLimiter<C: clock::Clock = clock::DefaultClock> {
    state: Tat,
    gcra: GCRA<C::Instant>,
    clock: C,
}

impl<C: clock::Clock> DirectRateLimiter<C> {
    /// Construct a new direct rate limiter for a quota.
    pub fn new(quota: Quota) -> DirectRateLimiter<C> {
        let clock: C = Default::default();
        DirectRateLimiter::new_with_clock(quota, &clock)
    }

    /// Allow a single cell through the rate limiter.
    ///
    /// If the rate limit is reached, `check` returns information about the earliest
    /// time that a cell might be allowed through again.  
    pub fn check(&self) -> Result<(), NotUntil<C::Instant>> {
        self.gcra.test_and_update(&self.state, self.clock.now())
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
    pub fn check_n_all(
        &self,
        n: NonZeroU32,
    ) -> Result<(), NegativeMultiDecision<NotUntil<C::Instant>>> {
        self.gcra
            .test_n_all_and_update(n, &self.state, self.clock.now())
    }

    /// Construct a new direct rate limiter with a custom clock.
    pub fn new_with_clock(quota: Quota, clock: &C) -> DirectRateLimiter<C> {
        let gcra: GCRA<C::Instant> = GCRA::new(clock.now(), quota);
        let clock = clock.clone();
        let state = gcra.new_state(clock.now());
        DirectRateLimiter { state, clock, gcra }
    }
}
