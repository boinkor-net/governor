#![cfg(all(feature = "std", feature = "dashmap"))]

use governor::{
    clock::{Clock, FakeRelativeClock},
    NegativeMultiDecision, Quota, RateLimiter,
};
use nonzero_ext::nonzero;
use std::time::Duration;

const KEYS: &[u32] = &[1u32, 2u32];

#[test]
fn accepts_first_cell() {
    let clock = FakeRelativeClock::default();
    let lb = RateLimiter::dashmap_with_clock(Quota::per_second(nonzero!(5u32)), &clock);
    for key in KEYS {
        assert_eq!(Ok(()), lb.check_key(&key), "key {:?}", key);
    }
}

#[test]
fn rejects_too_many() {
    let mut clock = FakeRelativeClock::default();
    let lb = RateLimiter::dashmap_with_clock(Quota::per_second(nonzero!(2u32)), &clock);
    let ms = Duration::from_millis(1);

    for key in KEYS {
        // use up our burst capacity (2 in the first second):
        assert_eq!(Ok(()), lb.check_key(key), "Now: {:?}", clock.now());
        clock.advance(ms * 1);
        assert_eq!(Ok(()), lb.check_key(key), "Now: {:?}", clock.now());

        clock.advance(ms * 1);
        assert_ne!(Ok(()), lb.check_key(key), "Now: {:?}", clock.now());

        // should be ok again in 1s:
        clock.advance(ms * 1000);
        assert_eq!(Ok(()), lb.check_key(key), "Now: {:?}", clock.now());
        clock.advance(ms);
        assert_eq!(Ok(()), lb.check_key(key));

        clock.advance(ms);
        assert_ne!(Ok(()), lb.check_key(key), "{:?}", lb);
    }
}

#[test]
fn actual_threadsafety() {
    use crossbeam;

    let mut clock = FakeRelativeClock::default();
    let lim = RateLimiter::dashmap_with_clock(Quota::per_second(nonzero!(20u32)), &clock);
    let ms = Duration::from_millis(1);

    for key in KEYS {
        crossbeam::scope(|scope| {
            for _i in 0..20 {
                scope.spawn(|_| {
                    assert_eq!(Ok(()), lim.check_key(key));
                });
            }
        })
        .unwrap();

        clock.advance(ms * 2);
        assert_ne!(Ok(()), lim.check_key(key));
        clock.advance(ms * 998);
        assert_eq!(Ok(()), lim.check_key(key));
    }
}
