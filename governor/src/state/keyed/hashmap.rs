use std::prelude::v1::*;

use crate::nanos::Nanos;
use crate::{clock, Quota, RateLimiter};
use crate::{
    middleware::NoOpMiddleware,
    state::{InMemoryState, StateStore},
};
use std::collections::HashMap;
use std::hash::Hash;

use crate::state::keyed::ShrinkableKeyedStateStore;

#[cfg(feature = "std")]
type Mutex<T> = parking_lot::Mutex<T>;

#[cfg(not(feature = "std"))]
type Mutex<T> = spinning_top::Spinlock<T>;

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
        if let Some(v) = (*map).get(key) {
            // fast path: a rate limiter is already present for the key.
            return v.measure_and_replace_one(f);
        }
        // not-so-fast path: make a new entry and measure it.
        let entry = (*map).entry(key.clone()).or_default();
        entry.measure_and_replace_one(f)
    }
}

impl<K: Hash + Eq + Clone> ShrinkableKeyedStateStore<K> for HashMapStateStore<K> {
    fn retain_recent(&self, drop_below: Nanos) {
        let mut map = self.lock();
        map.retain(|_, v| !v.is_older_than(drop_below));
    }

    fn shrink_to_fit(&self) {
        let mut map = self.lock();
        map.shrink_to_fit();
    }

    fn len(&self) -> usize {
        let map = self.lock();
        (*map).len()
    }
    fn is_empty(&self) -> bool {
        let map = self.lock();
        (*map).is_empty()
    }
}

/// # Keyed rate limiters - [`HashMap`]-backed
impl<K, C> RateLimiter<K, HashMapStateStore<K>, C, NoOpMiddleware<C::Instant>>
where
    K: Hash + Eq + Clone,
    C: clock::Clock,
{
    /// Constructs a new rate limiter with a custom clock, backed by a [`HashMap`].
    pub fn hashmap_with_clock(quota: Quota, clock: &C) -> Self {
        let state: HashMapStateStore<K> = HashMapStateStore::new(HashMap::new());
        RateLimiter::new(quota, state, clock)
    }
}
