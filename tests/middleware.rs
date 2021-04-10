use governor::{
    clock::{self, FakeRelativeClock},
    middleware::{RateLimitingMiddleware, StateInformationMiddleware, StateSnapshot},
    Quota, RateLimiter,
};
use nonzero_ext::nonzero;

#[derive(Debug, PartialEq)]
struct MyMW {}

impl RateLimitingMiddleware for MyMW {
    type PositiveOutcome = u16;

    fn allow<K>(_key: &K, _state: StateSnapshot) -> Self::PositiveOutcome {
        666
    }

    fn disallow<K, P: clock::Reference>(
        _key: &K,
        _state: StateSnapshot,
        _not_until: &governor::NotUntil<P, Self>,
    ) where
        Self: Sized,
    {
    }
}

#[test]
fn changes_allowed_type() {
    let clock = FakeRelativeClock::default();
    let lim = RateLimiter::direct_with_clock(Quota::per_second(nonzero!(4u32)), &clock)
        .with_middleware::<MyMW>();
    assert_eq!(Ok(666), lim.check());
}

#[test]
fn state_information() {
    let clock = FakeRelativeClock::default();
    let lim = RateLimiter::direct_with_clock(Quota::per_second(nonzero!(4u32)), &clock)
        .with_middleware::<StateInformationMiddleware>();
    assert_eq!(
        Ok(Some(3)),
        lim.check()
            .map(|outcome| outcome.remaining_burst_capacity())
    );
    assert_eq!(
        Ok(Some(2)),
        lim.check()
            .map(|outcome| outcome.remaining_burst_capacity())
    );
    assert_eq!(
        Ok(Some(1)),
        lim.check()
            .map(|outcome| outcome.remaining_burst_capacity())
    );
    assert_eq!(
        Ok(Some(0)),
        lim.check()
            .map(|outcome| outcome.remaining_burst_capacity())
    );
    assert!(lim.check().is_err());
}
