use std::prelude::v1::*;

use std::num::NonZeroU32;
use std::time::Duration;

/// A rate-limiting quota.
///
/// Quotas are expressed in a positive number of "cells" (the maximum number of positive decisions /
/// allowed items until the rate limiter needs to replenish) and the amount of time for the rate
/// limiter to replenish a single cell.
///
/// Neither the number of cells nor the replenishment unit of time may be zero.
///
/// # Burst sizes
/// There are multiple ways of expressing the same quota: a quota given as `Quota::per_second(1)`
/// allows, on average, the same number of cells through as a quota given as `Quota::per_minute(60)`.
/// However, the quota of `Quota::per_minute(60)` has a burst size of 60 cells, meaning it is possible
/// to accommodate 60 cells in one go, followed by a minute of waiting.
///
/// Burst size gets really important when you construct a rate limiter that should allow multiple
/// elements through at one time (using [`RateLimiter.check_n`](struct.RateLimiter.html#method.check_n)
/// and its related functions): Only
/// at most as many cells can be let through in one call as are given as the burst size.
///
/// In other words, the burst size is the maximum number of cells that the rate limiter will ever
/// allow through without replenishing them.
///
/// # Examples
///
/// Construct a quota that allows 50 cells per second (replenishing at a rate of one cell
/// per 20 milliseconds), with a burst size of 50 cells, allowing a full rate limiter to allow 50
/// cells through at a time:   
/// ```rust
/// # use governor::Quota;
/// # use nonzero_ext::nonzero;
/// # use std::time::Duration;
/// let q = Quota::per_second(nonzero!(50u32));
/// assert_eq!(q, Quota::per_second(nonzero!(50u32)).allow_burst(nonzero!(50u32)));
/// assert_eq!(q.replenish_interval(), Duration::from_millis(20));
/// assert_eq!(q.burst_size().get(), 50);
/// // The Quota::new constructor is deprecated, but this constructs the equivalent quota:
/// #[allow(deprecated)]
/// assert_eq!(q, Quota::new(nonzero!(50u32), Duration::from_secs(1)).unwrap());
/// ```
///
/// Construct a quota that allows 2 cells per hour through (replenishing at a rate of one cell
/// per 30min), but allows bursting up to 90 cells at once:
/// ```rust
/// # use governor::Quota;
/// # use nonzero_ext::nonzero;
/// # use std::time::Duration;
/// let q = Quota::per_hour(nonzero!(2u32)).allow_burst(nonzero!(90u32));
/// assert_eq!(q.replenish_interval(), Duration::from_secs(30 * 60));
/// assert_eq!(q.burst_size().get(), 90);
/// ```
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Quota {
    pub(crate) max_burst: NonZeroU32,
    pub(crate) replenish_1_per: Duration,
}

/// Constructors for Quotas
impl Quota {
    /// Construct a quota for a number of cells per second. The given number of cells is also
    /// assumed to be the maximum burst size.
    pub const fn per_second(max_burst: NonZeroU32) -> Quota {
        let replenish_interval_ns = Duration::from_secs(1).as_nanos() / (max_burst.get() as u128);
        Quota {
            max_burst,
            replenish_1_per: Duration::from_nanos(replenish_interval_ns as u64),
        }
    }

    /// Construct a quota for a number of cells per 60-second period. The given number of cells is
    /// also assumed to be the maximum burst size.
    pub const fn per_minute(max_burst: NonZeroU32) -> Quota {
        let replenish_interval_ns = Duration::from_secs(60).as_nanos() / (max_burst.get() as u128);
        Quota {
            max_burst,
            replenish_1_per: Duration::from_nanos(replenish_interval_ns as u64),
        }
    }

    /// Construct a quota for a number of cells per 60-minute (3600-second) period. The given number
    /// of cells is also assumed to be the maximum burst size.
    pub const fn per_hour(max_burst: NonZeroU32) -> Quota {
        let replenish_interval_ns =
            Duration::from_secs(60 * 60).as_nanos() / (max_burst.get() as u128);
        Quota {
            max_burst,
            replenish_1_per: Duration::from_nanos(replenish_interval_ns as u64),
        }
    }

    /// Adjusts the maximum burst size for a quota to construct a rate limiter with a capacity
    /// for at most the given number of cells.
    pub const fn allow_burst(self, max_burst: NonZeroU32) -> Quota {
        Quota { max_burst, ..self }
    }

    /// Construct a quota for a given burst size, replenishing the entire burst size in that
    /// given unit of time.
    ///
    /// Returns `None` if the duration is zero.
    ///
    /// This constructor allows greater control over the resulting quota, but doesn't make
    /// as much intuitive sense as other methods of constructing the same quotas. Unless your
    /// quotas are given as "max burst size, and time it takes to replenish that burst size", you
    /// are better served by the [`Quota::per_second`] (and similar) constructors with the
    /// [`allow_burst`](#method.allow_burst) modifier.
    #[deprecated(
        since = "0.2.0",
        note = "This constructor is often confusing and non-intuitive. \
    Use the `per_(interval)` and `max_burst` constructors instead."
    )]
    pub fn new(max_burst: NonZeroU32, replenish_all_per: Duration) -> Option<Quota> {
        if replenish_all_per.as_nanos() == 0 {
            None
        } else {
            Some(Quota {
                max_burst,
                replenish_1_per: replenish_all_per / max_burst.get(),
            })
        }
    }
}

/// Retrieving information about a quota
impl Quota {
    /// The time it takes for a rate limiter with an exhausted burst budget to replenish
    /// a single element.
    pub const fn replenish_interval(&self) -> Duration {
        self.replenish_1_per
    }

    /// The maximum number of cells that can be allowed in one burst.
    pub const fn burst_size(&self) -> NonZeroU32 {
        self.max_burst
    }

    /// The time it takes to replenish the entire maximum burst size.
    pub const fn burst_size_replenished_in(&self) -> Duration {
        let fill_in_ns = self.replenish_1_per.as_nanos() * self.max_burst.get() as u128;
        Duration::from_nanos(fill_in_ns as u64)
    }
}
