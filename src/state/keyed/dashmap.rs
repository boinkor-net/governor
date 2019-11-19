#![cfg(all(feature = "std", feature = "dashmap"))]

use crate::lib::*;

use crate::gcra::Tat;
use crate::nanos::Nanos;
use crate::state::StateStore;
use crate::{clock, Quota, RateLimiter};
use dashmap::DashMap;

pub type DashMapStateStore<K> = DashMap<K, Tat>;

impl<K: Hash + Eq + Clone> StateStore for DashMapStateStore<K> {
    type Key = K;

    fn measure_and_replace<T, F, E>(&self, key: &Self::Key, f: F) -> Result<T, E>
    where
        F: Fn(Option<Nanos>) -> Result<(T, Nanos), E>,
    {
        let entry = self.get_or_insert_with(key, Tat::default);
        (*entry).measure_and_replace_one(f)
    }
}

/// # Keyed rate limiters - Constructors
impl<K, C> RateLimiter<K, DashMapStateStore<K>, C>
where
    K: Hash + Eq + Clone,
    C: clock::Clock,
{
    pub fn dashmap_with_clock(quota: Quota, clock: &C) -> Self {
        let state: DashMapStateStore<K> = DashMap::default();
        RateLimiter::new(quota, state, clock)
    }
}
