use crate::lib::*;

use super::DirectRateLimiter;
use crate::{clock, Jitter};
use futures::task::{Context, Poll};
use futures::Future;
use futures::Sink;
use futures_timer::Delay;
use std::pin::Pin;

pub trait SinkExt<Item, S>: Sink<Item>
where
    S: Sink<Item>,
{
    /// Limits the rate at which items can be put into the current sink.
    fn ratelimit_sink<'a, C: clock::Clock<Instant = Instant>>(
        self,
        limiter: &'a DirectRateLimiter<C>,
    ) -> RatelimitedSink<'a, Item, S, C>
    where
        Self: Sized;

    /// Limits the rate at which items can be put into the current sink, with a randomized wait
    /// period.
    fn ratelimit_sink_with_jitter<'a, C: clock::Clock<Instant = Instant>>(
        self,
        limiter: &'a DirectRateLimiter<C>,
        jitter: Jitter,
    ) -> RatelimitedSink<'a, Item, S, C>
    where
        Self: Sized;
}

impl<Item, S: Sink<Item>> SinkExt<Item, S> for S {
    fn ratelimit_sink<'a, C: clock::Clock<Instant = Instant>>(
        self,
        limiter: &'a DirectRateLimiter<C>,
    ) -> RatelimitedSink<'a, Item, S, C>
    where
        Self: Sized,
    {
        RatelimitedSink::new(self, limiter, Jitter::NONE)
    }

    fn ratelimit_sink_with_jitter<'a, C: clock::Clock<Instant = Instant>>(
        self,
        limiter: &'a DirectRateLimiter<C>,
        jitter: Jitter,
    ) -> RatelimitedSink<'a, Item, S, C>
    where
        Self: Sized,
    {
        RatelimitedSink::new(self, limiter, jitter)
    }
}

#[derive(Debug)]
enum State {
    NotReady,
    Wait,
    Ready,
}

pub struct RatelimitedSink<'a, Item, S: Sink<Item>, C: clock::Clock<Instant = Instant>> {
    inner: S,
    state: State,
    limiter: &'a DirectRateLimiter<C>,
    delay: Delay,
    jitter: Jitter,
    phantom: PhantomData<Item>,
}

impl<'a, Item, S: Sink<Item>, C: clock::Clock<Instant = Instant>> RatelimitedSink<'a, Item, S, C> {
    fn new(
        inner: S,
        limiter: &'a DirectRateLimiter<C>,
        jitter: Jitter,
    ) -> RatelimitedSink<'a, Item, S, C> {
        RatelimitedSink {
            inner,
            limiter,
            delay: Delay::new(Default::default()),
            state: State::NotReady,
            jitter,
            phantom: PhantomData,
        }
    }

    /// Acquires a reference to the underlying sink that this combinator is sending into.
    pub fn get_ref(&self) -> &S {
        &self.inner
    }

    /// Acquires a mutable reference to the underlying sink that this combinator is sending into.
    pub fn get_mut(&mut self) -> &mut S {
        &mut self.inner
    }

    /// Consumes this combinator, returning the underlying sink.
    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<'a, Item, S: Sink<Item>, C: clock::Clock<Instant = Instant>> Sink<Item>
    for RatelimitedSink<'a, Item, S, C>
where
    S: Unpin,
    Item: Unpin,
    C: Unpin,
{
    type Error = S::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        loop {
            match self.state {
                State::NotReady => {
                    if let Err(negative) = self.limiter.check() {
                        let earliest = self.jitter + negative.earliest_possible();
                        self.delay.reset(earliest);
                        let future = Pin::new(&mut self.delay);
                        match future.poll(cx) {
                            Poll::Pending => {
                                self.state = State::Wait;
                                return Poll::Pending;
                            }
                            Poll::Ready(_) => {}
                        }
                    } else {
                        self.state = State::Ready;
                    }
                }
                State::Wait => {
                    let future = Pin::new(&mut self.delay);
                    match future.poll(cx) {
                        Poll::Pending => {
                            return Poll::Pending;
                        }
                        Poll::Ready(_) => {
                            self.state = State::NotReady;
                        }
                    }
                }
                State::Ready => {
                    let inner = Pin::new(&mut self.inner);
                    return inner.poll_ready(cx);
                }
            }
        }
    }

    fn start_send(mut self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        match self.state {
            State::Wait | State::NotReady => {
                unreachable!("Protocol violation: should not start_send before we say we can");
            }
            State::Ready => {
                self.state = State::NotReady;
                let inner = Pin::new(&mut self.inner);
                inner.start_send(item)
            }
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let inner = Pin::new(&mut self.inner);
        inner.poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let inner = Pin::new(&mut self.inner);
        inner.poll_close(cx)
    }
}
