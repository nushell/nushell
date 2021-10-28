use std::sync::{atomic::AtomicBool, Arc};

use crate::{ast::PathMember, ShellError, Span, Value, ValueStream};

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
