//! Time sources for rate limiters.
//!
//! The time sources contained in this module allow the rate limiter
//! to be (optionally) independent of std, and additionally
//! allow mocking the passage of time.

use crate::lib::*;

/// A measurement from a clock.
pub trait Reference:
    Sized + Add<Duration, Output = Self> + PartialEq + Eq + Ord + Copy + Clone + Send + Sync + Debug
{
    /// Determines the time that separates two measurements of a
    /// clock. Implementations of this must perform a saturating
    /// subtraction - if the `earlier` timestamp should be later,
    /// `duration_since` must return the zero duration.
    fn duration_since(&self, earlier: Self) -> Duration;

    /// Returns a reference point that lies at most `duration` in the
    /// past from the current reference. If an underflow should occur,
    /// returns the current reference.
    fn saturating_sub(&self, duration: Duration) -> Self;
}

/// A time source used by rate limiters.
pub trait Clock: Clone {
    /// A measurement of a monotonically increasing clock.
    type Instant: Reference;

    /// Returns a measurement of the clock.
    fn now(&self) -> Self::Instant;
}

impl Reference for Duration {
    fn duration_since(&self, earlier: Self) -> Duration {
        self.checked_sub(earlier)
            .unwrap_or_else(|| Duration::new(0, 0))
    }

    fn saturating_sub(&self, duration: Duration) -> Self {
        self.checked_sub(duration).unwrap_or(*self)
    }
}

/// A mock implementation of a clock. All it does is keep track of
/// what "now" is (relative to some point meaningful to the program),
/// and returns that.
///
/// # Thread safety
/// The mock time is represented as an atomic u64 count of nanoseconds, behind an [`Arc`].
/// Clones of this clock will all show the same time, even if the original advances.
#[derive(Debug, Clone, Default)]
pub struct FakeRelativeClock {
    now: Arc<AtomicU64>,
}

impl FakeRelativeClock {
    /// Advances the fake clock by the given amount.
    pub fn advance(&mut self, by: Duration) {
        let by: u64 = by
            .as_nanos()
            .try_into()
            .expect("Can not represent times past ~584 years");

        let mut prev = self.now.load(Ordering::Acquire);
        let mut next = prev + by;
        while let Err(next_prev) =
            self.now
                .compare_exchange_weak(prev, next, Ordering::Release, Ordering::Relaxed)
        {
            prev = next_prev;
            next = prev + by;
        }
    }
}

impl PartialEq for FakeRelativeClock {
    fn eq(&self, other: &Self) -> bool {
        self.now.load(Ordering::Relaxed) == other.now.load(Ordering::Relaxed)
    }
}

impl Clock for FakeRelativeClock {
    type Instant = Duration;

    fn now(&self) -> Self::Instant {
        Duration::from_nanos(self.now.load(Ordering::Relaxed))
    }
}

#[cfg(not(feature = "std"))]
mod no_std;
#[cfg(not(feature = "std"))]
pub use no_std::*;

#[cfg(feature = "std")]
mod with_std;
#[cfg(feature = "std")]
pub use with_std::*;
