use crate::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct InterruptibleStream<V> {
    inner: Box<dyn Iterator<Item = V> + Send + Sync>,
    interrupt_signal: Arc<AtomicBool>,
}

impl<V> InterruptibleStream<V> {
    pub fn new<S>(inner: S, interrupt_signal: Arc<AtomicBool>) -> InterruptibleStream<V>
    where
        S: Iterator<Item = V> + Send + Sync + 'static,
    {
        InterruptibleStream {
            inner: Box::new(inner),
            interrupt_signal,
        }
    }
}

impl<V> Iterator for InterruptibleStream<V> {
    type Item = V;

    fn next(&mut self) -> Option<Self::Item> {
        if self.interrupt_signal.load(Ordering::SeqCst) {
            None
        } else {
            self.inner.next()
        }
    }
}

pub trait Interruptible<V> {
    fn interruptible(self, ctrl_c: Arc<AtomicBool>) -> InterruptibleStream<V>;
}

impl<S, V> Interruptible<V> for S
where
    S: Iterator<Item = V> + Send + Sync + 'static,
{
    fn interruptible(self, ctrl_c: Arc<AtomicBool>) -> InterruptibleStream<V> {
        InterruptibleStream::new(self, ctrl_c)
    }
}
