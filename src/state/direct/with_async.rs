use crate::lib::*;

use super::DirectRateLimiter;
use crate::clock;
use futures_timer::Delay;

impl<C: clock::Clock<Instant = Instant>> DirectRateLimiter<C> {
    /// Returns a future that resolves as soon as the rate limiter allows it.
    pub async fn until_ready(&self) {
        while let Err(negative) = self.check() {
            let delay = Delay::new(negative.wait_time_from(self.clock.now()));
            delay.await;
        }
    }
}
