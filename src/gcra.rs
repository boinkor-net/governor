use crate::lib::*;
use crate::nanos::Nanos;
use crate::{clock, NegativeMultiDecision, Quota};

#[derive(Debug)]
pub(crate) struct Tat(AtomicU64);

impl Tat {
    fn new(tat: Nanos) -> Tat {
        Tat(AtomicU64::new(tat.into()))
    }

    fn measure_and_replace<F, E>(&self, f: F) -> Result<(), E>
    where
        F: Fn(Nanos) -> Result<Nanos, E>,
    {
        let mut prev = self.0.load(Ordering::Acquire);
        let mut decision = f(prev.into());
        while let Ok(new_data) = decision {
            match self.0.compare_exchange_weak(
                prev,
                new_data.into(),
                Ordering::Release,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Ok(()),
                Err(next_prev) => prev = next_prev,
            }
            decision = f(prev.into());
        }
        // This map shouldn't be needed, as we only get here in the error case, but the compiler
        // can't see it.
        decision.map(|_| ())
    }
}

#[derive(Debug, PartialEq)]
pub struct NotUntil<'a, P: clock::Reference> {
    limiter: &'a GCRA<P>,
    tat: Nanos,
}

impl<'a, P: clock::Reference> fmt::Display for NotUntil<'a, P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let tat: Duration = self.tat.into();
        write!(f, "rate-limited until {:?}", self.limiter.start + tat)
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct GCRA<P: clock::Reference = <clock::DefaultClock as clock::Clock>::Instant> {
    // The "weight" of a single packet in units of time.
    t: Nanos,

    // The "capacity" of the bucket.
    tau: Nanos,

    // Base reference to when the bucket got created. All measurements are relative to this
    // timestamp.
    start: P,
}

impl<P: clock::Reference> GCRA<P> {
    pub(crate) fn new(start: P, quota: Quota) -> Self {
        let tau: Nanos = quota.per.into();
        let t: Nanos = (quota.per / quota.max_burst.get()).into();
        GCRA { tau, t, start }
    }

    pub(crate) fn new_state(&self, now: P) -> Tat {
        let tat = Nanos::from(now.duration_since(self.start)) + self.t;
        Tat::new(tat)
    }

    pub(crate) fn test_and_update(&self, state: &Tat, t0: P) -> Result<(), NotUntil<P>> {
        let t0: Nanos = t0.duration_since(self.start).into();
        let tau = self.tau;
        let t = self.t;
        state.measure_and_replace(|tat| {
            if t0 < tat.saturating_sub(tau) {
                Err(NotUntil { limiter: self, tat })
            } else {
                Ok(cmp::max(tat, t0) + t)
            }
        })
    }

    /// Tests whether all `n` cells could be accommodated and updates the rate limiter state, if so.
    ///
    /// As this method is an extension of GCRA (using multiplication),
    /// it is likely not as fast (and not as obviously "right") as the
    /// single-cell variant.
    pub(crate) fn test_n_all_and_update(
        &self,
        n: NonZeroU32,
        state: &Tat,
        t0: P,
    ) -> Result<(), NegativeMultiDecision<NotUntil<P>>> {
        let t0: Nanos = t0.duration_since(self.start).into();
        let tau = self.tau;
        let t = self.t;
        let weight: u64 = n.get() as u64 * t.as_u64();
        if weight > tau.into() {
            return Err(NegativeMultiDecision::InsufficientCapacity(
                (tau.as_u64() / t.as_u64()) as u32,
            ));
        }
        state.measure_and_replace(|tat| {
            if t0 + weight.into() < tat.saturating_sub(tau) {
                Err(NegativeMultiDecision::BatchNonConforming(
                    n.get(),
                    NotUntil { limiter: self, tat },
                ))
            } else {
                Ok(cmp::max(tat, t0) + weight.into())
            }
        })
    }
}
