//! State stores for rate limiters

pub mod direct;

use crate::gcra::{Tat, GCRA};
use crate::nanos::Nanos;
use crate::{clock, Quota};
pub use direct::*;

/// A way for rate limiters to keep state.
///
/// There are two important kinds of state stores: Direct and keyed. The direct kind have only
/// one state, and are useful for "global" rate limit enforcement (e.g. a process should never
/// do more than N tasks a day). The keyed kind allows one rate limit per key (e.g. an API
/// call budget per client API key).
///
/// A direct state store is expressed as [`StateStore::Key`] = `()`. Keyed state stores have a
/// type parameter for the key and set their key to that.
pub trait StateStore {
    type Key;

    /// Updates a state store's rate limiting state for a given key, using the given closure.
    ///
    /// The closure parameter takes the old value of the state store at the key's location,
    /// checks if the request an be accommodated and:
    ///
    /// * If the request is rate-limited, returns `Err(E)`.
    /// * If the request can make it through, returns `Ok(T)` (an arbitrary positive return
    ///   value) and the updated state.
    ///
    /// It is `measure_and_replace`'s job then to safely replace the value at the key - it must
    /// only update the value if the value hasn't changed. The implementations in this
    /// crate use `AtomicU64` operations for this.    
    fn measure_and_replace<T, F, E>(&self, key: Self::Key, f: F) -> Result<T, E>
    where
        F: Fn(Nanos) -> Result<(T, Nanos), E>;

    /// Returns a new rate limiting state, given an initial value.
    fn new(initial: Nanos) -> Self;
}

/// A rate limiter.
pub struct RateLimiter<K, S, C>
where
    S: StateStore<Key = K>,
    C: clock::Clock,
{
    state: S,
    gcra: GCRA,
    clock: C,
    start: C::Instant,
}

impl<K, S, C> RateLimiter<K, S, C>
where
    S: StateStore<Key = K>,
    C: clock::Clock,
{
    //    pub fn new_with_clock(quota: Quota, state: S, clock: C) -> Self {
    //        let gcra: GCRA = GCRA::new(quota);
    //        let clock = clock.clone();
    //        RateLimiter {
    //            state,
    //            clock,
    //            gcra,
    //            start: clock.now(),
    //        }
    //    }
}

pub type DirectRateLimiter2<C> = RateLimiter<(), Tat, C>;
