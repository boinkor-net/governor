use governor::{
    clock::{Clock, FakeRelativeClock},
    DirectRateLimiter, NegativeMultiDecision, Quota,
};
use nonzero_ext::nonzero;
use std::time::Duration;

#[test]
fn accepts_first_cell() {
    let clock = FakeRelativeClock::default();
    let lb = DirectRateLimiter::new_with_clock(Quota::per_second(nonzero!(5u32)), &clock);
    assert_eq!(Ok(()), lb.check());
}

#[test]
fn rejects_too_many() {
    let mut clock = FakeRelativeClock::default();
    let lb = DirectRateLimiter::new_with_clock(Quota::per_second(nonzero!(2u32)), &clock);
    let ms = Duration::from_millis(1);

    // use up our burst capacity (2 in the first second):
    assert_eq!(Ok(()), lb.check(), "Now: {:?}", clock.now());
    clock.advance(ms * 1);
    assert_eq!(Ok(()), lb.check(), "Now: {:?}", clock.now());

    clock.advance(ms * 1);
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
    let mut clock = FakeRelativeClock::default();
    let lb = DirectRateLimiter::new_with_clock(Quota::per_second(nonzero!(2u32)), &clock);
    let ms = Duration::from_millis(1);
    let one = nonzero!(1u32);

    // use up our burst capacity (2 in the first second):
    assert_eq!(Ok(()), lb.check_all(one), "Now: {:?}", clock.now());
    clock.advance(ms * 1);
    assert_eq!(Ok(()), lb.check_all(one), "Now: {:?}", clock.now());

    clock.advance(ms * 1);
    assert_ne!(Ok(()), lb.check_all(one), "Now: {:?}", clock.now());

    // should be ok again in 1s:
    clock.advance(ms * 1000);
    assert_eq!(Ok(()), lb.check_all(one), "Now: {:?}", clock.now());
    clock.advance(ms);
    assert_eq!(Ok(()), lb.check_all(one));

    clock.advance(ms);
    assert_ne!(Ok(()), lb.check_all(one), "{:?}", lb);
}

#[test]
fn never_allows_more_than_capacity_all() {
    let mut clock = FakeRelativeClock::default();
    let lb = DirectRateLimiter::new_with_clock(Quota::per_second(nonzero!(4u32)), &clock);
    let ms = Duration::from_millis(1);

    // Use up the burst capacity:
    assert_eq!(
        Ok(()),
        lb.check_all(nonzero!(2u32)),
        "Now: {:?}",
        clock.now()
    );
    assert_eq!(
        Ok(()),
        lb.check_all(nonzero!(2u32)),
        "Now: {:?}",
        clock.now()
    );

    clock.advance(ms * 1);
    assert_ne!(
        Ok(()),
        lb.check_all(nonzero!(2u32)),
        "Now: {:?}",
        clock.now()
    );

    // should be ok again in 1s:
    clock.advance(ms * 1000);
    assert_eq!(
        Ok(()),
        lb.check_all(nonzero!(2u32)),
        "Now: {:?}",
        clock.now()
    );
    clock.advance(ms);
    assert_eq!(Ok(()), lb.check_all(nonzero!(2u32)));

    clock.advance(ms);
    assert_ne!(Ok(()), lb.check_all(nonzero!(2u32)), "{:?}", lb);
}

#[test]
fn rejects_too_many_all() {
    let mut clock = FakeRelativeClock::default();
    let lb = DirectRateLimiter::new_with_clock(Quota::per_second(nonzero!(5u32)), &clock);
    let ms = Duration::from_millis(1);

    // Should not allow the first 15 cells on a capacity 5 bucket:
    assert_ne!(Ok(()), lb.check_all(nonzero!(15u32)));

    // After 3 and 20 seconds, it should not allow 15 on that bucket either:
    clock.advance(ms * 3 * 1000);
    assert_ne!(Ok(()), lb.check_all(nonzero!(15u32)));

    clock.advance(ms * 17 * 1000);
    let result = lb.check_all(nonzero!(15u32));
    match result {
        Err(NegativeMultiDecision::InsufficientCapacity(n)) => assert_eq!(n, 5),
        _ => panic!("Did not expect {:?}", result),
    }
}

#[test]
fn correct_wait_time() {
    let mut clock = FakeRelativeClock::default();
    // Bucket adding a new element per 200ms:
    let lb = DirectRateLimiter::new_with_clock(Quota::per_second(nonzero!(5u32)), &clock);
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

    let mut clock = FakeRelativeClock::default();
    let lim = DirectRateLimiter::new_with_clock(Quota::per_second(nonzero!(20u32)), &clock);
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

/*
#[test]
fn too_early_wait_time_from() {
    let lim =
        LeakyBucket::construct(nonzero!(1u32), nonzero!(1u32), Duration::from_secs(1)).unwrap();
    let state = <LeakyBucket as Algorithm>::BucketState::default();
    let now = current_moment();
    let ms = Duration::from_millis(1);
    lim.test_and_update(&state, now).unwrap();
    if let Err(failure) = lim.test_and_update(&state, now) {
        assert_eq!(ms * 1000, failure.wait_time_from(now));
        assert_eq!(Duration::new(0, 0), failure.wait_time_from(now + ms * 1000));
        assert_eq!(Duration::new(0, 0), failure.wait_time_from(now + ms * 2001));
    } else {
        assert!(false, "Second attempt should fail");
    }
}
*/
