//! A time-keeping abstraction (nanoseconds) that works for storing in an atomic integer.

use crate::lib::*;
use derive_more::*;

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

#[test]
#[should_panic(expected = "Duration is longer than 584 years")]
fn panics_on_overflow() {
    let _: Nanos = Duration::from_secs(600 * 366 * 24 * 60 * 60).into();
}
