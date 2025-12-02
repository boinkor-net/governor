use governor::{clock::FakeRelativeClock, Quota, RateLimiter};
use nonzero_ext::nonzero;
use std::time::Duration;

#[test]
fn check_any_n_admits_available() {
    let clock = FakeRelativeClock::default();
    let lb = RateLimiter::direct_with_clock(Quota::per_second(nonzero!(10u32)), clock.clone());

    // Burst capacity is 10, so we can get all 10
    let (actual, _) = lb.check_any_n(nonzero!(10u32));
    assert_eq!(actual, 10, "Should get all 10 tokens from burst capacity");

    // No capacity left, should get 0
    let (actual, _) = lb.check_any_n(nonzero!(5u32));
    assert_eq!(actual, 0, "Should get 0 tokens when depleted");

    // Wait 500ms (allows 5 more tokens at 10/sec)
    clock.advance(Duration::from_millis(500));
    let (actual, _) = lb.check_any_n(nonzero!(10u32));
    assert_eq!(actual, 5, "Should get 5 tokens after 500ms");
}
#[test]
fn check_any_n_partial_vending() {
    let clock = FakeRelativeClock::default();
    let lb = RateLimiter::direct_with_clock(Quota::per_second(nonzero!(10u32)), clock.clone());

    // Take 7 tokens
    let (actual, _) = lb.check_any_n(nonzero!(7u32));
    assert_eq!(actual, 7, "Should get 7 tokens");

    // Ask for 10, but only 3 available
    let (actual, _) = lb.check_any_n(nonzero!(10u32));
    assert_eq!(actual, 3, "Should get 3 tokens (partial)");

    // No more available
    let (actual, _) = lb.check_any_n(nonzero!(10u32));
    assert_eq!(actual, 0, "Should get 0 tokens when depleted");
}

#[test]
fn check_any_n_never_exceeds_burst() {
    let clock = FakeRelativeClock::default();
    let lb = RateLimiter::direct_with_clock(Quota::per_second(nonzero!(5u32)), clock.clone());

    // Even if we ask for more than burst capacity, we only get the burst
    let (actual, _) = lb.check_any_n(nonzero!(100u32));
    assert_eq!(actual, 5, "Should not exceed burst capacity");

    // Verify we can't get any more
    let (actual, _) = lb.check_any_n(nonzero!(1u32));
    assert_eq!(actual, 0, "Should be depleted after taking burst");
}

#[test]
fn check_any_n_single_token_request() {
    let clock = FakeRelativeClock::default();
    let lb = RateLimiter::direct_with_clock(Quota::per_second(nonzero!(10u32)), clock);

    // Request just 1 token
    let (actual, _) = lb.check_any_n(nonzero!(1u32));
    assert_eq!(actual, 1, "Should get 1 token");
}

#[test]
fn check_any_n_gradual_refill() {
    let clock = FakeRelativeClock::default();
    let lb = RateLimiter::direct_with_clock(Quota::per_second(nonzero!(10u32)), clock.clone());

    // Deplete the bucket
    let (actual, _) = lb.check_any_n(nonzero!(10u32));
    assert_eq!(actual, 10);

    // Wait 100ms at a time and check how many tokens we get
    for _ in 0..5 {
        clock.advance(Duration::from_millis(100));
        let (actual, _) = lb.check_any_n(nonzero!(10u32));
        // At 10 tokens/sec, we should get 1 token per 100ms
        assert_eq!(actual, 1, "Should get 1 token every 100ms");
    }
}

#[test]
fn check_any_n_keyed_partial() {
    let clock = FakeRelativeClock::default();
    let lb = RateLimiter::dashmap_with_clock(Quota::per_second(nonzero!(10u32)), clock.clone());

    // Take 7 tokens for user1
    let (actual, _) = lb.check_key_any_n(&"user1", nonzero!(7u32));
    assert_eq!(actual, 7);

    // Ask for 10, should get 3
    let (actual, _) = lb.check_key_any_n(&"user1", nonzero!(10u32));
    assert_eq!(actual, 3);

    // user2 should still have full capacity
    let (actual, _) = lb.check_key_any_n(&"user2", nonzero!(10u32));
    assert_eq!(actual, 10, "User2 should have full capacity");
}
