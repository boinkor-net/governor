use super::FakeRelativeClock;

/// The default `no_std` clock that reports [`Durations`] must be advanced by the program.
pub type DefaultClock = FakeRelativeClock;
