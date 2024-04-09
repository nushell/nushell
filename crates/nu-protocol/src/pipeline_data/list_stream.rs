use crate::{Config, PipelineData, ShellError, Span, Value};
use std::{
    fmt::Debug,
    sync::{atomic::AtomicBool, Arc},
};

pub type ValueIterator = Box<dyn Iterator<Item = Value> + Send + 'static>;

/// A potentially infinite stream of values, optionally with a mean to send a Ctrl-C signal to stop
/// the stream from continuing.
///
/// In practice, a "stream" here means anything which can be iterated and produce Values as it iterates.
/// Like other iterators in Rust, observing values from this stream will drain the items as you view them
/// and the stream cannot be replayed.
pub struct ListStream {
    stream: ValueIterator,
    span: Span,
}

impl ListStream {
    pub fn new(
        stream: impl Iterator<Item = Value> + Send + 'static,
        span: Span,
        interrupt: Option<Arc<AtomicBool>>,
    ) -> Self {
        Self {
            stream: Box::new(Interrupt::new(stream, interrupt)),
            span,
        }
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn into_inner(self) -> ValueIterator {
        self.stream
    }

    pub fn into_string(self, separator: &str, config: &Config) -> String {
        self.into_iter()
            .map(|x: Value| x.to_expanded_string(", ", config))
            .collect::<Vec<String>>()
            .join(separator)
    }

    pub fn into_value(self) -> Value {
        Value::list(self.stream.collect(), self.span)
    }

    pub fn drain(self) -> Result<(), ShellError> {
        for next in self.into_iter() {
            if let Value::Error { error, .. } = next {
                return Err(*error);
            }
        }
        Ok(())
    }

    pub fn map(self, mapping: impl FnMut(Value) -> Value + Send + 'static) -> Self {
        Self {
            stream: Box::new(self.stream.map(mapping)),
            span: self.span,
        }
    }

    pub fn modify<I>(self, f: impl FnOnce(ValueIterator) -> I) -> Self
    where
        I: Iterator<Item = Value> + Send + 'static,
    {
        Self {
            stream: Box::new(f(self.stream)),
            span: self.span,
        }
    }
}

impl Debug for ListStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListStream").finish()
    }
}

impl IntoIterator for ListStream {
    type Item = Value;

    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            stream: self.into_inner(),
        }
    }
}

impl From<ListStream> for PipelineData {
    fn from(stream: ListStream) -> Self {
        Self::ListStream(stream, None)
    }
}

pub struct IntoIter {
    stream: ValueIterator,
}

impl Iterator for IntoIter {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        self.stream.next()
    }
}

struct Interrupt<I: Iterator> {
    iter: I,
    interrupt: Option<Arc<AtomicBool>>,
}

impl<I: Iterator> Interrupt<I> {
    fn new(iter: I, interrupt: Option<Arc<AtomicBool>>) -> Self {
        Self { iter, interrupt }
    }
}

impl<I: Iterator> Iterator for Interrupt<I> {
    type Item = <I as Iterator>::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if nu_utils::ctrl_c::was_pressed(&self.interrupt) {
            None
        } else {
            self.iter.next()
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}
