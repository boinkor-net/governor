use core::fmt;

use governor::{
    clock::{self, FakeRelativeClock},
    middleware::{RateLimitingMiddleware, StateInformationMiddleware, StateSnapshot},
    Quota, RateLimiter,
};
use nonzero_ext::nonzero;

#[derive(Debug)]
struct MyMW;

#[derive(Debug, PartialEq)]
struct NotAllowed;

impl fmt::Display for NotAllowed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Not allowed yet")
    }
}

impl RateLimitingMiddleware<<FakeRelativeClock as clock::Clock>::Instant> for MyMW {
    type PositiveOutcome = u16;

    fn allow<K>(_key: &K, _state: impl Into<StateSnapshot>) -> Self::PositiveOutcome {
        666
    }

    type NegativeOutcome = NotAllowed;

    fn disallow<K>(
        _key: &K,
        _limiter: impl Into<StateSnapshot>,
        _start_time: <FakeRelativeClock as clock::Clock>::Instant,
    ) -> Self::NegativeOutcome
    where
        Self: Sized,
    {
        NotAllowed
    }
}

#[test]
fn changes_allowed_type() {
    let clock = FakeRelativeClock::default();
    let lim = RateLimiter::direct_with_clock(Quota::per_hour(nonzero!(1_u32)), &clock)
        .with_middleware::<MyMW>();
    assert_eq!(Ok(666), lim.check());
    assert_eq!(Err(NotAllowed), lim.check());
}

#[test]
fn state_information() {
    let clock = FakeRelativeClock::default();
    let lim = RateLimiter::direct_with_clock(Quota::per_second(nonzero!(4u32)), &clock)
        .with_middleware::<StateInformationMiddleware>();
    assert_eq!(
        Ok(3),
        lim.check()
            .map(|outcome| outcome.remaining_burst_capacity())
    );
    assert_eq!(
        Ok(2),
        lim.check()
            .map(|outcome| outcome.remaining_burst_capacity())
    );
    assert_eq!(
        Ok(1),
        lim.check()
            .map(|outcome| outcome.remaining_burst_capacity())
    );
    assert_eq!(
        Ok(0),
        lim.check()
            .map(|outcome| outcome.remaining_burst_capacity())
    );
    assert!(lim.check().is_err());
}

#[test]
#[cfg(feature = "std")]
fn mymw_derives() {
    assert_eq!(NotAllowed, NotAllowed);
    assert_eq!(format!("{}", NotAllowed), "Not allowed yet");
    assert_eq!(format!("{:?}", MyMW), "MyMW");
}
