use std::prelude::v1::*;

use crate::nanos::Nanos;
use crate::state::{NotKeyed, StateStore};
use std::fmt;
use std::fmt::Debug;
use std::num::NonZeroU64;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Duration;

/// An in-memory representation of a GCRA's rate-limiting state.
///
/// Implemented using [`AtomicU64`] operations, this state representation can be used to
/// construct rate limiting states for other in-memory states: e.g., this crate uses
/// `InMemoryState` as the states it tracks in the keyed rate limiters it implements.
///
/// Internally, the number tracked here is the theoretical arrival time (a GCRA term) in number of
/// nanoseconds since the rate limiter was created.
#[derive(Default)]
pub struct InMemoryState(AtomicU64);

impl InMemoryState {
    pub(crate) fn measure_and_replace_one<T, F, E>(&self, mut f: F) -> Result<T, E>
    where
        F: FnMut(Option<Nanos>) -> Result<(T, Nanos), E>,
    {
        let mut prev = self.0.load(Ordering::Acquire);
        let mut decision = f(NonZeroU64::new(prev).map(|n| n.get().into()));
        while let Ok((result, new_data)) = decision {
            match self.0.compare_exchange_weak(
                prev,
                new_data.into(),
                Ordering::Release,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Ok(result),
                Err(next_prev) => prev = next_prev,
            }
            decision = f(NonZeroU64::new(prev).map(|n| n.get().into()));
        }
        // The loop above either ends in a successful compare-and-exchange operation,
        // or when the function `f` returns an error. In the case of an error,
        // result type is `Result<(T, Nanos), E>`, but we only want to return the `Result<T, E>`.
        // The `map` operation here is used to transform the result to the desired shape before returning.
        // While it might seem redundant, it provides clear semantic meaning. The loop structure guarantees
        // that we only reach this point in the error case. Hence, this map operation ensures that we
        // correctly handle the result type transformation without introducing possible infinite loops
        // or other unexpected behaviors.
        decision.map(|(result, _)| result)
    }

    pub(crate) fn measure_and_peek_one<T, F, E>(&self, mut f: F) -> Result<T, E>
    where
        F: FnMut(Option<Nanos>) -> Result<(T, Nanos), E>,
    {
        let mut prev = self.0.load(Ordering::Acquire);
        let original_prev = prev;
        let mut decision = f(NonZeroU64::new(prev).map(|n| n.get().into()));
        while let Ok((result, new_data)) = decision {
            match self.0.compare_exchange_weak(
                prev,
                new_data.into(),
                Ordering::Release,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    self.0.store(original_prev, Ordering::Release);
                    return Ok(result);
                }
                Err(next_prev) => prev = next_prev,
            }
            decision = f(NonZeroU64::new(prev).map(|n| n.get().into()));
        }
        // The loop above either ends in a successful compare-and-exchange operation,
        // or when the function `f` returns an error. In the case of an error,
        // result type is `Result<(T, Nanos), E>`, but we only want to return the `Result<T, E>`.
        // The `map` operation here is used to transform the result to the desired shape before returning.
        // While it might seem redundant, it provides clear semantic meaning. The loop structure guarantees
        // that we only reach this point in the error case. Hence, this map operation ensures that we
        // correctly handle the result type transformation without introducing possible infinite loops
        // or other unexpected behaviors.
        decision.map(|(result, _)| result)
    }

    pub(crate) fn is_older_than(&self, nanos: Nanos) -> bool {
        self.0.load(Ordering::Relaxed) <= nanos.into()
    }
}

/// The InMemoryState is the canonical "direct" state store.
impl StateStore for InMemoryState {
    type Key = NotKeyed;

    fn measure_and_replace<T, F, E>(&self, _key: &Self::Key, f: F) -> Result<T, E>
    where
        F: Fn(Option<Nanos>) -> Result<(T, Nanos), E>,
    {
        self.measure_and_replace_one(f)
    }

    fn measure_and_peek<T, F, E>(&self, _key: &Self::Key, f: F) -> Option<Result<T, E>>
    where
        F: Fn(Option<Nanos>) -> Result<(T, Nanos), E>,
    {
        Some(self.measure_and_peek_one(f))
    }
    fn reset(&self, _key: &Self::Key) {
        self.0.store(0, Ordering::Release);
    }
}

impl Debug for InMemoryState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let d = Duration::from_nanos(self.0.load(Ordering::Relaxed));
        write!(f, "InMemoryState({:?})", d)
    }
}

#[cfg(test)]
#[allow(clippy::needless_collect)]
mod test {

