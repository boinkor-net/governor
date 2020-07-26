# Changes for [`governor`](https://crates.io/crates/governor)

<!-- next-header -->

## [Unreleased] - ReleaseDate

## [[0.3.1](https://docs.rs/governor/0.3.1/governor/)] - 2020-07-26

### Added

  A little section to the
  [guide](https://docs.rs/governor/0.3.1/governor/_guide/index.html)
  explaining how to use keyed rate limiters.

## [[0.3.0](https://docs.rs/governor/0.3.0/governor/)] - 2020-07-25

### Added

* The `ShrinkableKeyedStateStore` trait now has required `len` and
  `is_empty` methods, which are also made available on any
  `RateLimiter` that uses a shrinkable (Hashmap / Dashmap backed)
  state store. Thanks to [@lytefast](https://github.com/lytefast) for
  the idea and [pull request](https://github.com/antifuchs/ratelimit_meter/pull/38)
  on `ratelimit_meter`!

### Changed

* The `MonotonicClock` and `SystemClock` struct definitions now are
  proper "empty" structs. Any non-`Default` construction of these clocks
  must now use `MonotonicClock` instead of `MonotonicClock()`.
* The `clock::ReasonablyRealtime` trait got simplified and no longer
  has any required methods to implement, only one default method.
* Replaced the `spin` crate with `parking_lot` for `no_std` contexts.

### Contributors

* [@Restioson](https://github.com/Restioson)
* [@korrat](https://github.com/korrat)
* [@lytefast](https://github.com/lytefast)

## [[0.2.0](https://docs.rs/governor/0.2.0/governor/)] - 2020-03-01

### Added

* This changelog!

* New type `RateLimiter`, superseding the `DirectRateLimiter` type.

* Support for keyed rate limiting in `RateLimiter`, which allows users
  to keep a distict rate limit state based on the value of a hashable
  element.

* Support for different state stores:
  * The direct in-memory state store
  * A keyed state store based on [dashmap](https://crates.io/crates/dashmap)
  * A keyed state store based on a mutex-locked [HashMap](https://doc.rust-lang.org/nightly/std/collections/struct.HashMap.html).

* Support for different clock kinds:
  * [Quanta](https://crates.io/crates/quanta) (the default), a high-performance clock
  * [Instant](https://doc.rust-lang.org/nightly/std/time/struct.Instant.html), the stdlib monotonic clock
  * A fake releative clock, useful for tests or in non-std environments.

* `Quota` constructors now support a separate `.allow_burst` method
  that specifies a maximum burst capacity that diverges from the
  default.

* New constructor `Quota::with_period` allows specifying the exact
  amount of time it takes to replenish a single element.

### Deprecated

* The `Quota::new` constructor has some very confusing modalities, and
  should not be used as-is.

### Fixed

* An off-by-one error in `check_n`, causing calls with `n =
  burst_size + 1` to return a "not yet" result instead of a "this will
  never work" result.

### Contributors

* [@jean-airoldie](https://github.com/jean-airoldie)
* [@antifuchs](https://github.com/antifuchs)

## [0.1.2](https://docs.rs/governor/0.1.2/governor/) - 2019-11-17

Initial release: A "direct" (a single rate-limiting state per
structure) rate limiter that works in an async context, using atomic
operations.
