use governor::{clock::FakeRelativeClock, Quota, RateLimiter};
use nonzero_ext::nonzero;
use std::hash::{BuildHasher, Hasher};
use std::time::Duration;

#[derive(Clone, Default, Debug)]
struct BadHasher;

impl Hasher for BadHasher {
    fn finish(&self) -> u64 {
        4 // chosen by fair dice roll
    }

    fn write(&mut self, _: &[u8]) {}
}

#[derive(Clone, Default, Debug)]
struct BadHasherBuilder;

impl BuildHasher for BadHasherBuilder {
    type Hasher = BadHasher;

    fn build_hasher(&self) -> Self::Hasher {
        BadHasher
    }
}

#[derive(Clone, Default, Debug)]
struct CustomHasher {
    value: u64,
}

impl Hasher for CustomHasher {
    fn finish(&self) -> u64 {
        self.value.wrapping_mul(0x517cc1b727220a95)
    }

    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.value = self.value.wrapping_add(b as u64);
        }
    }
}

#[derive(Clone, Default, Debug)]
struct CustomHasherBuilder;

impl BuildHasher for CustomHasherBuilder {
    type Hasher = CustomHasher;

    fn build_hasher(&self) -> Self::Hasher {
        CustomHasher::default()
    }
}

macro_rules! test_rate_limiter {
    ($map_method:ident, $hasher_type:ty, $test_name:ident) => {
        #[test]
        fn $test_name() {
            let clock = FakeRelativeClock::default();
            let hasher = <$hasher_type>::default();

            let lb =
                RateLimiter::$map_method(Quota::per_second(nonzero!(20u32)), clock.clone(), hasher);

            let key1 = 1u32;
            let key2 = 2u32;

            lb.check_key_n(&key1, nonzero!(20u32)).unwrap().unwrap();
            assert_ne!(lb.check_key(&key1), Ok(()));

            lb.check_key_n(&key2, nonzero!(20u32)).unwrap().unwrap();
            assert_ne!(lb.check_key(&key2), Ok(()));

            // Test reset after 1 second
            clock.advance(Duration::from_secs(1));
            assert_eq!(lb.check_key(&key1), Ok(()));
            assert_eq!(lb.check_key(&key2), Ok(()));

            // Verify remaining capacity tracking
            lb.check_key_n(&key1, nonzero!(19u32)).unwrap().unwrap();
            lb.check_key_n(&key2, nonzero!(19u32)).unwrap().unwrap();
        }
    };
}

// Test for simple constructors (without clock)
macro_rules! test_rate_limiter_simple {
    ($map_method:ident, $hasher_type:ty, $test_name:ident) => {
        #[test]
        fn $test_name() {
            let hasher = <$hasher_type>::default();

            let lb = RateLimiter::$map_method(Quota::per_second(nonzero!(20u32)), hasher);

            let key1 = 1u32;
            let key2 = 2u32;

            lb.check_key_n(&key1, nonzero!(20u32)).unwrap().unwrap();
            assert_ne!(lb.check_key(&key1), Ok(()));

            lb.check_key_n(&key2, nonzero!(20u32)).unwrap().unwrap();
            assert_ne!(lb.check_key(&key2), Ok(()));
        }
    };
}

macro_rules! test_hashmap_rate_limiter {
    ($hasher_type:ty, $test_name:ident) => {
        test_rate_limiter!(hashmap_with_clock_and_hasher, $hasher_type, $test_name);
    };
}

macro_rules! test_hashmap_simple_rate_limiter {
    ($hasher_type:ty, $test_name:ident) => {
        test_rate_limiter_simple!(hashmap_with_hasher, $hasher_type, $test_name);
    };
}

#[cfg(all(feature = "std", feature = "dashmap"))]
macro_rules! test_dashmap_rate_limiter {
    ($hasher_type:ty, $test_name:ident) => {
        test_rate_limiter!(dashmap_with_clock_and_hasher, $hasher_type, $test_name);
    };
}

#[cfg(all(feature = "std", feature = "dashmap"))]
macro_rules! test_dashmap_simple_rate_limiter {
    ($hasher_type:ty, $test_name:ident) => {
        test_rate_limiter_simple!(dashmap_with_hasher, $hasher_type, $test_name);
    };
}

// Generate the actual tests
test_hashmap_rate_limiter!(BadHasherBuilder, test_hashmap_with_identity_hasher);
test_hashmap_rate_limiter!(CustomHasherBuilder, test_hashmap_with_custom_hasher);
test_hashmap_simple_rate_limiter!(BadHasherBuilder, test_hashmap_simple_with_identity_hasher);
test_hashmap_simple_rate_limiter!(CustomHasherBuilder, test_hashmap_simple_with_custom_hasher);

#[cfg(all(feature = "std", feature = "dashmap"))]
test_dashmap_rate_limiter!(BadHasherBuilder, test_dashmap_with_identity_hasher);

#[cfg(all(feature = "std", feature = "dashmap"))]
test_dashmap_rate_limiter!(CustomHasherBuilder, test_dashmap_with_custom_hasher);

#[cfg(all(feature = "std", feature = "dashmap"))]
test_dashmap_simple_rate_limiter!(BadHasherBuilder, test_dashmap_simple_with_identity_hasher);

#[cfg(all(feature = "std", feature = "dashmap"))]
test_dashmap_simple_rate_limiter!(CustomHasherBuilder, test_dashmap_simple_with_custom_hasher);
