use core::convert::TryFrom;
use core::num::NonZeroU32;
use core::time::Duration;

use nonzero_ext::nonzero;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::nanos::Nanos;

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
/// However, the quota of `Quota::per_minute(60)` has a burst size of 60 cells, meaning it is
/// possible to accomodate 60 cells in one go, after which the equivalent of a minute of inactivity
/// is required for the burst allowance to be fully restored.
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
/// // The entire maximum burst size will be restored if no cells are let through for 45 hours:
/// assert_eq!(q.burst_size_replenished_in(), Duration::from_secs(60 * 60 * (90 / 2)));
/// ```
///
/// You can also use a human readable string to build a [Quota]. The format is
/// `<max_burst> ('per'|/) <duration> <unit>`. The string itself is case-insensitive. Acceptable
/// `unit`s are: `second(s)`, `minute(s)`, `hour(s)`.
/// ```rust
/// # use core::time::Duration;
/// # use core::convert::TryFrom;
/// # use governor::Quota;
/// # use nonzero_ext::nonzero;
/// assert_eq!(Quota::try_from("5 per second").unwrap(), Quota::per_second(nonzero!(5u32)));
/// assert_eq!(Quota::try_from("15 / Minute").unwrap(), Quota::per_minute(nonzero!(15u32)));
/// assert_eq!(
///     Quota::try_from("30 per 3 hours").unwrap(),
///     Quota::with_period(Duration::from_nanos(
///         (Duration::from_secs(3 * 60 * 60).as_nanos() / 30u128) as u64
///     ))
///     .unwrap()
///     .allow_burst(nonzero!(30u32))
/// );
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

    /// Construct a quota that replenishes one cell in a given
    /// interval.
    ///
    /// This constructor is meant to replace [`::new`](#method.new),
    /// in cases where a longer refresh period than 1 cell/hour is
    /// necessary.
    ///
    /// If the time interval is zero, returns `None`.
    ///
    /// # Example
    /// ```rust
    /// # use nonzero_ext::nonzero;
    /// # use governor::Quota;
    /// # use std::time::Duration;
    /// // Replenish one cell per day, with a burst capacity of 10 cells:
    /// let _quota = Quota::with_period(Duration::from_secs(60 * 60 * 24))
    ///     .unwrap()
    ///     .allow_burst(nonzero!(10u32));
    /// ```
    pub fn with_period(replenish_1_per: Duration) -> Option<Quota> {
        if replenish_1_per.as_nanos() == 0 {
            None
        } else {
            Some(Quota {
                max_burst: nonzero!(1u32),
                replenish_1_per,
            })
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
    /// This constructor allows greater control over the resulting
    /// quota, but doesn't make as much intuitive sense as other
    /// methods of constructing the same quotas. Unless your quotas
    /// are given as "max burst size, and time it takes to replenish
    /// that burst size", you are better served by the
    /// [`Quota::per_second`](#method.per_second) (and similar)
    /// constructors with the [`allow_burst`](#method.allow_burst)
    /// modifier.
    #[deprecated(
        since = "0.2.0",
        note = "This constructor is often confusing and non-intuitive. \
    Use the `per_(interval)` / `with_period` and `max_burst` constructors instead."
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

impl Quota {
    /// A way to reconstruct a Quota from an in-use Gcra.
    ///
    /// This is useful mainly for [`crate::middleware::RateLimitingMiddleware`]
    /// where custom code may want to construct information based on
    /// the amount of burst balance remaining.
    pub(crate) fn from_gcra_parameters(t: Nanos, tau: Nanos) -> Quota {
        // Safety assurance: by construction, the computed value is bounded by
        // one at the lower.
        //
        // The casts may look a little sketch, but they're constructed from
        // parameters that came from the crate exactly like that.
        let max_burst =
            unsafe { NonZeroU32::new_unchecked(1 + (tau.as_u64() / t.as_u64()) as u32) };
        let replenish_1_per = t.into();
        Quota {
            max_burst,
            replenish_1_per,
        }
    }
}

static HUMAN_READABLE_QUOTA_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?ix)
(?<max_burst>[0-9]+)              # max burst
\s*(?:/|\s*per\s*)                # separators: '/' or 'per'
\s*(?<duration>[0-9]+)*           # duration
\s*(?<unit>second|minute|hour)s?  # duration unit
",
    )
    .unwrap()
});

impl TryFrom<&str> for Quota {
    type Error = Box<dyn core::error::Error>;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if let Some(captures) = HUMAN_READABLE_QUOTA_REGEX.captures(value) {
            let max_burst = captures
                .name("max_burst")
                .ok_or(format!("Invalid rate limit string: '{value}'"))?
                .as_str()
                .parse::<NonZeroU32>()?;
            let duration = captures
                .name("duration")
                .map(|m| m.as_str())
                .unwrap_or("1")
                .parse::<u64>()?;
            let unit = captures
                .name("unit")
                .ok_or(format!("Invalid rate limit string: '{value}'"))?
                .as_str();
            let duration = match unit.to_lowercase().as_str() {
                "second" => duration,
                "minute" => duration * 60,
                "hour" => duration * 60 * 60,
                _ => {
                    return Err(format!("Unknown duration unit '{unit}'. Acceptable values are second(s)/minute(s)/hour(s)").into());
                }
            };
            let replenish_1_per =
                Duration::from_secs(duration).as_nanos() / (max_burst.get() as u128);
            match Quota::with_period(Duration::from_nanos(replenish_1_per as u64)) {
                Some(quota) => Ok(quota.allow_burst(max_burst)),
                None => Err(format!("Invalid period '{duration}'").into()),
            }
        } else {
            Err(format!("Unable to parse quota: '{value}'").into())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use nonzero_ext::nonzero;

    #[test]
    fn time_multiples() {
        let hourly = Quota::per_hour(nonzero!(1u32));
        let minutely = Quota::per_minute(nonzero!(1u32));
        let secondly = Quota::per_second(nonzero!(1u32));

        assert_eq!(
            hourly.replenish_interval() / 60,
            minutely.replenish_interval()
        );
        assert_eq!(
            minutely.replenish_interval() / 60,
            secondly.replenish_interval()
        );
    }

    #[test]
    fn period_error_cases() {
        assert!(Quota::with_period(Duration::from_secs(0)).is_none());

        #[allow(deprecated)]
        {
            assert!(Quota::new(nonzero!(1u32), Duration::from_secs(0)).is_none());
        }
    }

    #[test]
    fn test_parse() {
        assert_eq!(
            Quota::per_second(nonzero!(1u32)),
            Quota::try_from("1 per second").unwrap()
        );
        assert_eq!(
            Quota::per_second(nonzero!(15u32)),
            Quota::try_from("15 / Second").unwrap()
        );
        assert_eq!(
            Quota::per_minute(nonzero!(3u32)),
            Quota::try_from("3/MINUTE").unwrap()
        );
        assert_eq!(
            Quota::per_hour(nonzero!(17u32)),
            Quota::try_from("17/Hour").unwrap()
        );
        assert_eq!(
            Quota::with_period(Duration::from_secs(3 * 60 * 60))
                .unwrap()
                .allow_burst(nonzero!(1u32)),
            Quota::try_from("1 per 3 hours").unwrap()
        );
        assert_eq!(
            Quota::with_period(Duration::from_secs(15 * 60))
                .unwrap()
                .allow_burst(nonzero!(5u32)),
            Quota::try_from("5 per 15 mInUtEs").unwrap()
        );
    }
}
