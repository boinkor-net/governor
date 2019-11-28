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
//! In this example, we set up a rate limiter to allow 5 elements per
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

#![cfg_attr(not(feature = "std"), no_std)]

extern crate no_std_compat as std;

/// A facade around all the types we need from std/core crates, to
/// avoid unnecessary cfg-conditionalization everywhere.
mod lib {
    mod core {
        pub use std::*;
    }

    pub use self::core::borrow::Borrow;
    pub use self::core::clone::Clone;
    pub use self::core::cmp::{Eq, Ord, PartialEq};
    pub use self::core::convert::TryFrom;
    pub use self::core::convert::TryInto;
    pub use self::core::default::Default;
    pub use self::core::fmt::Debug;
    pub use self::core::hash::Hash;
    pub use self::core::marker::{Copy, PhantomData, Send, Sized, Sync};
    pub use self::core::num::{NonZeroU128, NonZeroU32, NonZeroU64};
    pub use self::core::ops::{Add, Div, Mul, Sub};
    pub use self::core::sync::atomic::{AtomicU64, Ordering};
    pub use self::core::time::Duration;

    pub use self::core::cmp;
    pub use self::core::fmt;

    /// Imports that are only available on std.
    #[cfg(feature = "std")]
    mod std {
        pub use std::collections::{hash_map::RandomState, HashMap};
        pub use std::hash::BuildHasher;
        pub use std::sync::Arc;
        pub use std::time::Instant;
    }

    #[cfg(not(feature = "std"))]
    mod no_std {
        pub use alloc::sync::Arc;
    }

    #[cfg(feature = "std")]
    pub use self::std::*;

    #[cfg(not(feature = "std"))]
    pub use self::no_std::*;
}

pub mod r#_guide;
pub mod clock;
mod errors;
mod gcra;
mod jitter;
mod nanos;
mod quota;
pub mod state;

pub use errors::*;
pub use gcra::NotUntil;
pub use jitter::Jitter;
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
