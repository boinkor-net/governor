//! # A more in-depth guide to `governor`
//!
//! Governor is a fork/rewrite/rebranding of the
//! [`ratelimit_meter`](https://crates.io/crates/ratelimit_meter) and
//! [`ratelimit_futures`](https://crates.io/crates/ratelimit_futures)
//! crates. Many of the things that worked there still work here, and
//! this guide's aim is to help you accomplish them.
//!
//! # Constructing a rate limiter
//!
//! Currently, only "direct" rate-limiters are supported - that is,
//! rate limiters that keep only a single state (most rate limiters in
//! the wild expose a "keyed" interface, keeping one state per key).
//!
//! Construction of rate limiters is designed to be mostly infallible,
//! given correctly-constructed parameters. To that end, `governor`
//! makes heavy use of the [`NonZeroU32`][std::num::NonZeroU32] type.
//!
//! To conveniently construct these nonzero numbers, use the
//! [`nonzero!`](../../nonzero_ext/macro.nonzero.html) macro.
//!
//! #### Quotas
//!
//! Each rate limiter has a quota: A rate of elements (could be API
//! requests, emails, phone calls... anything really) per unit of time (second,
//! minute, hour). Specify these in a [`Quota`][crate::Quota] object like so:
//!
//! ```rust
//! # use nonzero_ext::*;
//! use governor::Quota;
//! Quota::per_second(nonzero!(20u32));
//! ```
//!
//! #### Constructing a direct rate limiter
//!
//! To make a direct rate limiter, you have to construct a quota, as
//! above; and then use this to construct the rate limiter itself. In
//! `std` mode, this is easily accomplished like so:
//!
//! ```rust
//! # #[cfg(feature = "std")] fn main() {
//! # use nonzero_ext::*;
//! # use governor::{DirectRateLimiter, Quota};
//! DirectRateLimiter::new(Quota::per_second(nonzero!(50u32)));
//! # } #[cfg(not(feature = "std"))] fn main() {}
//! ```
//!
//! In `no_std` mode, there are is no default monotonic (or system)
//! clock available. To effectively limit rates, you will have to
//! either use the provided "fake" clock (which must be manually
//! advanced, and is mainly useful for tests), or implement the
//! `Clock` trait for your platform. Once that decision is made,
//! constructing a rate limiter with an explicit clock works like
//! this:
//!
//! ```rust
//! # use nonzero_ext::*;
//! # use governor::{clock::FakeRelativeClock, DirectRateLimiter, Quota};
//! let clock = FakeRelativeClock::default();
//! DirectRateLimiter::new_with_clock(Quota::per_second(nonzero!(50u32)), &clock);
//! ```
//!
//! # Data ownership and references to rate limiters
//!
//! `governor`'s rate limiter state is not hidden behind an [interior
//! mutability](https://doc.rust-lang.org/book/ch15-05-interior-mutability.html)
//! pattern, and so it is perfectly valid to have multiple references
//! to a rate limiter in a program. Since its state lives in
//! [`AtomicU64`][std::sync::atomic::AtomicU64] integers (which do not
//! implement [`Clone`]), the rate limiters themselves can not be
//! cloned.
//!
//! # Usage in multiple threads
//!
//! Sharing references to a rate limiter across threads is completely
//! OK (rate limiters are Send and Sync by default), but there is a
//! problem: A rate limiter's lifetime might be up before a thread
//! ends, which would invalidate the reference.
//!
//! So, to use a rate limiter in multiple threads without lifetime
//! issues, there are two equally valid strategies:
//!
//! #### `crossbeam` scoped tasks
//!
//! The `crossbeam` crate's
//! [scopes](https://docs.rs/crossbeam/0.7.3/crossbeam/thread/struct.Scope.html#method.spawn)
//! allow code to guarantee that a thread spawned in a scope
//! terminates before the scope terminates. This allows using
//! stack-allocated variables. Here is an example test using crossbeam
//! scopes:
//!
//! ```rust
//! # use crossbeam;
//! # use nonzero_ext::*;
//! # use governor::{clock::FakeRelativeClock, DirectRateLimiter, Quota};
//! # use std::time::Duration;
//!
//! let mut clock = FakeRelativeClock::default();
//! let lim = DirectRateLimiter::new_with_clock(Quota::per_second(nonzero!(20u32)), &clock);
//! let ms = Duration::from_millis(1);
//!
//! crossbeam::scope(|scope| {
//!     for _i in 0..20 {
//!         scope.spawn(|_| {
//!             assert_eq!(Ok(()), lim.check());
//!         });
//!     }
//! })
//! .unwrap();
//! ```
//!
//! #### Wrapping the limiter in an [`Arc`][std::sync::Arc]
//!
//! The other method uses only the standard library: Wrapping the rate
//! limiter in an [`Arc`][std::sync::Arc] will keep the limiter alive
//! for as long as there exist references to it - perfect for passing
//! to threads.
//!
//! In this example, note that we're cloning what looks like the
//! limiter, once per thread -- but that is only what it looks
//! like. The thing that actually gets cloned is the `Arc`; the rate
//! limiter stays identical, only its references are duplicated (and
//! refcounts incremented).
//!
//! ```rust
//! # #[cfg(feature = "std")] fn main() {
//! # use nonzero_ext::*;
//! # use governor::{DirectRateLimiter, Quota};
//! # use std::time::Duration;
//! # use std::sync::Arc;
//! # use std::thread;
//! let bucket = Arc::new(DirectRateLimiter::new(Quota::per_second(nonzero!(20u32))));
//! for _i in 0..20 {
//!     let bucket = bucket.clone();
//!     thread::spawn(move || {
//!         assert_eq!(Ok(()), bucket.check());
//!     })
//!     .join()
//!     .unwrap();
//! }
//! # } #[cfg(not(feature = "std"))] fn main() {}
//! ```
//!
