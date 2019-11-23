//! Time sources for rate limiters.
//!
//! The time sources contained in this module allow the rate limiter
//! to be (optionally) independent of std, and additionally
//! allow mocking the passage of time.

use crate::lib::*;

/// A measurement from a clock.
pub trait Reference:
    Sized + Add<Nanos, Output = Self> + PartialEq + Eq + Ord + Copy + Clone + Send + Sync + Debug
{
    /// Determines the time that separates two measurements of a
    /// clock. Implementations of this must perform a saturating
    /// subtraction - if the `earlier` timestamp should be later,
    /// `duration_since` must return the zero duration.
    fn duration_since(&self, earlier: Self) -> Nanos;

    /// Returns a reference point that lies at most `duration` in the
    /// past from the current reference. If an underflow should occur,
    /// returns the current reference.
    fn saturating_sub(&self, duration: Nanos) -> Self;
}

/// A time source used by rate limiters.
pub trait Clock: Clone {
    /// A measurement of a monotonically increasing clock.
    type Instant: Reference;

    /// Returns a measurement of the clock.
    fn now(&self) -> Self::Instant;
}

/// A clock reference is compatible with another reference.
///
/// The purpose of this trait is to restrict availability of the `async` functions, as they
/// wait real amounts of time - if a measurement is "off" or fake, the async function would still
/// cause the program to wait, but be unable to make progress because the rate limiter is on
/// another clock.  
pub trait CompatibleConversion<O>: Reference {}

impl Reference for Duration {
    fn duration_since(&self, earlier: Self) -> Nanos {
        self.checked_sub(earlier)
            .unwrap_or_else(|| Duration::new(0, 0))
            .into()
    }

    fn saturating_sub(&self, duration: Nanos) -> Self {
        self.checked_sub(duration.into()).unwrap_or(*self)
    }
}

/// Every instant is compatible with itself.
impl<T: Reference> CompatibleConversion<T> for T {}

impl Add<Nanos> for Duration {
    type Output = Self;

    fn add(self, other: Nanos) -> Self {
        let other: Duration = other.into();
        self + other
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
    pub fn advance(&self, by: Duration) {
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
    type Instant = Nanos;

    fn now(&self) -> Self::Instant {
        self.now.load(Ordering::Relaxed).into()
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

#[cfg(all(feature = "std", feature = "quanta"))]
mod quanta;
#[cfg(all(feature = "std", feature = "quanta"))]
pub use self::quanta::*;

mod default;

use crate::nanos::Nanos;
pub use default::*;
