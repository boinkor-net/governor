#![cfg(feature = "std")]

use crate::lib::*;
use crate::{clock, DirectRateLimiter, Jitter};
use futures::task::{Context, Poll};
use futures::{Future, Sink, Stream};
use futures_timer::Delay;
use std::pin::Pin;

pub trait StreamRateLimitExt<'a>: Stream {
    /// Limits the rate at which the stream produces items.
    ///
    /// Note that this combinator limits the rate at which it yields
    /// items, not necessarily the rate at which the underlying stream is polled.
    /// The combinator will buffer at most one item in order to adhere to the
    /// given limiter. I.e. if it already has an item buffered and needs to wait
    /// it will not `poll` the underlying stream.
    fn ratelimit_stream(
        self,
        limiter: &'a DirectRateLimiter<clock::MonotonicClock>,
    ) -> RatelimitedStream<'a, Self>
    where
        Self: Sized;

    /// Limits the rate at which the stream produces items, with a randomized wait period.
    ///
    /// Note that this combinator limits the rate at which it yields
    /// items, not necessarily the rate at which the underlying stream is polled.
    /// The combinator will buffer at most one item in order to adhere to the
    /// given limiter. I.e. if it already has an item buffered and needs to wait
    /// it will not `poll` the underlying stream.
    fn ratelimit_stream_with_jitter(
        self,
        limiter: &'a DirectRateLimiter<clock::MonotonicClock>,
        jitter: Jitter,
    ) -> RatelimitedStream<'a, Self>
    where
        Self: Sized;
}

impl<'a, S: Stream> StreamRateLimitExt<'a> for S {
    fn ratelimit_stream(
        self,
        limiter: &'a DirectRateLimiter<clock::MonotonicClock>,
    ) -> RatelimitedStream<'a, Self>
    where
        Self: Sized,
    {
        self.ratelimit_stream_with_jitter(limiter, Jitter::NONE)
    }

    fn ratelimit_stream_with_jitter(
        self,
        limiter: &'a DirectRateLimiter<clock::MonotonicClock>,
        jitter: Jitter,
    ) -> RatelimitedStream<'a, Self>
    where
        Self: Sized,
    {
        RatelimitedStream {
            inner: self,
            limiter: limiter,
            buf: None,
            delay: Delay::new(Duration::new(0, 0)),
            jitter: jitter,
            state: State::ReadInner,
        }
    }
}

#[derive(PartialEq, Debug)]
enum State {
    ReadInner,
    NotReady,
    Wait,
}

/// A stream combinator which will limit the rate of items passing through.
///
/// This is produced by the [`StreamRateLimitExt::ratelimit_stream`] and
/// [`StreamRateLimitExt::ratelimit_stream_with_jitter`] methods.
pub struct RatelimitedStream<'a, S: Stream> {
    inner: S,
    limiter: &'a DirectRateLimiter<clock::MonotonicClock>,
    delay: Delay,
    buf: Option<S::Item>,
    jitter: Jitter,
    state: State,
}

/// Conversion methods for the stream combinator.
impl<'a, S: Stream> RatelimitedStream<'a, S> {
    /// Acquires a reference to the underlying stream that this combinator is pulling from.
    pub fn get_ref(&self) -> &S {
        &self.inner
    }

    /// Acquires a mutable reference to the underlying stream that this combinator is pulling from.
    pub fn get_mut(&mut self) -> &mut S {
        &mut self.inner
    }

    /// Consumes this combinator, returning the underlying stream and any item
    /// which it has already produced but which is still being held back
    /// in order to abide by the limiter.
    pub fn into_inner(self) -> (S, Option<S::Item>) {
        (self.inner, self.buf)
    }
}

/// Implements the [`futures::Stream`] combinator.
impl<'a, S: Stream> Stream for RatelimitedStream<'a, S>
where
    S: Unpin,
    S::Item: Unpin,
    Self: Unpin,
{
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match self.state {
                State::ReadInner => {
                    let inner = Pin::new(&mut self.inner);
                    match inner.poll_next(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(None) => {
                            // never talk tome or my inner again
                            return Poll::Ready(None);
                        }
                        Poll::Ready(Some(x)) => {
                            self.buf.replace(x);
                            self.state = State::NotReady;
                        }
                    }
                }
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
                        self.state = State::ReadInner;
                        return Poll::Ready(self.buf.take());
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
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

/// Pass-through implementation for [`futures::Sink`] if the Stream also implements it.
impl<'a, Item, S: Stream + Sink<Item>> Sink<Item> for RatelimitedStream<'a, S>
where
    S: Unpin,
    S::Item: Unpin,
{
    type Error = <S as Sink<Item>>::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let inner = Pin::new(&mut self.inner);
        inner.poll_ready(cx)
    }

    fn start_send(mut self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        let inner = Pin::new(&mut self.inner);
        inner.start_send(item)
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
