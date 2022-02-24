use std::sync::{atomic::AtomicBool, Arc};

use crate::{ast::PathMember, Config, ListStream, RawStream, ShellError, Span, Value};

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
#[derive(Debug)]
pub enum PipelineData {
    Value(Value, Option<PipelineMetadata>),
    ListStream(ListStream, Option<PipelineMetadata>),
    RawStream(RawStream, Span, Option<PipelineMetadata>),
}

#[derive(Debug, Clone)]
pub struct PipelineMetadata {
    pub data_source: DataSource,
}

#[derive(Debug, Clone)]
pub enum DataSource {
    Ls,
}

impl PipelineData {
    pub fn new(span: Span) -> PipelineData {
        PipelineData::Value(Value::Nothing { span }, None)
    }

    pub fn new_with_metadata(metadata: Option<PipelineMetadata>, span: Span) -> PipelineData {
        PipelineData::Value(Value::Nothing { span }, metadata)
    }

    pub fn metadata(&self) -> Option<PipelineMetadata> {
        match self {
            PipelineData::ListStream(_, x) => x.clone(),
            PipelineData::RawStream(_, _, x) => x.clone(),
            PipelineData::Value(_, x) => x.clone(),
        }
    }

    pub fn set_metadata(mut self, metadata: Option<PipelineMetadata>) -> Self {
        match &mut self {
            PipelineData::ListStream(_, x) => *x = metadata,
            PipelineData::RawStream(_, _, x) => *x = metadata,
            PipelineData::Value(_, x) => *x = metadata,
        }

        self
    }

    pub fn is_nothing(&self) -> bool {
        matches!(self, PipelineData::Value(Value::Nothing { .. }, ..))
    }

    pub fn into_value(self, span: Span) -> Value {
        match self {
            PipelineData::Value(Value::Nothing { .. }, ..) => Value::nothing(span),
            PipelineData::Value(v, ..) => v,
            PipelineData::ListStream(s, ..) => Value::List {
                vals: s.collect(),
                span, // FIXME?
            },
            PipelineData::RawStream(mut s, ..) => {
                let mut items = vec![];

                for val in &mut s {
                    match val {
                        Ok(val) => {
                            items.push(val);
                        }
                        Err(e) => {
                            return Value::Error { error: e };
                        }
                    }
                }

                if s.is_binary {
                    let mut output = vec![];
                    for item in items {
                        match item.as_binary() {
                            Ok(item) => {
                                output.extend(item);
                            }
                            Err(err) => {
                                return Value::Error { error: err };
                            }
                        }
                    }

                    Value::Binary {
                        val: output,
                        span, // FIXME?
                    }
                } else {
                    let mut output = String::new();
                    for item in items {
                        match item.as_string() {
                            Ok(s) => output.push_str(&s),
                            Err(err) => {
                                return Value::Error { error: err };
                            }
                        }
                    }
                    Value::String {
                        val: output,
                        span, // FIXME?
                    }
                }
            }
        }
    }

    pub fn into_interruptible_iter(self, ctrlc: Option<Arc<AtomicBool>>) -> PipelineIterator {
        let mut iter = self.into_iter();

        if let PipelineIterator(PipelineData::ListStream(s, ..)) = &mut iter {
            s.ctrlc = ctrlc;
        }

        iter
    }

