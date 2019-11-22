use crate::lib::*;

use crate::clock::Clock;
use crate::nanos::Nanos;
use lazy_static::*;
use quanta;

/// A clock using the [`quanta`] crate.
///
/// It works by keeping a time keeping thread that updates a reference time every 100ns.
#[derive(Debug, Clone)]
pub struct QuantaClock(quanta::Clock);

impl Default for QuantaClock {
    fn default() -> Self {
        lazy_static! {
            static ref HANDLE: quanta::Handle = quanta::Builder::new(Duration::from_nanos(100))
                .start()
                .expect("should build the reference handle");
        }
        QuantaClock(Default::default())
    }
}

impl Clock for QuantaClock {
    type Instant = Nanos;

    fn now(&self) -> Self::Instant {
        Nanos::from(self.0.recent())
    }
}
