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
