use crate::gcra::{NotUntil, Tat, GCRA};
use crate::lib::*;
use crate::{clock, Quota};

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

    /// Construct a new direct rate limiter with a custom clock.
    pub fn new_with_clock(quota: Quota, clock: &C) -> DirectRateLimiter<C> {
        let gcra: GCRA<C::Instant> = GCRA::new(clock.now(), quota);
        let clock = clock.clone();
        let state = gcra.new_state(clock.now());
        DirectRateLimiter { state, clock, gcra }
    }

    pub fn check(&self) -> Result<(), NotUntil<C::Instant>> {
        self.gcra.test_and_update(&self.state, self.clock.now())
    }
}