    pub fn collect_string(self, separator: &str, config: &Config) -> Result<String, ShellError> {
        match self {
            PipelineData::Value(v, ..) => Ok(v.into_string(separator, config)),
            PipelineData::ListStream(s, ..) => Ok(s.into_string(separator, config)),
            PipelineData::RawStream(s, ..) => {
                let mut items = vec![];

                for val in s {
                    match val {
                        Ok(val) => {
                            items.push(val);
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                }

                let mut output = String::new();
                for item in items {
                    match item.as_string() {
                        Ok(s) => output.push_str(&s),
                        Err(err) => {
                            return Err(err);
                        }
                    }
                }

                Ok(output)
            }
        }
    }

    pub fn follow_cell_path(
        self,
        cell_path: &[PathMember],
        head: Span,
    ) -> Result<Value, ShellError> {
        match self {
            // FIXME: there are probably better ways of doing this
            PipelineData::ListStream(stream, ..) => Value::List {
                vals: stream.collect(),
                span: head,
            }
            .follow_cell_path(cell_path),
            PipelineData::Value(v, ..) => v.follow_cell_path(cell_path),
            _ => Err(ShellError::IOError("can't follow stream paths".into())),
        }
    }

    pub fn update_cell_path(
        &mut self,
        cell_path: &[PathMember],
        callback: Box<dyn FnOnce(&Value) -> Value>,
        head: Span,
    ) -> Result<(), ShellError> {
        match self {
            // FIXME: there are probably better ways of doing this
            PipelineData::ListStream(stream, ..) => Value::List {
                vals: stream.collect(),
                span: head,
            }
            .update_cell_path(cell_path, callback),
            PipelineData::Value(v, ..) => v.update_cell_path(cell_path, callback),
            _ => Ok(()),
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
            PipelineData::Value(Value::List { vals, .. }, ..) => {
                Ok(vals.into_iter().map(f).into_pipeline_data(ctrlc))
            }
            PipelineData::ListStream(stream, ..) => Ok(stream.map(f).into_pipeline_data(ctrlc)),
            PipelineData::RawStream(stream, ..) => {
                let collected = stream.into_bytes()?;

                if let Ok(st) = String::from_utf8(collected.clone().item) {
                    Ok(f(Value::String {
                        val: st,
                        span: collected.span,
                    })
                    .into_pipeline_data())
                } else {
                    Ok(f(Value::Binary {
                        val: collected.item,
                        span: collected.span,
                    })
                    .into_pipeline_data())
                }
            }

            PipelineData::Value(Value::Range { val, .. }, ..) => {
                Ok(val.into_range_iter()?.map(f).into_pipeline_data(ctrlc))
            }
            PipelineData::Value(v, ..) => match f(v) {
                Value::Error { error } => Err(error),
                v => Ok(v.into_pipeline_data()),
            },
        }
    }

    /// Simplified flatmapper. For full iterator support use `.into_iter()` instead
    pub fn flat_map<U: 'static, F>(
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
            PipelineData::Value(Value::List { vals, .. }, ..) => {
                Ok(vals.into_iter().flat_map(f).into_pipeline_data(ctrlc))
            }
            PipelineData::ListStream(stream, ..) => {
                Ok(stream.flat_map(f).into_pipeline_data(ctrlc))
            }
            PipelineData::RawStream(stream, ..) => {
                let collected = stream.into_bytes()?;

                if let Ok(st) = String::from_utf8(collected.clone().item) {
                    Ok(f(Value::String {
                        val: st,
                        span: collected.span,
                    })
                    .into_iter()
                    .into_pipeline_data(ctrlc))
                } else {
                    Ok(f(Value::Binary {
                        val: collected.item,
                        span: collected.span,
                    })
                    .into_iter()
                    .into_pipeline_data(ctrlc))
                }
            }
            PipelineData::Value(Value::Range { val, .. }, ..) => match val.into_range_iter() {
                Ok(iter) => Ok(iter.flat_map(f).into_pipeline_data(ctrlc)),
                Err(error) => Err(error),
            },
            PipelineData::Value(v, ..) => Ok(f(v).into_iter().into_pipeline_data(ctrlc)),
        }
    }

