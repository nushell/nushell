use crate::{
    ast::{CellPath, PathMember},
    engine::Closure,
    NuGlob, Range, Record, ShellError, Spanned, Value,
};
use chrono::{DateTime, FixedOffset};
use std::path::PathBuf;

/// A trait for loading a value from a `Value`.
pub trait FromValue: Sized {
    // TODO: instead of ShellError, maybe we could have a FromValueError that implements Into<ShellError>
    /// Loads a value from a `Value`.
    ///
    /// Just like [`FromStr`](std::str::FromStr), this operation may fail
    /// because the raw `Value` is able to represent more values than the
    /// expected value here.
    fn from_value(v: Value) -> Result<Self, ShellError>;
}

impl FromValue for Value {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        Ok(v)
    }
}

impl<T> FromValue for Spanned<T> where T: FromValue {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        let span = v.span();
        Ok(Spanned {
            item: T::from_value(v)?,
            span
        })
    }
}

impl FromValue for i64 {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        match v {
            Value::Int { val, .. } => Ok(val),
            Value::Filesize { val, .. } => Ok(val),
            Value::Duration { val, .. } => Ok(val),

            v => Err(ShellError::CantConvert {
                to_type: "int".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for f64 {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        match v {
            Value::Float { val, .. } => Ok(val),
            Value::Int { val, .. } => Ok(val as f64),
            v => Err(ShellError::CantConvert {
                to_type: "float".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for usize {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        let span = v.span();
        match v {
            Value::Int { val, .. } => {
                if val.is_negative() {
                    Err(ShellError::NeedsPositiveValue { span })
                } else {
                    Ok(val as usize)
                }
            }
            Value::Filesize { val, .. } => {
                if val.is_negative() {
                    Err(ShellError::NeedsPositiveValue { span })
                } else {
                    Ok(val as usize)
                }
            }
            Value::Duration { val, .. } => {
                if val.is_negative() {
                    Err(ShellError::NeedsPositiveValue { span })
                } else {
                    Ok(val as usize)
                }
            }

            v => Err(ShellError::CantConvert {
                to_type: "non-negative int".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for String {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        // FIXME: we may want to fail a little nicer here
        match v {
            Value::CellPath { val, .. } => Ok(val.to_string()),
            Value::String { val, .. } => Ok(val),
            v => Err(ShellError::CantConvert {
                to_type: "string".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for NuGlob {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        // FIXME: we may want to fail a little nicer here
        match v {
            Value::CellPath { val, .. } => Ok(NuGlob::Expand(val.to_string())),
            Value::String { val, .. } => Ok(NuGlob::DoNotExpand(val)),
            Value::Glob {
                val,
                no_expand: quoted,
                ..
            } => {
                if quoted {
                    Ok(NuGlob::DoNotExpand(val))
                } else {
                    Ok(NuGlob::Expand(val))
                }
            }
            v => Err(ShellError::CantConvert {
                to_type: "string".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for Vec<String> {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        // FIXME: we may want to fail a little nicer here
        match v {
            Value::List { vals, .. } => vals
                .into_iter()
                .map(|val| match val {
                    Value::String { val, .. } => Ok(val),
                    c => Err(ShellError::CantConvert {
                        to_type: "string".into(),
                        from_type: c.get_type().to_string(),
                        span: c.span(),
                        help: None,
                    }),
                })
                .collect::<Result<Vec<String>, ShellError>>(),
            v => Err(ShellError::CantConvert {
                to_type: "string".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for Vec<Spanned<String>> {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        // FIXME: we may want to fail a little nicer here
        match v {
            Value::List { vals, .. } => vals
                .into_iter()
                .map(|val| {
                    let val_span = val.span();
                    match val {
                        Value::String { val, .. } => Ok(Spanned {
                            item: val,
                            span: val_span,
                        }),
                        c => Err(ShellError::CantConvert {
                            to_type: "string".into(),
                            from_type: c.get_type().to_string(),
                            span: c.span(),
                            help: None,
                        }),
                    }
                })
                .collect::<Result<Vec<Spanned<String>>, ShellError>>(),
            v => Err(ShellError::CantConvert {
                to_type: "string".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for Vec<bool> {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        match v {
            Value::List { vals, .. } => vals
                .into_iter()
                .map(|val| match val {
                    Value::Bool { val, .. } => Ok(val),
                    c => Err(ShellError::CantConvert {
                        to_type: "bool".into(),
                        from_type: c.get_type().to_string(),
                        span: c.span(),
                        help: None,
                    }),
                })
                .collect::<Result<Vec<bool>, ShellError>>(),
            v => Err(ShellError::CantConvert {
                to_type: "bool".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for CellPath {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        let span = v.span();
        match v {
            Value::CellPath { val, .. } => Ok(val),
            Value::String { val, .. } => Ok(CellPath {
                members: vec![PathMember::String {
                    val,
                    span,
                    optional: false,
                }],
            }),
            Value::Int { val, .. } => {
                if val.is_negative() {
                    Err(ShellError::NeedsPositiveValue { span })
                } else {
                    Ok(CellPath {
                        members: vec![PathMember::Int {
                            val: val as usize,
                            span,
                            optional: false,
                        }],
                    })
                }
            }
            x => Err(ShellError::CantConvert {
                to_type: "cell path".into(),
                from_type: x.get_type().to_string(),
                span,
                help: None,
            }),
        }
    }
}

impl FromValue for bool {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        match v {
            Value::Bool { val, .. } => Ok(val),
            v => Err(ShellError::CantConvert {
                to_type: "bool".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for DateTime<FixedOffset> {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        match v {
            Value::Date { val, .. } => Ok(val),
            v => Err(ShellError::CantConvert {
                to_type: "date".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for Range {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        match v {
            Value::Range { val, .. } => Ok(*val),
            v => Err(ShellError::CantConvert {
                to_type: "range".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for Vec<u8> {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        match v {
            Value::Binary { val, .. } => Ok(val),
            Value::String { val, .. } => Ok(val.into_bytes()),
            v => Err(ShellError::CantConvert {
                to_type: "binary data".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for Spanned<PathBuf> {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        let span = v.span();
        match v {
            Value::String { val, .. } => Ok(Spanned {
                item: val.into(),
                span,
            }),
            v => Err(ShellError::CantConvert {
                to_type: "range".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for Vec<Value> {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        // FIXME: we may want to fail a little nicer here
        match v {
            Value::List { vals, .. } => Ok(vals),
            v => Err(ShellError::CantConvert {
                to_type: "Vector of values".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for Record {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        match v {
            Value::Record { val, .. } => Ok(val.into_owned()),
            v => Err(ShellError::CantConvert {
                to_type: "Record".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for Closure {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        match v {
            Value::Closure { val, .. } => Ok(*val),
            v => Err(ShellError::CantConvert {
                to_type: "Closure".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}
