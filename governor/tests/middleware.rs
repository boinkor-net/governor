use governor::{
    clock::{self, FakeRelativeClock},
    middleware::{RateLimitingMiddleware, StateInformationMiddleware, StateSnapshot},
    Quota, RateLimiter,
};
use nonzero_ext::nonzero;

#[derive(Debug)]
struct MyMW;

impl RateLimitingMiddleware<<FakeRelativeClock as clock::Clock>::Instant> for MyMW {
    type PositiveOutcome = u16;

    fn allow<K>(_key: &K, _state: impl Into<StateSnapshot>) -> Self::PositiveOutcome {
        666
    }

    type NegativeOutcome = ();

    fn disallow<K>(
        _key: &K,
        _limiter: impl Into<StateSnapshot>,
        _start_time: <FakeRelativeClock as clock::Clock>::Instant,
    ) -> Self::NegativeOutcome {
    }
}

#[test]
fn changes_allowed_type() {
    let clock = FakeRelativeClock::default();
    let lim = RateLimiter::direct_with_clock(Quota::per_hour(nonzero!(1_u32)), clock.clone())
        .with_middleware::<MyMW>();
    assert_eq!(Ok(666), lim.check());
    assert_eq!(Err(()), lim.check());
}

#[test]
fn state_information() {
    let clock = FakeRelativeClock::default();
    let lim = RateLimiter::direct_with_clock(Quota::per_second(nonzero!(4u32)), clock.clone())
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
    assert_eq!(format!("{:?}", MyMW), "MyMW");
}

#[test]
fn state_snapshot_tracks_quota_accurately() {
    use governor::middleware::StateInformationMiddleware;
    use governor::{Quota, RateLimiter};
    use std::num::NonZeroU32;
    use std::time::Duration;

    let burst_size = NonZeroU32::new(2).unwrap();
    let period = Duration::from_millis(90);
    let quota = Quota::with_period(period).unwrap().allow_burst(burst_size);

    let clock = FakeRelativeClock::default();

    // First test
    let lim = RateLimiter::direct_with_clock(quota, clock.clone())
        .with_middleware::<StateInformationMiddleware>();

    assert_eq!(lim.check().unwrap().remaining_burst_capacity(), 1);
    assert_eq!(lim.check().unwrap().remaining_burst_capacity(), 0);
    assert_eq!(lim.check().map_err(|_| ()), Err(()), "should rate limit");

    clock.advance(Duration::from_secs(120));
    assert_eq!(lim.check().map(|s| s.remaining_burst_capacity()), Ok(2));
    assert_eq!(lim.check().map(|s| s.remaining_burst_capacity()), Ok(1));
    assert_eq!(lim.check().map(|s| s.remaining_burst_capacity()), Ok(0));
    assert_eq!(lim.check().map_err(|_| ()), Err(()), "should rate limit");
}

#[test]
#[cfg(feature = "std")]
fn state_snapshot_tracks_quota_accurately_with_real_clock() {
    use governor::middleware::StateInformationMiddleware;
    use governor::{Quota, RateLimiter};
    use std::num::NonZeroU32;
    use std::time::Duration;

    let burst_size = NonZeroU32::new(2).unwrap();
    let period = Duration::from_millis(90);
    let quota = Quota::with_period(period).unwrap().allow_burst(burst_size);
    let lim = RateLimiter::direct(quota).with_middleware::<StateInformationMiddleware>();

    assert_eq!(lim.check().unwrap().remaining_burst_capacity(), 1);
    assert_eq!(lim.check().unwrap().remaining_burst_capacity(), 0);
    assert_eq!(lim.check().map_err(|_| ()), Err(()), "should rate limit");
}
