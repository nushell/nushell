use crate::{Span, Value, ValueStream};

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
}

impl Default for PipelineData {
    fn default() -> Self {
        PipelineData::new()
    }
}

impl Iterator for PipelineData {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
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

impl<T> IntoPipelineData for T
where
    T: Iterator<Item = Value> + Send + 'static,
{
    fn into_pipeline_data(self) -> PipelineData {
        PipelineData::Stream(ValueStream(Box::new(self)))
    }
}
