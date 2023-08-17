use crate::{ShellError, SpannedValue};

impl SpannedValue {
    pub fn as_f64(&self) -> Result<f64, ShellError> {
        match self {
            SpannedValue::Float { val, .. } => Ok(*val),
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
            SpannedValue::Int { val, .. } => Ok(*val),
            SpannedValue::Filesize { val, .. } => Ok(*val),
            SpannedValue::Duration { val, .. } => Ok(*val),
            x => Err(ShellError::CantConvert {
                to_type: "i64".into(),
                from_type: x.get_type().to_string(),
                span: self.span(),
                help: None,
            }),
        }
    }
}
