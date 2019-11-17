use crate::lib::*;
use crate::nanos::Nanos;
use crate::state::StateStore;
use crate::{clock, NegativeMultiDecision, Quota};

/// An in-memory representation of a GCRA's rate-limiting state.
pub struct Tat(AtomicU64);

impl Tat {
    pub(crate) fn measure_and_replace_one<T, F, E>(&self, f: F) -> Result<T, E>
    where
        F: Fn(Nanos) -> Result<(T, Nanos), E>,
    {
        let mut prev = self.0.load(Ordering::Acquire);
        let mut decision = f(prev.into());
        while let Ok((result, new_data)) = decision {
            match self.0.compare_exchange_weak(
                prev,
                new_data.into(),
                Ordering::Release,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Ok(result),
                Err(next_prev) => prev = next_prev,
            }
            decision = f(prev.into());
        }
        // This map shouldn't be needed, as we only get here in the error case, but the compiler
        // can't see it.
        decision.map(|(result, _)| result)
    }
}

/// Tat is a valid in-memory state store.
impl StateStore for Tat {
    type Key = ();
    type CreationParameters = Nanos;

    fn measure_and_replace<T, F, E>(&self, _key: Self::Key, f: F) -> Result<T, E>
    where
        F: Fn(Nanos) -> Result<(T, Nanos), E>,
    {
        self.measure_and_replace_one(f)
    }

    fn new(start: Self::CreationParameters) -> Self {
        Tat(AtomicU64::new(start.into()))
    }
}

impl Debug for Tat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let d = Duration::from_nanos(self.0.load(Ordering::Relaxed));
        write!(f, "Tat({:?})", d)
    }
}

/// A negative rate-limiting outcome.
///
/// `NotUntil`'s methods indicate when a caller can expect the next positive
/// rate-limiting result.
#[derive(Debug, PartialEq)]
pub struct NotUntil<'a, P: clock::Reference> {
    limiter: &'a GCRA,
    tat: Nanos,
    start: P,
}

impl<'a, P: clock::Reference> NotUntil<'a, P> {
    /// Returns the earliest time at which a decision could be
    /// conforming (excluding conforming decisions made by the Decider
    /// that are made in the meantime).
    pub fn earliest_possible(&self) -> P {
        let tat: Duration = self.tat.into();
        self.start + tat
    }

    /// Returns the minimum amount of time from the time that the
    /// decision was made that must pass before a
    /// decision can be conforming.
    ///
    /// If the time of the next expected positive result is in the past,
    /// `wait_time_from` returns a zero `Duration`.
    pub fn wait_time_from(&self, from: P) -> Duration {
        let earliest = self.earliest_possible();
        earliest.duration_since(earliest.min(from))
    }
}

impl<'a, P: clock::Reference> fmt::Display for NotUntil<'a, P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let tat: Duration = self.tat.into();
        write!(f, "rate-limited until {:?}", self.start + tat)
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct GCRA {
    // The "weight" of a single packet in units of time.
    t: Nanos,

    // The "capacity" of the bucket.
    tau: Nanos,
}

impl GCRA {
    pub(crate) fn new(quota: Quota) -> Self {
        let tau: Nanos = quota.per.into();
        let t: Nanos = (quota.per / quota.max_burst.get()).into();
        GCRA { tau, t }
    }

    pub(crate) fn starting_state<P: clock::Reference>(&self, now: P, start: P) -> Nanos {
        Nanos::from(now.duration_since(start)) + self.t
    }

    /// Tests a single cell against the rate limiter state and updates it at the given key.
    pub(crate) fn test_and_update<K, P: clock::Reference>(
        &self,
        start: P,
        key: K,
        state: &impl StateStore<Key = K>,
        t0: P,
    ) -> Result<(), NotUntil<P>> {
        let t0: Nanos = t0.duration_since(start).into();
        let tau = self.tau;
        let t = self.t;
        state.measure_and_replace(key, |tat| {
            let earliest_time = tat.saturating_sub(tau);
            if t0 < earliest_time {
                Err(NotUntil {
                    limiter: self,
                    tat: earliest_time,
                    start,
                })
            } else {
                Ok(((), cmp::max(tat, t0) + t))
            }
        })
    }

    /// Tests whether all `n` cells could be accommodated and updates the rate limiter state, if so.
    pub(crate) fn test_n_all_and_update<K, P: clock::Reference>(
        &self,
        start: P,
        key: K,
        n: NonZeroU32,
        state: &impl StateStore<Key = K>,
        t0: P,
    ) -> Result<(), NegativeMultiDecision<NotUntil<P>>> {
        let t0: Nanos = t0.duration_since(start).into();
        let tau = self.tau;
        let t = self.t;
        let weight = t * (n.get() - 1) as u64;
        if weight > tau {
            return Err(NegativeMultiDecision::InsufficientCapacity(
                (tau.as_u64() / t.as_u64()) as u32,
            ));
        }
        state.measure_and_replace(key, |tat| {
            let earliest_time = (tat + weight).saturating_sub(tau);
            if t0 < earliest_time {
                Err(NegativeMultiDecision::BatchNonConforming(
                    n.get(),
                    NotUntil {
                        limiter: self,
                        tat: earliest_time,
                        start,
                    },
                ))
            } else {
                Ok(((), cmp::max(tat, t0) + t + weight))
            }
        })
    }
}
