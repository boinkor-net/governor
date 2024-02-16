# Changes for [`governor`](https://crates.io/crates/governor)

<!-- next-header -->

## [Unreleased] - ReleaseDate

## [[0.6.3](https://docs.rs/governor/0.6.3/governor/)] - 2024-02-16

### Changed

* Just another bug-fixed release process. This time, it should
  actually release out of github actions.

## [[0.6.2](https://docs.rs/governor/0.6.2/governor/)] - 2024-02-16

### Changed

* New release process: governor can now be automatically released
  using github actions.

## [[0.6.1](https://docs.rs/governor/0.6.1/governor/)] - 2024-02-16

### Changed

* The governor repo now lives in the `boinkor-net` github
  organization. No ownership has changed (@antifuchs still manages
  this org), but this makes it easier to securely manage the CI and
  release setup.

* The `.per_second` constructor for `Quota` now constructs a quota
  that ensures all rate-limiting calls succeed when given values in
  excess of 1 billion (previously, this would result in rate limiters
  that would incorrectly reject values). Reported in
  [#203](https://github.com/antifuchs/governor/issues/203).

* `QuantaUpkeepInstant` now [properly advances
  forward](https://github.com/boinkor-net/governor/pull/223).

* `no_std` is now [properly
  supported](https://github.com/boinkor-net/governor/pull/222):
  Instead of parking-lot, governor now uses the spinning_top crate in
  `no_std` mode.

### Contributors
* [@rkd-msw](https://github.com/rkd-msw)
* [@Serene-Arc](https://github.com/Serene-Arc)
* [@waynerobinson](https://github.com/waynerobinson)
* [@mammothbane](https://github.com/mammothbane)

## [[0.6.0](https://docs.rs/governor/0.6.0/governor/)] - 2023-07-12

### Added
* Type aliases `DefaultDirectRateLimiter` and
  `DefaultKeyedRateLimiter` to cut down on type-typing of typical rate
  limiters in struct and function definitions. Requested in
  [#85](https://github.com/antifuchs/governor/issues/85).

### Changed
* The API for `.check_n` and `.until_n` (and their keyed counterpart)
  have changed to return a nested `Result` - the outer indicating
  whether the check could ever succeed and the inner one indicating
  the rate limiting result, if it could succeed.
* Updated the [quanta dependency to
  0.11.1](https://github.com/metrics-rs/quanta/blob/main/CHANGELOG.md#0110---2023-03-24).
  This change also adds a reference u64 instant to all instances of
  the `QuantaUpkeepClock` structure. All lower-resolution quanta
  timekeeping used by governor will now be relative to that reference
  instant.
* Some parts of the documentation for burst sizes has been rephrased
  to be less confusing.

### Contributors
* [@brprkr](https://github.com/brprkr)

## [[0.5.1](https://docs.rs/governor/0.5.1/governor/)] - 2022-11-29

### Changed
* Fixed quota reporting for positive rate limiting decisions when
  `StateInformationMiddleware` is in use with a real clock. Reported
  in [#157](https://github.com/antifuchs/governor/issues/157)

### Contributors
* [@AaronErhardt](https://github.com/AaronErhardt)

## [[0.5.0](https://docs.rs/governor/0.5.0/governor/)] - 2022-09-19

### Changed
* Fixed a bug in `StateSnapshot::remaining_burst_capacity`: Now
  returns the correct number of cells after enough time has
  passed. Thanks to [@holmesmr](https://github.com/holmesmr) for
  reporting [#102](https://github.com/antifuchs/governor/issues/102)!

* Dropped a `From<(&_ Gcra, Nanos)>` impl for `StateSnapshot`.

## [[0.4.2](https://docs.rs/governor/0.4.2/governor/)] - 2022-02-09

### Changed
* Upgraded `dashmap` back [to 5.1.0](https://github.com/antifuchs/governor/pull/110).
* Upgraded `parking_lot` [to 0.12.0](https://github.com/antifuchs/governor/pull/109).

### Internal
* Migrated the `governor` code base to [cargo
  workspaces](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html).

## [[0.4.1](https://docs.rs/governor/0.4.1/governor/)] - 2022-01-21

### Changed
* Downgraded `dashmap` to 4.0.2 as a remediation for
  [RUSTSEC-2022-0002](https://rustsec.org/advisories/RUSTSEC-2022-0002)
  via [#104](https://github.com/antifuchs/governor/pull/104).

### Contributors
* [@kim](https://github.com/kim)

## [[0.4.0](https://docs.rs/governor/0.4.0/governor/)] - 2021-12-28

### Added
* You can now alter&expand the information returned from a rate
  limiter by attaching middleware to it using
  `.with_middleware::<YourClass>()` at construction time.

  This is an incompatible change, as the type signature of RateLimiter
  gained an additional generic parameter. See the [pull
  request](https://github.com/antifuchs/governor/pull/67) and
  [issue #66](https://github.com/antifuchs/governor/issues/66) for
  details.

### Changed

* Updated the [`Arc` guide section](https://docs.rs/governor/0.3.3/governor/_guide/index.html#wrapping-the-limiter-in-an-arc) to use `Arc::clone()` instead of `limiter.clone()`.
* Updated the [`quanta` dependency](https://crates.io/crates/quanta)
  to 0.8.0, speeding up the quanta clock by a bit. This changes the
  upkeep clock interface incompatibly: The quanta upkeep Builder
  structure got renamed to `quanta::Upkeep`.
* The `nanos` module is now public, allowing other crates to implement
  the `Clock` trait.
* When using the `std` feature, governor will no longer pull in the
  `hashbrown` crate.

### Contributors
* [@bradfier](https://github.com/bradfier)
* [@izik1](https://github.com/izik1)
* [@ldm0](https://github.com/ldm0)

## [[0.3.2](https://docs.rs/governor/0.3.2/governor/)] - 2021-01-28

## [[0.3.1](https://docs.rs/governor/0.3.1/governor/)] - 2020-07-26

### Added

* A little section to the
  [guide](https://docs.rs/governor/0.3.1/governor/_guide/index.html)
  explaining how to use keyed rate limiters.

### Changed

  Several dependencies' minimum versions were bumped, including a
  version bump of
  [`smallvec`](https://github.com/servo/rust-smallvec), a transitive
  dependency which could previously result in trees using `governor`
  pulling in a vulnerable smallvec version.

### Contributors

* [@AaronErhardt](https://github.com/AaronErhardt)
* [@FintanH](https://github.com/FintanH)

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
