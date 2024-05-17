use crate::{Config, PipelineData, ShellError, Span, Value};
use std::{
    fmt::Debug,
    sync::{atomic::AtomicBool, Arc},
};

pub type ValueIterator = Box<dyn Iterator<Item = Value> + Send + 'static>;

/// A potentially infinite, interruptible stream of [`Value`]s.
///
/// In practice, a "stream" here means anything which can be iterated and produces Values.
/// Like other iterators in Rust, observing values from this stream will drain the items
/// as you view them and the stream cannot be replayed.
pub struct ListStream {
    stream: ValueIterator,
    span: Span,
}

impl ListStream {
    /// Create a new [`ListStream`] from a [`Value`] `Iterator`.
    pub fn new(
        iter: impl Iterator<Item = Value> + Send + 'static,
        span: Span,
        interrupt: Option<Arc<AtomicBool>>,
    ) -> Self {
        Self {
            stream: Box::new(Interrupt::new(iter, interrupt)),
            span,
        }
    }

    /// Returns the [`Span`] associated with this [`ListStream`].
    pub fn span(&self) -> Span {
        self.span
    }

    /// Convert a [`ListStream`] into its inner [`Value`] `Iterator`.
    pub fn into_inner(self) -> ValueIterator {
        self.stream
    }

    /// Converts each value in a [`ListStream`] into a string and then joins the strings together
    /// using the given separator.
    pub fn into_string(self, separator: &str, config: &Config) -> String {
        self.into_iter()
            .map(|val| val.to_expanded_string(", ", config))
            .collect::<Vec<String>>()
            .join(separator)
    }

    /// Collect the values of a [`ListStream`] into a list [`Value`].
    pub fn into_value(self) -> Value {
        Value::list(self.stream.collect(), self.span)
    }

    /// Consume all values in the stream, returning an error if any of the values is a `Value::Error`.
    pub fn drain(self) -> Result<(), ShellError> {
        for next in self {
            if let Value::Error { error, .. } = next {
                return Err(*error);
            }
        }
        Ok(())
    }

    /// Modify the inner iterator of a [`ListStream`] using a function.
    ///
    /// This can be used to call any number of standard iterator functions on the [`ListStream`].
    /// E.g., `take`, `filter`, `step_by`, and more.
    ///
    /// ```
    /// use nu_protocol::{ListStream, Span, Value};
    ///
    /// let span = Span::unknown();
    /// let stream = ListStream::new(std::iter::repeat(Value::int(0, span)), span, None);
    /// let new_stream = stream.modify(|iter| iter.take(100));
    /// ```
    pub fn modify<I>(self, f: impl FnOnce(ValueIterator) -> I) -> Self
    where
        I: Iterator<Item = Value> + Send + 'static,
    {
        Self {
            stream: Box::new(f(self.stream)),
            span: self.span,
        }
    }

    /// Create a new [`ListStream`] whose values are the results of applying the given function
    /// to each of the values in the original [`ListStream`].
    pub fn map(self, mapping: impl FnMut(Value) -> Value + Send + 'static) -> Self {
        self.modify(|iter| iter.map(mapping))
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
