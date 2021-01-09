use crate::prelude::*;
use futures::task::Poll;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct InterruptibleStream<V> {
    inner: BoxStream<'static, V>,
    interrupt_signal: Arc<AtomicBool>,
}

impl<V> InterruptibleStream<V> {
    pub fn new<S>(inner: S, interrupt_signal: Arc<AtomicBool>) -> InterruptibleStream<V>
    where
        S: Stream<Item = V> + Send + 'static,
    {
        InterruptibleStream {
            inner: inner.boxed(),
            interrupt_signal,
        }
    }
}

impl<V> Stream for InterruptibleStream<V> {
    type Item = V;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        if self.interrupt_signal.load(Ordering::SeqCst) {
            Poll::Ready(None)
        } else {
            Stream::poll_next(std::pin::Pin::new(&mut self.inner), cx)
        }
    }
}

pub trait Interruptible<V> {
    fn interruptible(self, ctrl_c: Arc<AtomicBool>) -> InterruptibleStream<V>;
}

impl<S, V> Interruptible<V> for S
where
    S: Stream<Item = V> + Send + 'static,
{
    fn interruptible(self, ctrl_c: Arc<AtomicBool>) -> InterruptibleStream<V> {
        InterruptibleStream::new(self, ctrl_c)
    }
}
