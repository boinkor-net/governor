use governor::{
    clock::{Clock, FakeRelativeClock},
    DefaultDirectRateLimiter, InsufficientCapacity, Quota, RateLimiter,
};
use nonzero_ext::nonzero;
use std::time::Duration;

#[test]
fn accepts_first_cell() {
    let clock = FakeRelativeClock::default();
    let lb = RateLimiter::direct_with_clock(Quota::per_second(nonzero!(5u32)), clock);
    assert_eq!(Ok(()), lb.check());
}

#[test]
fn rejects_too_many() {
    let clock = FakeRelativeClock::default();
    let lb = RateLimiter::direct_with_clock(Quota::per_second(nonzero!(2u32)), clock.clone());
    let ms = Duration::from_millis(1);

    // use up our burst capacity (2 in the first second):
    assert_eq!(Ok(()), lb.check(), "Now: {:?}", clock.now());
    clock.advance(ms);
    assert_eq!(Ok(()), lb.check(), "Now: {:?}", clock.now());

    clock.advance(ms);
    assert_ne!(Ok(()), lb.check(), "Now: {:?}", clock.now());

    // should be ok again in 1s:
    clock.advance(ms * 1000);
    assert_eq!(Ok(()), lb.check(), "Now: {:?}", clock.now());
    clock.advance(ms);
    assert_eq!(Ok(()), lb.check());

    clock.advance(ms);
    assert_ne!(Ok(()), lb.check(), "{:?}", lb);
}

#[test]
fn all_1_identical_to_1() {
    let clock = FakeRelativeClock::default();
    let lb = RateLimiter::direct_with_clock(Quota::per_second(nonzero!(2u32)), clock.clone());
    let ms = Duration::from_millis(1);
    let one = nonzero!(1u32);

    // use up our burst capacity (2 in the first second):
    assert_eq!(Ok(Ok(())), lb.check_n(one), "Now: {:?}", clock.now());
    clock.advance(ms);
    assert_eq!(Ok(Ok(())), lb.check_n(one), "Now: {:?}", clock.now());

    clock.advance(ms);
    assert_ne!(Ok(Ok(())), lb.check_n(one), "Now: {:?}", clock.now());

    // should be ok again in 1s:
    clock.advance(ms * 1000);
    assert_eq!(Ok(Ok(())), lb.check_n(one), "Now: {:?}", clock.now());
    clock.advance(ms);
    assert_eq!(Ok(Ok(())), lb.check_n(one));

    clock.advance(ms);
    assert_ne!(Ok(Ok(())), lb.check_n(one), "{:?}", lb);
}

#[test]
fn never_allows_more_than_capacity_all() {
    let clock = FakeRelativeClock::default();
    let lb = RateLimiter::direct_with_clock(Quota::per_second(nonzero!(4u32)), clock.clone());
    let ms = Duration::from_millis(1);

    // Use up the burst capacity:
    assert_eq!(Ok(Ok(())), lb.check_n(nonzero!(2u32)));
    assert_eq!(Ok(Ok(())), lb.check_n(nonzero!(2u32)));

    clock.advance(ms);
    assert_ne!(Ok(Ok(())), lb.check_n(nonzero!(2u32)));

    // should be ok again in 1s:
    clock.advance(ms * 1000);
    assert_eq!(Ok(Ok(())), lb.check_n(nonzero!(2u32)));
    clock.advance(ms);
    assert_eq!(Ok(Ok(())), lb.check_n(nonzero!(2u32)));

    clock.advance(ms);
    assert_ne!(Ok(Ok(())), lb.check_n(nonzero!(2u32)), "{:?}", lb);
}

#[test]
fn rejects_too_many_all() {
    let clock = FakeRelativeClock::default();
    let lb = RateLimiter::direct_with_clock(Quota::per_second(nonzero!(5u32)), clock.clone());
    let ms = Duration::from_millis(1);

    // Should not allow the first 15 cells on a capacity 5 bucket:
    assert_ne!(Ok(Ok(())), lb.check_n(nonzero!(15u32)));

    // After 3 and 20 seconds, it should not allow 15 on that bucket either:
    clock.advance(ms * 3 * 1000);
    assert_ne!(Ok(Ok(())), lb.check_n(nonzero!(15u32)));
}

#[test]
fn all_capacity_check_rejects_excess() {
    let clock = FakeRelativeClock::default();
    let lb = RateLimiter::direct_with_clock(Quota::per_second(nonzero!(5u32)), clock);

    assert_eq!(Err(InsufficientCapacity(5)), lb.check_n(nonzero!(15u32)));
    assert_eq!(Err(InsufficientCapacity(5)), lb.check_n(nonzero!(6u32)));
    assert_eq!(Err(InsufficientCapacity(5)), lb.check_n(nonzero!(7u32)));
}

#[test]
fn correct_wait_time() {
    let clock = FakeRelativeClock::default();
    // Bucket adding a new element per 200ms:
    let lb = RateLimiter::direct_with_clock(Quota::per_second(nonzero!(5u32)), clock.clone());
    let ms = Duration::from_millis(1);
    let mut conforming = 0;
    for _i in 0..20 {
        clock.advance(ms);
        let res = lb.check();
        match res {
            Ok(()) => {
                conforming += 1;
            }
            Err(wait) => {
                clock.advance(wait.wait_time_from(clock.now()));
                assert_eq!(Ok(()), lb.check());
                conforming += 1;
            }
        }
    }
    assert_eq!(20, conforming);
}

#[test]
fn actual_threadsafety() {
    use crossbeam;

    let clock = FakeRelativeClock::default();
    let lim = RateLimiter::direct_with_clock(Quota::per_second(nonzero!(20u32)), clock.clone());
    let ms = Duration::from_millis(1);

    crossbeam::scope(|scope| {
        for _i in 0..20 {
            scope.spawn(|_| {
                assert_eq!(Ok(()), lim.check());
            });
        }
    })
    .unwrap();

    clock.advance(ms * 2);
    assert_ne!(Ok(()), lim.check());
    clock.advance(ms * 998);
    assert_eq!(Ok(()), lim.check());
}

#[test]
fn default_direct() {
    let clock = governor::clock::DefaultClock::default();
    let limiter: DefaultDirectRateLimiter =
        RateLimiter::direct_with_clock(Quota::per_second(nonzero!(20u32)), clock);
    assert_eq!(Ok(()), limiter.check());
}

#[cfg(feature = "std")]
#[test]
fn stresstest_large_quotas() {
    use std::{sync::Arc, thread};

    use governor::middleware::StateInformationMiddleware;

    let quota = Quota::per_second(nonzero!(1_000_000_001u32));
    let rate_limiter =
        Arc::new(RateLimiter::direct(quota).with_middleware::<StateInformationMiddleware>());

    fn rlspin(rl: Arc<DefaultDirectRateLimiter<StateInformationMiddleware>>) {
        for _ in 0..1_000_000 {
            rl.check().map_err(|e| dbg!(e)).unwrap();
        }
    }

    let rate_limiter2 = rate_limiter.clone();
    thread::spawn(move || {
        rlspin(rate_limiter2);
    });
    rlspin(rate_limiter);
}
