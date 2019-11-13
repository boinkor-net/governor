use derive_more::*;
use std::convert::TryInto;
use std::ops::{Add, Sub};
use std::time::Duration;

/// A number of nanoseconds from a reference point.
///
/// Can not represent durations >584 years, but hopefully that
/// should not be a problem in real-world applications.
#[derive(Add, Sub, PartialEq, Eq, Default, Debug, From, Into, Clone, Copy, PartialOrd, Ord)]
pub(crate) struct Nanos(u64);

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