    use super::*;
    use std::sync::Arc;

    #[cfg(feature = "std")]
    fn try_triggering_collisions(
        n_threads: u64,
        tries_per_thread: u64,
        max_sleep_duration: Duration,
    ) -> (u64, u64) {
        use rand::Rng;
        use std::thread;

        let mut state = Arc::new(InMemoryState(AtomicU64::new(0)));
        let threads: Vec<thread::JoinHandle<_>> = (0..n_threads)
            .map(|_| {
                thread::spawn({
                    let state = Arc::clone(&state);
                    move || {
                        let mut hits = 0;
                        for _ in 0..tries_per_thread {
                            let sleep_duration = Duration::from_millis(
                                rand::thread_rng()
                                    .gen_range(0..max_sleep_duration.as_millis() as u64),
                            );
                            thread::sleep(sleep_duration);
                            assert!(state
                                .measure_and_replace_one(|old| {
                                    hits += 1;
                                    Ok::<((), Nanos), ()>((
                                        (),
                                        Nanos::from(old.map(Nanos::as_u64).unwrap_or(0) + 1),
                                    ))
                                })
                                .is_ok());
                        }
                        hits
                    }
                })
            })
            .collect();
        let hits: u64 = threads.into_iter().map(|t| t.join().unwrap()).sum();
        let value = Arc::get_mut(&mut state).unwrap().0.get_mut();
        (*value, hits)
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_measure_and_peek_one_does_not_touch_state() {
        use std::thread;
        for initial_value in &[0u64, 1, 42, 100, 999, 2000] {
            let state = Arc::new(InMemoryState(AtomicU64::new(*initial_value)));

            // Clone for use in another thread.
            let state_for_thread = Arc::clone(&state);

            // Spawn a thread to modify the atomic value after a short sleep.
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(10));
                let new_val = state_for_thread.0.load(Ordering::Acquire).wrapping_add(1);
                state_for_thread.0.store(new_val, Ordering::Release);
            });

            let _ = state.measure_and_peek_one(|current_val| {
                // Depending on the current value, decide what to return.
                match current_val {
                    Some(value) if value < Nanos::from(1000) => Ok(((), value + Nanos::from(1))),
                    None => Ok(((), Nanos::from(1))),
                    _ => Err("Some other error"),
                }
            });

            // Check that the state did not change.
            assert_eq!(state.0.load(Ordering::Acquire), *initial_value);
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_measure_and_peek_one_contention() {
        use std::{error::Error, thread};
        let state = Arc::new(InMemoryState(AtomicU64::new(0)));

        // Spawn a thread to call measure_and_peek_one
        let handle: thread::JoinHandle<Result<(), Box<dyn Error + Send>>> = thread::spawn({
            let state_clone = Arc::clone(&state);
            move || {
                state_clone.measure_and_peek_one(|old| {
                    // Simulate some work
                    thread::sleep(Duration::from_millis(10));
                    Ok::<(_, _), Box<dyn Error + Send>>((
                        (),
                        Nanos::from(old.map(Nanos::as_u64).unwrap_or(0) + 1),
                    ))
                })
            }
        });

        // Introduce a delay to ensure the other thread enters measure_and_peek_one first
        thread::sleep(Duration::from_millis(5));
        state.0.store(42, Ordering::Release); // Modify the state to force contention

        let _ = handle.join().unwrap();
    }

    #[cfg(feature = "std")]
    #[test]
    fn stresstest_collision() {
        const MAX_TRIES: u64 = 1000;

        // Get the number of available CPUs/cores.
        let threads = 2 * (num_cpus::get() as u64) + 1;
        let mut collisions_occurred = false;
        for tries in 0..MAX_TRIES {
            let attempt = try_triggering_collisions(threads, tries, Duration::from_millis(10));
            let value = attempt.0;
            let hits = attempt.1;
            if hits > value {
                collisions_occurred = true;
                break;
            }
        }
        assert!(
            collisions_occurred,
            "Expected to detect a collision, but did not."
        );
    }

    #[test]
    fn measure_and_peek_one_no_threads() {
        for initial_value in &[0u64, 1, 42, 100, 999, 2000] {
            let state = Arc::new(InMemoryState(AtomicU64::new(*initial_value)));

            let _ = state.measure_and_peek_one(|current_val| {
                // Depending on the current value, decide what to return.
                match current_val {
                    Some(value) if value < Nanos::from(1000) => Ok(((), value + Nanos::from(1))),
                    None => Ok(((), Nanos::from(1))),
                    _ => Err("Some other error"),
                }
            });

            // Check that the state did not change.
            assert_eq!(state.0.load(Ordering::Acquire), *initial_value);
        }
    }