    pub fn filter<F>(
        self,
        mut f: F,
        ctrlc: Option<Arc<AtomicBool>>,
    ) -> Result<PipelineData, ShellError>
    where
        Self: Sized,
        F: FnMut(&Value) -> bool + 'static + Send,
    {
        match self {
            PipelineData::Value(Value::List { vals, .. }, ..) => {
                Ok(vals.into_iter().filter(f).into_pipeline_data(ctrlc))
            }
            PipelineData::ListStream(stream, ..) => Ok(stream.filter(f).into_pipeline_data(ctrlc)),
            PipelineData::RawStream(stream, ..) => {
                let collected = stream.into_bytes()?;

                if let Ok(st) = String::from_utf8(collected.clone().item) {
                    let v = Value::String {
                        val: st,
                        span: collected.span,
                    };

                    if f(&v) {
                        Ok(v.into_pipeline_data())
                    } else {
                        Ok(PipelineData::new(collected.span))
                    }
                } else {
                    let v = Value::Binary {
                        val: collected.item,
                        span: collected.span,
                    };

                    if f(&v) {
                        Ok(v.into_pipeline_data())
                    } else {
                        Ok(PipelineData::new(collected.span))
                    }
                }
            }
            PipelineData::Value(Value::Range { val, .. }, ..) => {
                Ok(val.into_range_iter()?.filter(f).into_pipeline_data(ctrlc))
            }
            PipelineData::Value(v, ..) => {
                if f(&v) {
                    Ok(v.into_pipeline_data())
                } else {
                    Ok(Value::Nothing { span: v.span()? }.into_pipeline_data())
                }
            }
        }
    }
}

pub struct PipelineIterator(PipelineData);

impl IntoIterator for PipelineData {
    type Item = Value;

    type IntoIter = PipelineIterator;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            PipelineData::Value(Value::List { vals, .. }, metadata) => {
                PipelineIterator(PipelineData::ListStream(
                    ListStream {
                        stream: Box::new(vals.into_iter()),
                        ctrlc: None,
                    },
                    metadata,
                ))
            }
            PipelineData::Value(Value::Range { val, .. }, metadata) => {
                match val.into_range_iter() {
                    Ok(iter) => PipelineIterator(PipelineData::ListStream(
                        ListStream {
                            stream: Box::new(iter),
                            ctrlc: None,
                        },
                        metadata,
                    )),
                    Err(error) => PipelineIterator(PipelineData::ListStream(
                        ListStream {
                            stream: Box::new(std::iter::once(Value::Error { error })),
                            ctrlc: None,
                        },
                        metadata,
                    )),
                }
            }
            x => PipelineIterator(x),
        }
    }
}

impl Iterator for PipelineIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            PipelineData::Value(Value::Nothing { .. }, ..) => None,
            PipelineData::Value(v, ..) => Some(std::mem::take(v)),
            PipelineData::ListStream(stream, ..) => stream.next(),
            PipelineData::RawStream(stream, ..) => stream.next().map(|x| match x {
                Ok(x) => x,
                Err(err) => Value::Error { error: err },
            }),
        }
    }
}

pub trait IntoPipelineData {
    fn into_pipeline_data(self) -> PipelineData;
}

impl<V> IntoPipelineData for V
where
    V: Into<Value>,
{
    fn into_pipeline_data(self) -> PipelineData {
        PipelineData::Value(self.into(), None)
    }
}

pub trait IntoInterruptiblePipelineData {
    fn into_pipeline_data(self, ctrlc: Option<Arc<AtomicBool>>) -> PipelineData;
    fn into_pipeline_data_with_metadata(
        self,
        metadata: PipelineMetadata,
        ctrlc: Option<Arc<AtomicBool>>,
    ) -> PipelineData;
}

impl<I> IntoInterruptiblePipelineData for I
where
    I: IntoIterator + Send + 'static,
    I::IntoIter: Send + 'static,
    <I::IntoIter as Iterator>::Item: Into<Value>,
{
    fn into_pipeline_data(self, ctrlc: Option<Arc<AtomicBool>>) -> PipelineData {
        PipelineData::ListStream(
            ListStream {
                stream: Box::new(self.into_iter().map(Into::into)),
                ctrlc,
            },
            None,
        )
    }

    fn into_pipeline_data_with_metadata(
        self,
        metadata: PipelineMetadata,
        ctrlc: Option<Arc<AtomicBool>>,
    ) -> PipelineData {
        PipelineData::ListStream(
            ListStream {
                stream: Box::new(self.into_iter().map(Into::into)),
                ctrlc,
            },
            Some(metadata),
        )
    }
}
