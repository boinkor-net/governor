use crate::lib::*;

/// A rate-limiting quota.
///
/// Quotas are expressed in a positive number of "cells" (the number of positive decisions /
/// allowed items) per unit of time.
///
/// Neither the number of cells nor the unit of time may be zero.
///
/// # Burst sizes
/// There are multiple ways of expressing the same quota: a quota given as `Quota::per_second(1)`
/// allows, on average, the same number of cells through as a quota given as `Quota::per_minute(60)`.
/// However, the quota of `Quota::per_minute(60)` has a burst size of 60 cells, meaning it is possible
/// to accommodate 60 cells in one go, followed by a minute of waiting.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Quota {
    pub(crate) max_burst: NonZeroU32,
    pub(crate) per: Duration,
}

impl Quota {
    /// Construct a quota for a number of cells per second.
    pub const fn per_second(max_burst: NonZeroU32) -> Quota {
        Quota {
            max_burst,
            per: Duration::from_secs(1),
        }
    }

    /// Construct a quota for a number of cells per 60-second period.
    pub const fn per_minute(max_burst: NonZeroU32) -> Quota {
        Quota {
            max_burst,
            per: Duration::from_secs(60),
        }
    }

    /// Construct a quota for a number of cells per 60-minute (3600-second) period.
    pub const fn per_hour(max_burst: NonZeroU32) -> Quota {
        Quota {
            max_burst,
            per: Duration::from_secs(60 * 60),
        }
    }

    /// Construct a quota for a given burst size per unit of time.
    ///
    /// Returns `None` if the duration is zero.
    pub fn new(max_burst: NonZeroU32, per: Duration) -> Option<Quota> {
        if per.as_nanos() == 0 {
            None
        } else {
            Some(Quota { max_burst, per })
        }
    }
}
