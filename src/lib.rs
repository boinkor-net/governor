use derive_more::*;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use std::{cmp, fmt};

mod clock;
pub use clock::*;
use std::ops::{Add, Sub};

/// A number of nanoseconds from a reference point.
#[derive(Add, Sub, PartialEq, Eq, Default, Debug, From, Into)]
struct Nanos(u64);

impl From<Duration> for Nanos {
    fn from(d: Duration) -> Self {
        Nanos(d.as_nanos() as u64)
    }
}

impl Add<Duration> for Nanos {
    type Output = Nanos;

    fn add(self, rhs: Duration) -> Self::Output {
        let rhs: Nanos = rhs.into();
        Nanos(rhs.0 + self.0)
    }
}

impl Sub<Duration> for Nanos {
    type Output = Nanos;

    fn sub(self, rhs: Duration) -> Self::Output {
        let rhs: Nanos = rhs.into();
        Nanos(rhs.0 - self.0)
    }
}

#[derive(Default, Debug)]
pub struct Tat(AtomicU64);

impl Tat {
    fn tat_value_from_stored(u: u64) -> Option<Duration> {
        NonZeroU64::new(u).map(|ns| Duration::from_nanos(ns.get()))
    }

    fn measure_and_replace<F, E>(&self, f: F) -> Result<(), E>
    where
        F: Fn(Option<Duration>) -> Result<Duration, E>,
    {
        let mut prev = self.0.load(Ordering::Acquire);
        let mut decision = f(Self::tat_value_from_stored(prev));
        while let Ok(new_data) = decision {
            match self.0.compare_exchange_weak(
                prev,
                new_data.as_nanos() as u64, // TODO: correctly wrap (after 500 years, lol)
                Ordering::Release,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Ok(()),
                Err(next_prev) => prev = next_prev,
            }
            decision = f(Self::tat_value_from_stored(prev));
        }
        // This map shouldn't be needed, as we only get here in the error case, but the compiler
        // can't see it.
        decision.map(|_| ())
    }
}

#[derive(Debug, PartialEq)]
pub struct NotUntil<'a, P: clock::Reference> {
    limiter: &'a GCRA<P>,
    tat: Duration,
}

impl<'a, P: clock::Reference> fmt::Display for NotUntil<'a, P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "rate-limited until {:?}", self.limiter.start + self.tat)
    }
}

#[derive(Debug, PartialEq)]
pub struct GCRA<P: clock::Reference = <clock::DefaultClock as clock::Clock>::Instant> {
    // The "weight" of a single packet in units of time.
    t: Duration,

    // The "capacity" of the bucket.
    tau: Duration,

    // Base reference to when the bucket got created. All measurements are relative to this
    // timestamp.
    start: P,
}

impl<P: clock::Reference> GCRA<P> {
    pub fn test_and_update(&self, state: &Tat, t0: P) -> Result<(), NotUntil<P>> {
        let t0 = self.start.duration_since(t0);
        let tau = self.tau;
        let t = self.t;
        state.measure_and_replace(|tat| {
            // the "theoretical arrival time" of the next cell:
            let tat = tat.unwrap_or(t0);
            if t0 < tat.saturating_sub(tau) {
                Err(NotUntil { limiter: self, tat })
            } else {
                Ok(cmp::max(tat, t0) + t)
            }
        })
    }
}

#[test]
fn check_with_duration() {
    let clock = FakeRelativeClock::default();
    let gcra = GCRA {
        t: Duration::from_secs(1),
        tau: Duration::from_secs(1),
        start: clock.now(),
    };
    let state: Tat = Default::default();
    let now = clock.now();

    crossbeam::scope(|scope| {
        scope.spawn(|_| {
            (&gcra)
                .test_and_update(&state, now)
                .expect("should succeed");
        });
        scope.spawn(|_| {
            (&gcra)
                .test_and_update(&state, now)
                .expect("should succeed");
        });
    })
    .unwrap();

    assert_ne!((&gcra).test_and_update(&state, now), Ok(()));
}
