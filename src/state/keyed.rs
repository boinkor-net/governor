//! Keyed rate limiters.
//!
//! These are rate limiters that have one set of parameters (burst capacity per time period) but
//! apply those to several sets of actual rate-limiting states, e.g. to enforce one API call rate
//! limit per API key.

use crate::lib::*;

use crate::state::StateStore;
use crate::{clock, NegativeMultiDecision, NotUntil, RateLimiter};

pub trait KeyedStateStore<K: Hash>: StateStore<Key = K> {}

impl<T, K: Hash> KeyedStateStore<K> for T where T: StateStore<Key = K> {}

/// # Keyed rate limiters - Manually checking cells
impl<K, S, C> RateLimiter<K, S, C>
where
    S: KeyedStateStore<K>,
    K: Hash,
    C: clock::Clock,
{
    /// Allow a single cell through the rate limiter for the given key.
    ///
    /// If the rate limit is reached, `check_key` returns information about the earliest
    /// time that a cell might be allowed through again under that key.
    pub fn check_key(&self, key: &K) -> Result<(), NotUntil<C::Instant>> {
        self.gcra
            .test_and_update(self.start, key, &self.state, self.clock.now())
    }

    /// Allow *only all* `n` cells through the rate limiter for the given key.
    ///
    /// This method can succeed in only one way and fail in two ways:
    /// * Success: If all `n` cells can be accommodated, it returns `Ok(())`.
    /// * Failure (but ok): Not all cells can make it through at the current time.
    ///   The result is `Err(NegativeMultiDecision::BatchNonConforming(NotUntil))`, which can
    ///   be interrogated about when the batch might next conform.
    /// * Failure (the batch can never go through): The rate limit is too low for the given number
    ///   of cells.
    ///
    /// ### Performance
    /// This method diverges a little from the GCRA algorithm, using
    /// multiplication to determine the next theoretical arrival time, and so
    /// is not as fast as checking a single cell.
    pub fn check_key_all(
        &self,
        key: &K,
        n: NonZeroU32,
    ) -> Result<(), NegativeMultiDecision<NotUntil<C::Instant>>> {
        self.gcra
            .test_n_all_and_update(self.start, key, n, &self.state, self.clock.now())
    }
}

#[cfg(feature = "std")]
mod hash_map;

#[cfg(feature = "std")]
pub use hash_map::HashMapStateStore;

#[cfg(all(feature = "std", feature = "dashmap"))]
mod dashmap;

#[cfg(all(feature = "std", feature = "dashmap"))]
pub use self::dashmap::DashMapStateStore;
