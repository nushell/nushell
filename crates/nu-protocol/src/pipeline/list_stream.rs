//! Module managing the streaming of individual [`Value`]s as a [`ListStream`] between pipeline
//! elements
//!
//! For more general infos regarding our pipelining model refer to [`PipelineData`]
use crate::{Config, PipelineData, ShellError, Signals, Span, Value};
use std::fmt::Debug;

pub type ValueIterator = Box<dyn Iterator<Item = Value> + Send + 'static>;

/// A potentially infinite, interruptible stream of [`Value`]s.
///
/// In practice, a "stream" here means anything which can be iterated and produces Values.
/// Like other iterators in Rust, observing values from this stream will drain the items
/// as you view them and the stream cannot be replayed.
pub struct ListStream {
    stream: ValueIterator,
    span: Span,
    caller_spans: Vec<Span>,
}

impl ListStream {
    /// Create a new [`ListStream`] from a [`Value`] `Iterator`.
    pub fn new(
        iter: impl Iterator<Item = Value> + Send + 'static,
        span: Span,
        signals: Signals,
    ) -> Self {
        Self {
            stream: Box::new(InterruptIter::new(iter, signals)),
            span,
            caller_spans: vec![],
        }
    }

    /// Returns the [`Span`] associated with this [`ListStream`].
    pub fn span(&self) -> Span {
        self.span
    }

    /// Push a caller [`Span`] to the bytestream, it's useful to construct a backtrace.
    pub fn push_caller_span(&mut self, span: Span) {
        if span != self.span {
            self.caller_spans.push(span)
        }
    }

    /// Get all caller [`Span`], it's useful to construct a backtrace.
    pub fn get_caller_spans(&self) -> &Vec<Span> {
        &self.caller_spans
    }

    /// Changes the [`Span`] associated with this [`ListStream`].
    pub fn with_span(mut self, span: Span) -> Self {
        self.span = span;
        self
    }

    /// Convert a [`ListStream`] into its inner [`Value`] `Iterator`.
    pub fn into_inner(self) -> ValueIterator {
        self.stream
    }

    /// Take a single value from the inner `Iterator`, modifying the stream.
    pub fn next_value(&mut self) -> Option<Value> {
        self.stream.next()
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
    /// use nu_protocol::{ListStream, Signals, Span, Value};
    ///
    /// let span = Span::unknown();
    /// let stream = ListStream::new(std::iter::repeat(Value::int(0, span)), span, Signals::empty());
    /// let new_stream = stream.modify(|iter| iter.take(100));
    /// ```
    pub fn modify<I>(self, f: impl FnOnce(ValueIterator) -> I) -> Self
    where
        I: Iterator<Item = Value> + Send + 'static,
    {
        Self {
            stream: Box::new(f(self.stream)),
            span: self.span,
            caller_spans: self.caller_spans,
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
        Self::list_stream(stream, None)
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

struct InterruptIter<I: Iterator> {
    iter: I,
    signals: Signals,
}

impl<I: Iterator> InterruptIter<I> {
    fn new(iter: I, signals: Signals) -> Self {
        Self { iter, signals }
    }
}

impl<I: Iterator> Iterator for InterruptIter<I> {
    type Item = <I as Iterator>::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.signals.interrupted() {
            None
        } else {
            self.iter.next()
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}
