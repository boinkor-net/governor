#![cfg(feature = "std")]

use crate::lib::*;

use crate::nanos::Nanos;
use crate::state::{InMemoryState, StateStore};
use crate::{clock, Quota, RateLimiter};
use parking_lot::Mutex;

/// A thread-safe (but not very performant) implementation of a keyed rate limiter state
/// store using [`HashMap`].
///
/// The `HashMapStateStore` is the default state store in `std` when no other thread-safe
/// features are enabled.
pub type HashMapStateStore<K> = Mutex<HashMap<K, InMemoryState>>;

impl<K: Hash + Eq + Clone> StateStore for HashMapStateStore<K> {
    type Key = K;

    fn measure_and_replace<T, F, E>(&self, key: &Self::Key, f: F) -> Result<T, E>
    where
        F: Fn(Option<Nanos>) -> Result<(T, Nanos), E>,
    {
        let mut map = self.lock();
        let entry = (*map)
            .entry(key.clone())
            .or_insert_with(InMemoryState::default);
        entry.measure_and_replace_one(f)
    }
}

/// # Keyed rate limiters - [`HashMap`]-backed
impl<K, C> RateLimiter<K, HashMapStateStore<K>, C>
where
    K: Hash + Eq + Clone,
    C: clock::Clock,
{
    /// Constructs a new rate limiter with a custom clock, backed by a [`HashMap`].
    pub fn hashmap_with_clock(quota: Quota, clock: &C) -> Self {
        let state: HashMapStateStore<K> = Mutex::new(HashMap::new());
        RateLimiter::new(quota, state, clock)
    }
}
