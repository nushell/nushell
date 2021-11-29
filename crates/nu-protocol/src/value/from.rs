use crate::{ShellError, Span, Value};

impl From<String> for Value {
    fn from(val: String) -> Self {
        Value::String {
            val,
            span: Span::unknown(),
        }
    }
}

impl From<bool> for Value {
    fn from(val: bool) -> Self {
        Value::Bool {
            val,
            span: Span::unknown(),
        }
    }
}

impl From<u8> for Value {
    fn from(val: u8) -> Self {
        Value::Int {
            val: val as i64,
            span: Span::unknown(),
        }
    }
}

impl From<u16> for Value {
    fn from(val: u16) -> Self {
        Value::Int {
            val: val as i64,
            span: Span::unknown(),
        }
    }
}

impl From<u32> for Value {
    fn from(val: u32) -> Self {
        Value::Int {
            val: val as i64,
            span: Span::unknown(),
        }
    }
}

impl From<u64> for Value {
    fn from(val: u64) -> Self {
        Value::Int {
            val: val as i64,
            span: Span::unknown(),
        }
    }
}

impl From<i8> for Value {
    fn from(val: i8) -> Self {
        Value::Int {
            val: val as i64,
            span: Span::unknown(),
        }
    }
}

impl From<i16> for Value {
    fn from(val: i16) -> Self {
        Value::Int {
            val: val as i64,
            span: Span::unknown(),
        }
    }
}

impl From<i32> for Value {
    fn from(val: i32) -> Self {
        Value::Int {
            val: val as i64,
            span: Span::unknown(),
        }
    }
}

impl From<i64> for Value {
    fn from(val: i64) -> Self {
        Value::Int {
            val: val as i64,
            span: Span::unknown(),
        }
    }
}

impl From<f32> for Value {
    fn from(val: f32) -> Self {
        Value::Float {
            val: val as f64,
            span: Span::unknown(),
        }
    }
}

impl From<f64> for Value {
    fn from(val: f64) -> Self {
        Value::Float {
            val: val as f64,
            span: Span::unknown(),
        }
    }
}

impl Value {
    pub fn as_f64(&self) -> Result<f64, ShellError> {
        match self {
            Value::Float { val, .. } => Ok(*val),
            x => Err(ShellError::CantConvert(
                "f64".into(),
                x.get_type().to_string(),
                self.span()?,
            )),
        }
    }

    pub fn as_i64(&self) -> Result<i64, ShellError> {
        match self {
            Value::Int { val, .. } => Ok(*val),
            x => Err(ShellError::CantConvert(
                "rf64".into(),
                x.get_type().to_string(),
                self.span()?,
            )),
        }
    }
}