    #[test]
    fn in_memory_state_impls() {
        let state = InMemoryState(AtomicU64::new(0));
        assert!(!format!("{:?}", state).is_empty());
    }

    #[test]
    fn test_reset_in_memory_state() {
        let state = InMemoryState(AtomicU64::new(42));
        assert_eq!(state.0.load(Ordering::Acquire), 42);
        state.reset(&crate::state::NotKeyed::NonKey);
        assert_eq!(state.0.load(Ordering::Acquire), 0);
    }

    // This test is not as precise as the next one but does cover the QuantaUpkeepClock struct,
    // which the second one doesn't.
    #[cfg(feature = "std")]
    #[test]
    fn test_measure_and_peek_one_with_race_condition() {
        let state = Arc::new(InMemoryState(AtomicU64::new(0)));

        // Spawn a thread to perform an operation that could cause a race condition
        let state_clone = Arc::clone(&state);
        let handle = std::thread::spawn(move || {
            state_clone
                .measure_and_peek_one(|old| {
                    // Simulate some work
                    std::thread::sleep(Duration::from_millis(50));
                    Ok::<(_, Nanos), &str>((
                        (),
                        Nanos::from(old.map(Nanos::as_u64).unwrap_or(0) + 100),
                    ))
                })
                .unwrap();
        });

        // Main thread: introduce a delay and then modify the state to create a race condition
        std::thread::sleep(Duration::from_millis(80)); // Adjust this timing as needed
        state.0.store(42, Ordering::Release);

        // Wait for the spawned thread to complete
        handle.join().unwrap();

        // Check if the state was updated as expected
        assert_eq!(state.0.load(Ordering::Acquire), 42);
    }

    // A synchronization primitive, `Barrier`, is created. This is used to
    // ensure that both threads start their operations at the same time,
    // further ensuring the race condition.
    // Two threads, `thread_a` and `thread_b`, are then spawned. `thread_a`
    // calls the `measure_and_peek_one` function on the shared state. This
    // function takes a closure that simulates some work (represented by a sleep call),
    // then calculates a new value based on the old one (if it exists),
    // or uses a default value. The closure returns this new value wrapped in a `Nanos` struct.
    // Meanwhile, `thread_b` waits for a shorter duration than `thread_a` and then
    // modifies the shared state. This is designed to simulate a race condition where
    // the value of the shared state changes after `thread_a` reads it but before `thread_a`
    // has a chance to store the new value.
    // The `measure_and_peek_one` function itself uses an atomic compare-and-exchange
    // operation to safely update the shared state. If the old value hasn't changed since it
    // was read (i.e., there was no race condition), the new value is stored.
    // If the old value has changed (i.e., there was a race condition), the function retries
    // with the updated old value. This process continues until the update is successful
    // or the provided function returns an error.
    // The test checks that the `measure_and_peek_one` function behaves correctly
    // in this multi-threaded context, specifically under race conditions.
    #[cfg(feature = "std")]
    #[test]
    fn test_measure_and_peek_one_race_condition() {
        let state = Arc::new(InMemoryState(AtomicU64::new(0)));
        let barrier = Arc::new(std::sync::Barrier::new(2));

        let state_clone_for_thread_a = Arc::clone(&state);
        let barrier_clone_for_thread_a = Arc::clone(&barrier);
        let thread_a = std::thread::spawn(move || {
            barrier_clone_for_thread_a.wait(); // Ensure both threads start together

            state_clone_for_thread_a
                .measure_and_peek_one(|old| {
                    // Simulate some work or delay if necessary
                    std::thread::sleep(Duration::from_millis(10));

                    let new_val = old.map(|n| n.as_u64() + 100).unwrap_or(100);
                    Ok::<(_, Nanos), &str>(((), Nanos::from(new_val)))
                })
                .unwrap();
        });

        let state_clone_for_thread_b = Arc::clone(&state);
        let barrier_clone_for_thread_b = Arc::clone(&barrier);
        let thread_b = std::thread::spawn(move || {
            barrier_clone_for_thread_b.wait(); // Ensure both threads start together

            // Change the atomic variable's value after Thread A reads it
            std::thread::sleep(Duration::from_millis(5));
            state_clone_for_thread_b.0.store(42, Ordering::Release);
        });

        thread_a.join().unwrap();
        thread_b.join().unwrap();
    }
}
