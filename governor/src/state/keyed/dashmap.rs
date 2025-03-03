#![cfg(all(feature = "std", feature = "dashmap"))]

use std::prelude::v1::*;

use crate::nanos::Nanos;
use crate::state::keyed::DefaultHasher;
use crate::state::{InMemoryState, StateStore};
use crate::{clock, Quota, RateLimiter};
use crate::{middleware::NoOpMiddleware, state::keyed::ShrinkableKeyedStateStore};
use core::hash::Hash;
use dashmap::DashMap;

/// A concurrent, thread-safe and fairly performant hashmap based on [`DashMap`].
pub type DashMapStateStore<K, S = DefaultHasher> = DashMap<K, InMemoryState, S>;

impl<K: Hash + Eq + Clone, S: core::hash::BuildHasher + Clone> StateStore
    for DashMapStateStore<K, S>
{
    type Key = K;

    fn measure_and_replace<T, F, E>(&self, key: &Self::Key, f: F) -> Result<T, E>
    where
        F: Fn(Option<Nanos>) -> Result<(T, Nanos), E>,
    {
        if let Some(v) = self.get(key) {
            // fast path: measure existing entry
            return v.measure_and_replace_one(f);
        }
        // make an entry and measure that:
        let entry = self.entry(key.clone()).or_default();
        (*entry).measure_and_replace_one(f)
    }
}

/// # Keyed rate limiters - [`DashMap`]-backed with a default hasher
impl<K, C> RateLimiter<K, DashMapStateStore<K>, C, NoOpMiddleware<C::Instant>>
where
    K: Hash + Eq + Clone,
    C: clock::Clock,
{
    /// Constructs a new rate limiter with a custom clock, backed by a
    /// [`DashMap`] with the default hasher.
    pub fn dashmap_with_clock(quota: Quota, clock: C) -> Self {
        let state: DashMapStateStore<K> = DashMap::default();
        RateLimiter::new(quota, state, clock)
    }
}

/// # Keyed rate limiters - [`DashMap`]-backed with a custom hasher
impl<K, S, C> RateLimiter<K, DashMapStateStore<K, S>, C, NoOpMiddleware<C::Instant>>
where
    K: Hash + Eq + Clone,
    S: core::hash::BuildHasher + Default + Clone,
    C: clock::Clock,
{
    /// Constructs a new rate limiter with a custom clock and hasher, backed by a
    /// [`DashMap`].
    pub fn dashmap_with_clock_and_hasher(quota: Quota, clock: C, hasher: S) -> Self {
        let state: DashMapStateStore<K, S> = DashMap::with_hasher(hasher);
        RateLimiter::new(quota, state, clock)
    }
}

impl<K: Hash + Eq + Clone, S: core::hash::BuildHasher + Clone> ShrinkableKeyedStateStore<K>
    for DashMapStateStore<K, S>
{
    fn retain_recent(&self, drop_below: Nanos) {
        self.retain(|_, v| !v.is_older_than(drop_below));
    }

    fn shrink_to_fit(&self) {
        self.shrink_to_fit();
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}
