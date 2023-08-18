use crate::{ShellError, Value};

impl Value {
    pub fn as_f64(&self) -> Result<f64, ShellError> {
        match self {
            Value::Float { val, .. } => Ok(*val),
            x => Err(ShellError::CantConvert {
                to_type: "f64".into(),
                from_type: x.get_type().to_string(),
                span: self.span(),
                help: None,
            }),
        }
    }

    pub fn as_i64(&self) -> Result<i64, ShellError> {
        match self {
            Value::Int { val, .. } => Ok(*val),
            Value::Filesize { val, .. } => Ok(*val),
            Value::Duration { val, .. } => Ok(*val),
            x => Err(ShellError::CantConvert {
                to_type: "i64".into(),
                from_type: x.get_type().to_string(),
                span: self.span(),
                help: None,
            }),
        }
    }
}
