use std::sync::{atomic::AtomicBool, Arc};

use crate::{ast::PathMember, ShellError, Span, Value, ValueStream};

/// The foundational abstraction for input and output to commands
///
/// This represents either a single Value or a stream of values coming into the command or leaving a command.
///
/// A note on implementation:
///
/// We've tried a few variations of this structure. Listing these below so we have a record.
///
/// * We tried always assuming a stream in Nushell. This was a great 80% solution, but it had some rough edges.
/// Namely, how do you know the difference between a single string and a list of one string. How do you know
/// when to flatten the data given to you from a data source into the stream or to keep it as an unflattened
/// list?
///
/// * We tried putting the stream into Value. This had some interesting properties as now commands "just worked
/// on values", but lead to a few unfortunate issues.
///
/// The first is that you can't easily clone Values in a way that felt largely immutable. For example, if
/// you cloned a Value which contained a stream, and in one variable drained some part of it, then the second
/// variable would see different values based on what you did to the first.
///
/// To make this kind of mutation thread-safe, we would have had to produce a lock for the stream, which in
/// practice would have meant always locking the stream before reading from it. But more fundamentally, it
/// felt wrong in practice that observation of a value at runtime could affect other values which happen to
/// alias the same stream. By separating these, we don't have this effect. Instead, variables could get
/// concrete list values rather than streams, and be able to view them without non-local effects.
///
/// * A balance of the two approaches is what we've landed on: Values are thread-safe to pass, and we can stream
/// them into any sources. Streams are still available to model the infinite streams approach of original
/// Nushell.
pub enum PipelineData {
    Value(Value),
    Stream(ValueStream),
}

impl PipelineData {
    pub fn new() -> PipelineData {
        PipelineData::Value(Value::nothing())
    }

    pub fn into_value(self) -> Value {
        match self {
            PipelineData::Value(v) => v,
            PipelineData::Stream(s) => Value::List {
                vals: s.collect(),
                span: Span::unknown(), // FIXME?
            },
        }
    }

    pub fn collect_string(self) -> String {
        match self {
            PipelineData::Value(v) => v.collect_string(),
            PipelineData::Stream(s) => s.collect_string(),
        }
    }

    pub fn follow_cell_path(self, cell_path: &[PathMember]) -> Result<Value, ShellError> {
        match self {
            // FIXME: there are probably better ways of doing this
            PipelineData::Stream(stream) => Value::List {
                vals: stream.collect(),
                span: Span::unknown(),
            }
            .follow_cell_path(cell_path),
            PipelineData::Value(v) => v.follow_cell_path(cell_path),
        }
    }

    /// Simplified mapper to help with simple values also. For full iterator support use `.into_iter()` instead
    pub fn map<F>(
        self,
        mut f: F,
        ctrlc: Option<Arc<AtomicBool>>,
    ) -> Result<PipelineData, ShellError>
    where
        Self: Sized,
        F: FnMut(Value) -> Value + 'static + Send,
    {
        match self {
            PipelineData::Value(Value::List { vals, .. }) => {
                Ok(vals.into_iter().map(f).into_pipeline_data(ctrlc))
            }
            PipelineData::Stream(stream) => Ok(stream.map(f).into_pipeline_data(ctrlc)),
            PipelineData::Value(Value::Range { val, .. }) => {
                Ok(val.into_range_iter()?.map(f).into_pipeline_data(ctrlc))
            }
            PipelineData::Value(v) => {
                let output = f(v);
                match output {
                    Value::Error { error } => Err(error),
                    v => Ok(v.into_pipeline_data()),
                }
            }
        }
    }

    /// Simplified flatmapper. For full iterator support use `.into_iter()` instead
    pub fn flat_map<U, F>(
        self,
        mut f: F,
        ctrlc: Option<Arc<AtomicBool>>,
    ) -> Result<PipelineData, ShellError>
    where
        Self: Sized,
        U: IntoIterator<Item = Value>,
        <U as IntoIterator>::IntoIter: 'static + Send,
        F: FnMut(Value) -> U + 'static + Send,
    {
        match self {
            PipelineData::Value(Value::List { vals, .. }) => {
                Ok(vals.into_iter().map(f).flatten().into_pipeline_data(ctrlc))
            }
            PipelineData::Stream(stream) => Ok(stream.map(f).flatten().into_pipeline_data(ctrlc)),
            PipelineData::Value(Value::Range { val, .. }) => match val.into_range_iter() {
                Ok(iter) => Ok(iter.map(f).flatten().into_pipeline_data(ctrlc)),
                Err(error) => Err(error),
            },
            PipelineData::Value(v) => Ok(f(v).into_iter().into_pipeline_data(ctrlc)),
        }
    }
}

impl Default for PipelineData {
    fn default() -> Self {
        PipelineData::new()
    }
}

pub struct PipelineIterator(PipelineData);

impl IntoIterator for PipelineData {
    type Item = Value;

    type IntoIter = PipelineIterator;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            PipelineData::Value(Value::List { vals, .. }) => {
                PipelineIterator(PipelineData::Stream(ValueStream {
                    stream: Box::new(vals.into_iter()),
                    ctrlc: None,
                }))
            }
            PipelineData::Value(Value::Range { val, .. }) => match val.into_range_iter() {
                Ok(val) => PipelineIterator(PipelineData::Stream(ValueStream {
                    stream: Box::new(val),
                    ctrlc: None,
                })),
                Err(e) => PipelineIterator(PipelineData::Stream(ValueStream {
                    stream: Box::new(vec![Value::Error { error: e }].into_iter()),
                    ctrlc: None,
                })),
            },
            x => PipelineIterator(x),
        }
    }
}

impl Iterator for PipelineIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            PipelineData::Value(Value::Nothing { .. }) => None,
            PipelineData::Value(v) => {
                let prev = std::mem::take(v);
                Some(prev)
            }
            PipelineData::Stream(stream) => stream.next(),
        }
    }
}

pub trait IntoPipelineData {
    fn into_pipeline_data(self) -> PipelineData;
}

impl IntoPipelineData for Value {
    fn into_pipeline_data(self) -> PipelineData {
        PipelineData::Value(self)
    }
}

pub trait IntoInterruptiblePipelineData {
    fn into_pipeline_data(self, ctrlc: Option<Arc<AtomicBool>>) -> PipelineData;
}

impl<T> IntoInterruptiblePipelineData for T
where
    T: Iterator<Item = Value> + Send + 'static,
{
    fn into_pipeline_data(self, ctrlc: Option<Arc<AtomicBool>>) -> PipelineData {
        PipelineData::Stream(ValueStream {
            stream: Box::new(self),
            ctrlc,
        })
    }
}
