# Changes for [`governor`](https://crates.io/crates/governor)

<!-- next-header -->

## [Unreleased] - ReleaseDate

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
