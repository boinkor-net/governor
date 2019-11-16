#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

/// A facade around all the types we need from std/core crates, to
/// avoid unnecessary cfg-conditionalization everywhere.
mod lib {
    mod core {
        #[cfg(not(feature = "std"))]
        pub use core::*;

        #[cfg(feature = "std")]
        pub use std::*;
    }

    pub use self::core::clone::Clone;
    pub use self::core::cmp::{Eq, Ord, PartialEq};
    pub use self::core::convert::TryFrom;
    pub use self::core::convert::TryInto;
    pub use self::core::default::Default;
    pub use self::core::fmt::Debug;
    pub use self::core::marker::{Copy, PhantomData, Send, Sized, Sync};
    pub use self::core::num::{NonZeroU128, NonZeroU32};
    pub use self::core::ops::{Add, Div, Mul, Sub};
    pub use self::core::sync::atomic::{AtomicU64, Ordering};
    pub use self::core::time::Duration;

    pub use self::core::cmp;
    pub use self::core::fmt;

    /// Imports that are only available on std.
    #[cfg(feature = "std")]
    mod std {
        pub use std::collections::hash_map::RandomState;
        pub use std::hash::{BuildHasher, Hash};
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

pub mod clock;
mod errors;
mod gcra;
mod jitter;
mod nanos;
mod quota;
mod state;

pub use errors::*;
pub use gcra::NotUntil;
pub use jitter::Jitter;
pub use quota::Quota;
pub use state::direct::DirectRateLimiter;

#[cfg(feature = "std")]
pub use state::direct::SinkExt;
