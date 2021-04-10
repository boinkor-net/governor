use governor::{
    clock,
    middleware::{RateLimitingMiddleware, StateSnapshot},
    Quota, RateLimiter,
};
use nonzero_ext::nonzero;

#[derive(Debug, PartialEq)]
struct MyMW {}

impl RateLimitingMiddleware for MyMW {
    type PositiveOutcome = u16;

    fn allow<K, P>(_key: &K, _state: StateSnapshot<P>) -> Self::PositiveOutcome
    where
        P: clock::Reference,
    {
        666
    }

    fn disallow<K, P: governor::clock::Reference>(
        _key: &K,
        _state: StateSnapshot<P>,
        _not_until: &governor::NotUntil<P, Self>,
    ) where
        Self: Sized,
    {
    }
}

#[test]
fn changes_allowed_type() {
    let lim = RateLimiter::direct(Quota::per_second(nonzero!(4u32))).with_middleware::<MyMW>();
    assert_eq!(Ok(666), lim.check());
}
