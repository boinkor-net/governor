//! # governor - a rate-limiting library for rust.
//!
//! Governor aims to be a very efficient and ergonomic way to enforce
//! rate limits in Rust programs. It implements the [Generic Cell Rate
//! Algorithm](https://en.wikipedia.org/wiki/Generic_cell_rate_algorithm)
//! and keeps state in a very efficient way.
//!
//! For detailed information on usage, please see the [user's guide][crate::_guide].
//!
//! # Quick example
//!
//! In this example, we set up a rate limiter to allow 50 elements per
//! second, and check that a single element can pass through.
//!
//! ``` rust
//! use std::num::NonZeroU32;
//! use nonzero_ext::*;
//! use governor::{Quota, RateLimiter};
//!
//! # #[cfg(feature = "std")]
//! # fn main () {
//! let mut lim = RateLimiter::direct(Quota::per_second(nonzero!(50u32))); // Allow 50 units per second
//! assert_eq!(Ok(()), lim.check());
//! # }
//! # #[cfg(not(feature = "std"))]
//! # fn main() {}
//! ```
//!

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]
// Clippy config: Deny warnings but allow unknown lint configuration (so I can use nightly)
#![deny(warnings)]
#![allow(unknown_lints)]
// Unfortunately necessary, otherwise features aren't supported in doctests:
#![allow(clippy::needless_doctest_main)]

extern crate alloc;

pub mod r#_guide;
pub mod clock;
mod errors;
mod gcra;
mod jitter;
pub mod middleware;
pub mod nanos;
mod quota;
pub mod state;

pub use errors::*;
pub use gcra::NotUntil;
#[cfg(all(feature = "std", feature = "jitter"))]
pub use jitter::Jitter;
#[cfg(all(feature = "std", not(feature = "jitter")))]
pub(crate) use jitter::Jitter;
pub use quota::Quota;
#[doc(inline)]
pub use state::RateLimiter;

#[cfg(feature = "std")]
pub use state::direct::RatelimitedSink;
#[cfg(feature = "std")]
pub use state::direct::RatelimitedStream;

/// The collection of asynchronous traits exported from this crate.
pub mod prelude {
    #[cfg(feature = "std")]
    pub use crate::state::direct::SinkRateLimitExt;
    #[cfg(feature = "std")]
    pub use crate::state::direct::StreamRateLimitExt;
}

/// A rate limiter representing a single item of state in memory, running on the default clock.
///
/// See the [`RateLimiter`] documentation for details.
pub type DefaultDirectRateLimiter<
    MW = middleware::NoOpMiddleware<<clock::DefaultClock as clock::Clock>::Instant>,
> = RateLimiter<state::direct::NotKeyed, state::InMemoryState, clock::DefaultClock, MW>;

/// A rate limiter with one state per key, running on the default clock.
///
/// See the [`RateLimiter`] documentation for details.
pub type DefaultKeyedRateLimiter<
    K,
    MW = middleware::NoOpMiddleware<<clock::DefaultClock as clock::Clock>::Instant>,
    S = state::keyed::DefaultHasher,
> = RateLimiter<K, state::keyed::DefaultKeyedStateStore<K, S>, clock::DefaultClock, MW>;
