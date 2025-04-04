use nu_protocol::{ShellError, Span};
use polars::prelude::{DataType, UnknownKind};

#[derive(Debug, Clone)]
pub struct NuDataType {
    dtype: DataType,
}

impl NuDataType {
    pub fn new(dtype: DataType) -> Self {
        Self { dtype }
    }

    pub fn to_polars(&self) -> DataType {
        self.dtype.clone()
    }
}

