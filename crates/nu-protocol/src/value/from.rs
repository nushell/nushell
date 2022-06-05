use crate::{ShellError, Span, Value};

impl Value {
    pub fn as_f64(&self, span: Span) -> Result<f64, ShellError> {
        match self {
            Value::Float(val) => Ok(*val),
            x => Err(ShellError::CantConvert(
                "f64".into(),
                x.get_type().to_string(),
                span,
                None,
            )),
        }
    }

    pub fn as_i64(&self, span: Span) -> Result<i64, ShellError> {
        match self {
            Value::Int(val) => Ok(*val),
            x => Err(ShellError::CantConvert(
                "i64".into(),
                x.get_type().to_string(),
                span,
                None,
            )),
        }
    }
}
