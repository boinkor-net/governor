//! A time-keeping abstraction (nanoseconds) that works for storing in an atomic integer.

use crate::clock;

use std::convert::TryInto;
use std::fmt;
use std::ops::{Add, Div, Mul};
use std::prelude::v1::*;
use std::time::Duration;

/// A number of nanoseconds from a reference point.
///
/// Nanos can not represent durations >584 years, but hopefully that
/// should not be a problem in real-world applications.
#[derive(PartialEq, Eq, Default, Clone, Copy, PartialOrd, Ord)]
pub struct Nanos(u64);

impl Nanos {
    pub(crate) fn as_u64(self) -> u64 {
        self.0
    }
}

/// Nanos as used by Jitter and other std-only features.
#[cfg(feature = "std")]
impl Nanos {
    pub(crate) const fn new(u: u64) -> Self {
        Nanos(u)
    }
}

impl From<Duration> for Nanos {
    fn from(d: Duration) -> Self {
        // This will panic:
        Nanos(
            d.as_nanos()
                .try_into()
                .expect("Duration is longer than 584 years"),
        )
    }
}

impl fmt::Debug for Nanos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let d = Duration::from_nanos(self.0);
        write!(f, "Nanos({:?})", d)
    }
}

impl Add<Nanos> for Nanos {
    type Output = Nanos;

    fn add(self, rhs: Nanos) -> Self::Output {
        Nanos(self.0 + rhs.0)
    }
}

impl Mul<u64> for Nanos {
    type Output = Nanos;

    fn mul(self, rhs: u64) -> Self::Output {
        Nanos(self.0 * rhs)
    }
}

impl Div<Nanos> for Nanos {
    type Output = u64;

    fn div(self, rhs: Nanos) -> Self::Output {
        self.0 / rhs.0
    }
}

impl From<u64> for Nanos {
    fn from(u: u64) -> Self {
        Nanos(u)
    }
}

impl Into<u64> for Nanos {
    fn into(self) -> u64 {
        self.0
    }
}

impl Into<Duration> for Nanos {
    fn into(self) -> Duration {
        Duration::from_nanos(self.0)
    }
}

impl Nanos {
    pub(crate) fn saturating_sub(self, rhs: Nanos) -> Nanos {
        Nanos(self.0.saturating_sub(rhs.0))
    }
}

impl clock::Reference for Nanos {
    fn duration_since(&self, earlier: Self) -> Nanos {
        (*self as Nanos).saturating_sub(earlier)
    }

    fn saturating_sub(&self, duration: Nanos) -> Self {
        (*self as Nanos).saturating_sub(duration)
    }
}

impl Add<Duration> for Nanos {
    type Output = Self;

    fn add(self, other: Duration) -> Self {
        let other: Nanos = other.into();
        self + other
    }
}
