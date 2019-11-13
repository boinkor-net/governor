//! Time sources for the rate limiter.
//!
//! The time sources contained in this module allow the rate limiter
//! to be (optionally) independent of std, and should additionally
//! allow mocking the passage of time.

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
pub trait Clock: Default + Clone {
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
#[derive(Debug, PartialEq, Clone, Default)]
pub struct FakeRelativeClock {
    now: Duration,
}

impl FakeRelativeClock {
    /// Advances the fake clock by the given amount.
    pub fn advance(&mut self, by: Duration) {
        self.now += by
    }
}

impl Clock for FakeRelativeClock {
    type Instant = Duration;

    fn now(&self) -> Self::Instant {
        self.now
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

use std::fmt::Debug;
use std::ops::Add;
use std::time::Duration;
