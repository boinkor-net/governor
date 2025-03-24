use crate::nanos::Nanos;
use crate::{clock, Quota, RateLimiter};
use crate::{
    middleware::NoOpMiddleware,
    state::{InMemoryState, StateStore},
};
use core::hash::Hash;

#[cfg(feature = "no_std")]
pub use hashbrown::HashMap;
#[cfg(not(feature = "no_std"))]
pub use std::collections::HashMap;

use crate::state::keyed::{DefaultHasher, ShrinkableKeyedStateStore};

#[cfg(feature = "std")]
type Mutex<T> = parking_lot::Mutex<T>;

#[cfg(not(feature = "std"))]
type Mutex<T> = spinning_top::Spinlock<T>;

/// A thread-safe (but not very performant) implementation of a keyed rate limiter state
/// store using [`HashMap`].
///
/// The `HashMapStateStore` is the default state store in `std` when no other thread-safe
/// features are enabled.
pub type HashMapStateStore<K, S = DefaultHasher> = Mutex<HashMap<K, InMemoryState, S>>;

impl<K: Hash + Eq + Clone, S: core::hash::BuildHasher> StateStore for HashMapStateStore<K, S> {
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

impl<K: Hash + Eq + Clone, S: core::hash::BuildHasher> ShrinkableKeyedStateStore<K>
    for HashMapStateStore<K, S>
{
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

/// # Keyed rate limiters - [`HashMap`]-backed with a default hasher
impl<K, C> RateLimiter<K, HashMapStateStore<K>, C, NoOpMiddleware<C::Instant>>
where
    K: Hash + Eq + Clone,
    C: clock::Clock,
{
    /// Constructs a new rate limiter with a custom clock, backed by a [`HashMap`] with the default hasher.
    pub fn hashmap_with_clock(quota: Quota, clock: C) -> Self {
        let state: HashMapStateStore<K> = HashMapStateStore::new(HashMap::default());
        RateLimiter::new(quota, state, clock)
    }
}

/// # Keyed rate limiters - [`HashMap`]-backed with a custom hasher
impl<K, S, C> RateLimiter<K, HashMapStateStore<K, S>, C, NoOpMiddleware<C::Instant>>
where
    K: Hash + Eq + Clone,
    S: core::hash::BuildHasher,
    C: clock::Clock,
{
    /// Constructs a new rate limiter with a custom clock and hasher, backed by a [`HashMap`].
    pub fn hashmap_with_clock_and_hasher(quota: Quota, clock: C, hasher: S) -> Self {
        let state: HashMapStateStore<K, S> = HashMapStateStore::new(HashMap::with_hasher(hasher));
        RateLimiter::new(quota, state, clock)
    }
}
