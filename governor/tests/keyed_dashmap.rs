#![cfg(all(feature = "std", feature = "dashmap"))]

use governor::{
    clock::{Clock, FakeRelativeClock},
    Quota, RateLimiter,
};
use governor::{middleware::NoOpMiddleware, state::keyed::DashMapStateStore};
use nonzero_ext::nonzero;
use std::hash::Hash;
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
    let clock = FakeRelativeClock::default();
    let lb = RateLimiter::dashmap_with_clock(Quota::per_second(nonzero!(2u32)), &clock);
    let ms = Duration::from_millis(1);

    for key in KEYS {
        // use up our burst capacity (2 in the first second):
        assert_eq!(Ok(()), lb.check_key(key), "Now: {:?}", clock.now());
        clock.advance(ms);
        assert_eq!(Ok(()), lb.check_key(key), "Now: {:?}", clock.now());

        clock.advance(ms);
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
fn rejects_too_many_n() {
    let clock = FakeRelativeClock::default();
    let lb = RateLimiter::dashmap_with_clock(Quota::per_second(nonzero!(2u32)), &clock);
    let ms = Duration::from_millis(1);

    for key in KEYS {
        // use up our burst capacity (2 in the first second):
        assert!(lb.check_key_n(key, nonzero!(2u32)).unwrap().is_ok());
        clock.advance(ms);

        assert_ne!(Ok(()), lb.check_key(key), "Now: {:?}", clock.now());

        // should be ok again in 1s:
        clock.advance(ms * 1000);
        assert!(lb.check_key_n(key, nonzero!(2u32)).unwrap().is_ok());
        clock.advance(ms);

        assert_ne!(Ok(()), lb.check_key(key), "{:?}", lb);
    }
}

#[test]
fn peek_does_not_change_the_decision() {
    let clock = FakeRelativeClock::default();
    let lb = RateLimiter::dashmap_with_clock(Quota::per_second(nonzero!(2u32)), &clock);
    let ms = Duration::from_millis(1);

    for key in KEYS {
        // no key is always positive outcome
        for _ in 0..10 {
            clock.advance(ms);
            assert!(lb.peek_key(key).is_ok());
        }

        // use up our burst capacity (2 in the first second):
        assert!(lb.check_key(key).is_ok());

        for _ in 0..10 {
            clock.advance(ms);
            assert!(lb.peek_key(key).is_ok());
        }

        clock.advance(ms);
        assert!(lb.check_key(key).is_ok());

        for _ in 0..10 {
            clock.advance(ms);
            assert_ne!(Ok(()), lb.peek_key(key));
        }

        clock.advance(ms);
        assert_ne!(Ok(()), lb.check_key(key));

        for _ in 0..10 {
            clock.advance(ms);
            assert_ne!(Ok(()), lb.peek_key(key));
        }

        // should be ok again in 1s:
        clock.advance(ms * 1000);
        assert!(lb.check_key(key).is_ok());
        clock.advance(ms);

        for _ in 0..10 {
            clock.advance(ms);
            assert!(lb.peek_key(key).is_ok());
        }

        assert!(lb.check_key(key).is_ok());

        clock.advance(ms);
        assert_ne!(Ok(()), lb.check_key(key));

        for _ in 0..10 {
            clock.advance(ms);
            assert_ne!(Ok(()), lb.peek_key(key));
        }
    }
}

#[test]
fn peek_does_not_change_the_decision_n() {
    let clock = FakeRelativeClock::default();
    let lb = RateLimiter::dashmap_with_clock(Quota::per_second(nonzero!(2u32)), &clock);
    let ms = Duration::from_millis(1);

    for key in KEYS {
        // no key is always positive outcome
        assert!(lb.peek_key_n(key, nonzero!(3u32)).is_err());
        assert!(lb.peek_key_n(key, nonzero!(2u32)).is_ok());

        // use up our burst capacity (2 in the first second):
        assert!(lb.check_key(key).is_ok());

        assert!(lb.peek_key_n(key, nonzero!(1u32)).is_ok());
        assert!(lb.peek_key_n(key, nonzero!(2u32)).is_ok());
        assert!(lb.peek_key_n(key, nonzero!(2u32)).unwrap().is_err());

        clock.advance(ms);
        assert!(lb.check_key(key).is_ok());

        assert_ne!(Ok(Ok(())), lb.peek_key_n(key, nonzero!(2u32)));

        clock.advance(ms);
        assert_ne!(Ok(()), lb.check_key(key));

        assert_ne!(Ok(Ok(())), lb.peek_key_n(key, nonzero!(2u32)));

        // should be ok again in 1s:
        clock.advance(ms * 1000);
        assert!(lb.peek_key_n(key, nonzero!(2u32)).is_ok());

        assert!(lb.check_key_n(key, nonzero!(2u32)).is_ok());
        assert!(lb.check_key_n(key, nonzero!(2u32)).is_ok());
        assert!(lb.peek_key_n(key, nonzero!(2u32)).unwrap().is_err());
    }
}

#[test]
fn hashmap_reset_does_work() {
    let clock = FakeRelativeClock::default();
    let lb = RateLimiter::hashmap_with_clock(Quota::per_second(nonzero!(2u32)), &clock);
    let ms = Duration::from_millis(1);

    for key in KEYS {
        // use up our burst capacity (2 in the first second):
        assert_eq!(Ok(()), lb.check_key(key), "Now: {:?}", clock.now());
        clock.advance(ms);
        assert_eq!(Ok(()), lb.check_key(key), "Now: {:?}", clock.now());

        for _ in 0..10 {
            clock.advance(ms);
            assert_ne!(Ok(()), lb.peek_key(key), "Now: {:?}", clock.now());
        }
        clock.advance(ms);
        assert_ne!(Ok(()), lb.check_key(key), "Now: {:?}", clock.now());

        lb.reset_key(key);

        for _ in 0..10 {
            clock.advance(ms);
            assert_eq!(Ok(()), lb.peek_key(key), "Now: {:?}", clock.now());
        }

        assert_eq!(Ok(()), lb.check_key(key), "Now: {:?}", clock.now());
        clock.advance(ms);
        lb.reset_key(key);
        assert_eq!(Ok(()), lb.check_key(key), "Now: {:?}", clock.now());
        clock.advance(ms);
        assert_eq!(Ok(()), lb.check_key(key), "Now: {:?}", clock.now());

        for _ in 0..10 {
            clock.advance(ms);
            assert_ne!(Ok(()), lb.peek_key(key), "Now: {:?}", clock.now());
        }
        clock.advance(ms);
        assert_ne!(Ok(()), lb.check_key(key), "Now: {:?}", clock.now());
    }
}

#[test]
fn dashmap_reset_does_work() {
    let clock = FakeRelativeClock::default();
    let lb = RateLimiter::dashmap_with_clock(Quota::per_second(nonzero!(2u32)), &clock);
    let ms = Duration::from_millis(1);

    for key in KEYS {
        // use up our burst capacity (2 in the first second):
        assert_eq!(Ok(()), lb.check_key(key), "Now: {:?}", clock.now());
        clock.advance(ms);
        assert_eq!(Ok(()), lb.check_key(key), "Now: {:?}", clock.now());

        for _ in 0..10 {
            clock.advance(ms);
            assert_ne!(Ok(()), lb.peek_key(key), "Now: {:?}", clock.now());
        }
        clock.advance(ms);
        assert_ne!(Ok(()), lb.check_key(key), "Now: {:?}", clock.now());

        lb.reset_key(key);

        for _ in 0..10 {
            clock.advance(ms);
            assert_eq!(Ok(()), lb.peek_key(key), "Now: {:?}", clock.now());
        }

        assert_eq!(Ok(()), lb.check_key(key), "Now: {:?}", clock.now());
        clock.advance(ms);
        lb.reset_key(key);
        assert_eq!(Ok(()), lb.check_key(key), "Now: {:?}", clock.now());
        clock.advance(ms);
        assert_eq!(Ok(()), lb.check_key(key), "Now: {:?}", clock.now());

        for _ in 0..10 {
            clock.advance(ms);
            assert_ne!(Ok(()), lb.peek_key(key), "Now: {:?}", clock.now());
        }
        clock.advance(ms);
        assert_ne!(Ok(()), lb.check_key(key), "Now: {:?}", clock.now());
    }
}

fn retained_keys<T: Clone + Hash + Eq + Copy + Ord>(
    keys: &[T],
    limiter: RateLimiter<
        T,
        DashMapStateStore<T>,
        FakeRelativeClock,
        NoOpMiddleware<<FakeRelativeClock as Clock>::Instant>,
    >,
) -> Vec<T> {
    let state = limiter.into_state_store();
    let mut keys: Vec<T> = keys
        .iter()
        .copied()
        .filter(|&k| state.contains_key(&k))
        .collect();
    keys.sort();
    keys
}

#[test]
fn expiration() {
    let clock = FakeRelativeClock::default();
    let ms = Duration::from_millis(1);

    let make_bucket = || {
        let lim = RateLimiter::dashmap_with_clock(Quota::per_second(nonzero!(1u32)), &clock);
        lim.check_key(&"foo").unwrap();
        clock.advance(ms * 200);
        lim.check_key(&"bar").unwrap();
        clock.advance(ms * 600);
        lim.check_key(&"baz").unwrap();
        lim
    };
    let keys = &["bar", "baz", "foo"];

    // clean up all keys that are indistinguishable from unoccupied keys:
    let lim_shrunk = make_bucket();
    lim_shrunk.retain_recent();
    assert_eq!(retained_keys(keys, lim_shrunk), keys);

    let lim_later = make_bucket();
    clock.advance(ms * 1200);
    lim_later.retain_recent();
    assert_eq!(retained_keys(keys, lim_later), vec!["bar", "baz"]);

    let lim_later = make_bucket();
    clock.advance(ms * (1200 + 200));
    lim_later.retain_recent();
    assert_eq!(retained_keys(keys, lim_later), vec!["baz"]);

    let lim_later = make_bucket();
    clock.advance(ms * (1200 + 200 + 600));
    lim_later.retain_recent();
    assert_eq!(retained_keys(keys, lim_later), Vec::<&str>::new());
}

#[test]
fn actual_threadsafety() {
    use crossbeam;

    let clock = FakeRelativeClock::default();
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

#[test]
fn dashmap_length() {
    let lim = RateLimiter::dashmap(Quota::per_second(nonzero!(1u32)));
    assert_eq!(lim.len(), 0);
    assert!(lim.is_empty());

    lim.check_key(&"foo").unwrap();
    assert_eq!(lim.len(), 1);
    assert!(!lim.is_empty(),);

    lim.check_key(&"bar").unwrap();
    assert_eq!(lim.len(), 2);
    assert!(!lim.is_empty());

    lim.check_key(&"baz").unwrap();
    assert_eq!(lim.len(), 3);
    assert!(!lim.is_empty());
}

#[test]
fn dashmap_shrink_to_fit() {
    let clock = FakeRelativeClock::default();
    // a steady rate of 3ms between elements:
    let lim = RateLimiter::dashmap_with_clock(Quota::per_second(nonzero!(20u32)), &clock);
    let ms = Duration::from_millis(1);

    assert_eq!(
        lim.check_key_n(&"long-lived".to_string(), nonzero!(10_u32)),
        Ok(Ok(()))
    );
    assert_eq!(lim.check_key(&"short-lived".to_string()), Ok(()));

    // Move the clock forward far enough that the short-lived key gets dropped:
    clock.advance(ms * 300);
    lim.retain_recent();
    lim.shrink_to_fit();

    assert_eq!(lim.len(), 1);
}
