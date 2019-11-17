#![cfg(feature = "std")]

// This test uses procinfo, so can only be run on Linux.
extern crate libc;

use governor::{Quota, RateLimiter};
use nonzero_ext::*;
use std::sync::Arc;
use std::thread;

fn resident_memory_size() -> i64 {
    let mut out: libc::rusage = unsafe { std::mem::zeroed() };
    assert!(unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut out) } == 0);
    out.ru_maxrss
}

const LEAK_TOLERANCE: i64 = 1024 * 1024 * 10;

struct LeakCheck {
    usage_before: i64,
    n_iter: usize,
}

impl Drop for LeakCheck {
    fn drop(&mut self) {
        let usage_after = resident_memory_size();
        assert!(
            usage_after <= self.usage_before + LEAK_TOLERANCE,
            "Plausible memory leak!\nAfter {} iterations, usage before: {}, usage after: {}",
            self.n_iter,
            self.usage_before,
            usage_after
        );
    }
}

impl LeakCheck {
    fn new(n_iter: usize) -> Self {
        LeakCheck {
            n_iter,
            usage_before: resident_memory_size(),
        }
    }
}

#[test]
fn memleak_gcra() {
    let bucket = RateLimiter::direct(Quota::per_second(nonzero!(1_000_000u32)));

    let leak_check = LeakCheck::new(500_000);

    for _i in 0..leak_check.n_iter {
        drop(bucket.check());
    }
}

#[test]
fn memleak_gcra_multi() {
    let bucket = RateLimiter::direct(Quota::per_second(nonzero!(1_000_000u32)));
    let leak_check = LeakCheck::new(500_000);

    for _i in 0..leak_check.n_iter {
        drop(bucket.check_all(nonzero!(2u32)));
    }
}

#[test]
fn memleak_gcra_threaded() {
    let bucket = Arc::new(RateLimiter::direct(Quota::per_second(nonzero!(
        1_000_000u32
    ))));
    let leak_check = LeakCheck::new(5_000);

    for _i in 0..leak_check.n_iter {
        let bucket = bucket.clone();
        thread::spawn(move || {
            assert_eq!(Ok(()), bucket.check());
        })
        .join()
        .unwrap();
    }
}
