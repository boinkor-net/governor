//! Keyed rate limiters (those that can hold one state per key).
//!
//! These are rate limiters that have one set of parameters (burst capacity per time period) but
//! apply those to several sets of actual rate-limiting states, e.g. to enforce one API call rate
//! limit per API key.
//!
//! Rate limiters based on these types are constructed with
//! [the `RateLimiter` constructors](../struct.RateLimiter.html#keyed-rate-limiters---default-constructors)

use crate::lib::*;

use crate::state::StateStore;
use crate::{clock, NegativeMultiDecision, NotUntil, Quota, RateLimiter};

/// A trait for state stores with one rate limiting state per key.
///
/// This is blanket-implemented by all [`StateStore`]s with hashable (`Eq + Hash + Clone`) key
/// associated types.
pub trait KeyedStateStore<K: Hash>: StateStore<Key = K> {}

impl<T, K: Hash> KeyedStateStore<K> for T
where
    T: StateStore<Key = K>,
    K: Eq + Clone + Hash,
{
}

#[cfg(feature = "std")]
/// # Keyed rate limiters - default constructors
impl<K> RateLimiter<K, DefaultKeyedStateStore<K>, clock::MonotonicClock>
where
    K: Clone + Hash + Eq,
{
    #[cfg(all(feature = "std", feature = "dashmap"))]
    /// Construct a new keyed rate limiter backed by
    /// the [`DefaultKeyedStateStore`].
    pub fn keyed(quota: Quota) -> Self {
        let state = DefaultKeyedStateStore::default();
        let clock = clock::MonotonicClock::default();
        RateLimiter::new(quota, state, &clock)
    }

    #[cfg(all(feature = "std", feature = "dashmap"))]
    /// Constructs a new keyed rate limiter explicitly backed by a [`DashMap`][dashmap::DashMap].
    pub fn dashmap(quota: Quota) -> Self {
        let state = DashMapStateStore::default();
        let clock = clock::MonotonicClock::default();
        RateLimiter::new(quota, state, &clock)
    }

    #[cfg(all(feature = "std", not(feature = "dashmap")))]
    /// Constructs a new keyed rate limiter explicitly backed by a
    /// [`HashMap`][std::collections::HashMap].
    pub fn hashmap(quota: Quota) -> Self {
        let state = HashMapStateStore::default();
        let clock = clock::MonotonicClock::default();
        RateLimiter::new(quota, state, &clock)
    }
}

#[cfg(all(feature = "std", feature = "dashmap"))]
impl<K> RateLimiter<K, HashMapStateStore<K>, clock::MonotonicClock>
where
    K: Clone + Hash + Eq,
{
    /// Constructs a new keyed rate limiter explicitly backed by a
    /// [`HashMap`][std::collections::HashMap].
    pub fn hashmap(quota: Quota) -> Self {
        let state = HashMapStateStore::default();
        let clock = clock::MonotonicClock::default();
        RateLimiter::new(quota, state, &clock)
    }
}

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
mod hashmap;
#[cfg(feature = "std")]
pub use hashmap::HashMapStateStore;

#[cfg(all(feature = "std", feature = "dashmap"))]
mod dashmap;
#[cfg(all(feature = "std", feature = "dashmap"))]
pub use self::dashmap::DashMapStateStore;

#[cfg(feature = "std")]
mod future;

#[cfg(all(feature = "std", not(feature = "dashmap")))]
/// The default keyed rate limiter type: a mutex-wrapped [`HashMap`][std::collections::HashMap].
pub type DefaultKeyedStateStore<K> = HashMapStateStore<K>;

#[cfg(all(feature = "std", feature = "dashmap"))]
/// The default keyed rate limiter type: the concurrent [`DashMap`][dashmap::DashMap].
pub type DefaultKeyedStateStore<K> = DashMapStateStore<K>;
