use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

use crate::ast::{CellPath, PathMember};
use crate::engine::{Block, Closure};
use crate::ShellError;
use crate::{Range, Spanned, Value};
use chrono::{DateTime, FixedOffset};

pub trait FromValue: Sized {
    fn from_value(v: &Value) -> Result<Self, ShellError>;
}

impl FromValue for Value {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        Ok(v.clone())
    }
}

impl FromValue for Spanned<i64> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value::Int { val, span } => Ok(Spanned {
                item: *val,
                span: *span,
            }),
            Value::Filesize { val, span } => Ok(Spanned {
                item: *val,
                span: *span,
            }),
            Value::Duration { val, span } => Ok(Spanned {
                item: *val,
                span: *span,
            }),

            v => Err(ShellError::CantConvert(
                "integer".into(),
                v.get_type().to_string(),
                v.span()?,
                None,
            )),
        }
    }
}

impl FromValue for i64 {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value::Int { val, .. } => Ok(*val),
            Value::Filesize { val, .. } => Ok(*val),
            Value::Duration { val, .. } => Ok(*val),

            v => Err(ShellError::CantConvert(
                "integer".into(),
                v.get_type().to_string(),
                v.span()?,
                None,
            )),
        }
    }
}

impl FromValue for Spanned<f64> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value::Int { val, span } => Ok(Spanned {
                item: *val as f64,
                span: *span,
            }),
            Value::Float { val, span } => Ok(Spanned {
                item: *val,
                span: *span,
            }),

            v => Err(ShellError::CantConvert(
                "float".into(),
                v.get_type().to_string(),
                v.span()?,
                None,
            )),
        }
    }
}

impl FromValue for f64 {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value::Float { val, .. } => Ok(*val),
            Value::Int { val, .. } => Ok(*val as f64),
            v => Err(ShellError::CantConvert(
                "float".into(),
                v.get_type().to_string(),
                v.span()?,
                None,
            )),
        }
    }
}

impl FromValue for Spanned<usize> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value::Int { val, span } => {
                if val.is_negative() {
                    Err(ShellError::NeedsPositiveValue(*span))
                } else {
                    Ok(Spanned {
                        item: *val as usize,
                        span: *span,
                    })
                }
            }
            Value::Filesize { val, span } => {
                if val.is_negative() {
                    Err(ShellError::NeedsPositiveValue(*span))
                } else {
                    Ok(Spanned {
                        item: *val as usize,
                        span: *span,
                    })
                }
            }
            Value::Duration { val, span } => {
                if val.is_negative() {
                    Err(ShellError::NeedsPositiveValue(*span))
                } else {
                    Ok(Spanned {
                        item: *val as usize,
                        span: *span,
                    })
                }
            }

            v => Err(ShellError::CantConvert(
                "non-negative integer".into(),
                v.get_type().to_string(),
                v.span()?,
                None,
            )),
        }
    }
}

impl FromValue for usize {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value::Int { val, span } => {
                if val.is_negative() {
                    Err(ShellError::NeedsPositiveValue(*span))
                } else {
                    Ok(*val as usize)
                }
            }
            Value::Filesize { val, span } => {
                if val.is_negative() {
                    Err(ShellError::NeedsPositiveValue(*span))
                } else {
                    Ok(*val as usize)
                }
            }
            Value::Duration { val, span } => {
                if val.is_negative() {
                    Err(ShellError::NeedsPositiveValue(*span))
                } else {
                    Ok(*val as usize)
                }
            }

            v => Err(ShellError::CantConvert(
                "non-negative integer".into(),
                v.get_type().to_string(),
                v.span()?,
                None,
            )),
        }
    }
}

impl FromValue for String {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        // FIXME: we may want to fail a little nicer here
        match v {
            Value::CellPath { val, .. } => Ok(val.into_string()),
            Value::String { val, .. } => Ok(val.clone()),
            v => Err(ShellError::CantConvert(
                "string".into(),
                v.get_type().to_string(),
                v.span()?,
                None,
            )),
        }
    }
}

impl FromValue for Spanned<String> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        Ok(Spanned {
            item: match v {
                Value::CellPath { val, .. } => val.into_string(),
                Value::String { val, .. } => val.clone(),
                v => {
                    return Err(ShellError::CantConvert(
                        "string".into(),
                        v.get_type().to_string(),
                        v.span()?,
                        None,
                    ))
                }
            },
            span: v.span()?,
        })
    }
}

impl FromValue for Vec<String> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        // FIXME: we may want to fail a little nicer here
        match v {
            Value::List { vals, .. } => vals
                .iter()
                .map(|val| match val {
                    Value::String { val, .. } => Ok(val.clone()),
                    c => Err(ShellError::CantConvert(
                        "string".into(),
                        c.get_type().to_string(),
                        c.span()?,
                        None,
                    )),
                })
                .collect::<Result<Vec<String>, ShellError>>(),
            v => Err(ShellError::CantConvert(
                "string".into(),
                v.get_type().to_string(),
                v.span()?,
                None,
            )),
        }
    }
}

impl FromValue for Vec<Spanned<String>> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        // FIXME: we may want to fail a little nicer here
        match v {
            Value::List { vals, .. } => vals
                .iter()
                .map(|val| match val {
                    Value::String { val, span } => Ok(Spanned {
                        item: val.clone(),
                        span: *span,
                    }),
                    c => Err(ShellError::CantConvert(
                        "string".into(),
                        c.get_type().to_string(),
                        c.span()?,
                        None,
                    )),
                })
                .collect::<Result<Vec<Spanned<String>>, ShellError>>(),
            v => Err(ShellError::CantConvert(
                "string".into(),
                v.get_type().to_string(),
                v.span()?,
                None,
            )),
        }
    }
}

impl FromValue for Vec<bool> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value::List { vals, .. } => vals
                .iter()
                .map(|val| match val {
                    Value::Bool { val, .. } => Ok(*val),
                    c => Err(ShellError::CantConvert(
                        "bool".into(),
                        c.get_type().to_string(),
                        c.span()?,
                        None,
                    )),
                })
                .collect::<Result<Vec<bool>, ShellError>>(),
            v => Err(ShellError::CantConvert(
                "bool".into(),
                v.get_type().to_string(),
                v.span()?,
                None,
            )),
        }
    }
}

impl FromValue for CellPath {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        let span = v.span()?;
        match v {
            Value::CellPath { val, .. } => Ok(val.clone()),
            Value::String { val, .. } => Ok(CellPath {
                members: vec![PathMember::String {
                    val: val.clone(),
                    span,
                }],
            }),
            Value::Int { val, span } => {
                if val.is_negative() {
                    Err(ShellError::NeedsPositiveValue(*span))
                } else {
                    Ok(CellPath {
                        members: vec![PathMember::Int {
                            val: *val as usize,
                            span: *span,
                        }],
                    })
                }
            }
            x => Err(ShellError::CantConvert(
                "cell path".into(),
                x.get_type().to_string(),
                span,
                None,
            )),
        }
    }
}

impl FromValue for bool {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value::Bool { val, .. } => Ok(*val),
            v => Err(ShellError::CantConvert(
                "bool".into(),
                v.get_type().to_string(),
                v.span()?,
                None,
            )),
        }
    }
}

impl FromValue for Spanned<bool> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value::Bool { val, span } => Ok(Spanned {
                item: *val,
                span: *span,
            }),
            v => Err(ShellError::CantConvert(
                "bool".into(),
                v.get_type().to_string(),
                v.span()?,
                None,
            )),
        }
    }
}

impl FromValue for DateTime<FixedOffset> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value::Date { val, .. } => Ok(*val),
            v => Err(ShellError::CantConvert(
                "date".into(),
                v.get_type().to_string(),
                v.span()?,
                None,
            )),
        }
    }
}

impl FromValue for Spanned<DateTime<FixedOffset>> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value::Date { val, span } => Ok(Spanned {
                item: *val,
                span: *span,
            }),
            v => Err(ShellError::CantConvert(
                "date".into(),
                v.get_type().to_string(),
                v.span()?,
                None,
            )),
        }
    }
}

impl FromValue for Range {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value::Range { val, .. } => Ok((**val).clone()),
            v => Err(ShellError::CantConvert(
                "range".into(),
                v.get_type().to_string(),
                v.span()?,
                None,
            )),
        }
    }
}

impl FromValue for Spanned<Range> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value::Range { val, span } => Ok(Spanned {
                item: (**val).clone(),
                span: *span,
            }),
            v => Err(ShellError::CantConvert(
                "range".into(),
                v.get_type().to_string(),
                v.span()?,
                None,
            )),
        }
    }
}

impl FromValue for Vec<u8> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value::Binary { val, .. } => Ok(val.clone()),
            Value::String { val, .. } => Ok(val.bytes().collect()),
            v => Err(ShellError::CantConvert(
                "binary data".into(),
                v.get_type().to_string(),
                v.span()?,
                None,
            )),
        }
    }
}

impl FromValue for Spanned<Vec<u8>> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value::Binary { val, span } => Ok(Spanned {
                item: val.clone(),
                span: *span,
            }),
            Value::String { val, span } => Ok(Spanned {
                item: val.bytes().collect(),
                span: *span,
            }),
            v => Err(ShellError::CantConvert(
                "binary data".into(),
                v.get_type().to_string(),
                v.span()?,
                None,
            )),
        }
    }
}

impl FromValue for Spanned<PathBuf> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value::String { val, span } => Ok(Spanned {
                item: PathBuf::from_str(val)
                    .map_err(|err| ShellError::FileNotFoundCustom(err.to_string(), *span))?,
                span: *span,
            }),
            v => Err(ShellError::CantConvert(
                "range".into(),
                v.get_type().to_string(),
                v.span()?,
                None,
            )),
        }
    }
}

impl FromValue for Vec<Value> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        // FIXME: we may want to fail a little nicer here
        match v {
            Value::List { vals, .. } => Ok(vals.clone()),
            v => Err(ShellError::CantConvert(
                "Vector of values".into(),
                v.get_type().to_string(),
                v.span()?,
                None,
            )),
        }
    }
}

// A record
impl FromValue for (Vec<String>, Vec<Value>) {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value::Record { cols, vals, .. } => Ok((cols.clone(), vals.clone())),
            v => Err(ShellError::CantConvert(
                "Record".into(),
                v.get_type().to_string(),
                v.span()?,
                None,
            )),
        }
    }
}

impl FromValue for Closure {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value::Closure { val, captures, .. } => Ok(Closure {
                block_id: *val,
                captures: captures.clone(),
            }),
            Value::Block { val, .. } => Ok(Closure {
                block_id: *val,
                captures: HashMap::new(),
            }),
            v => Err(ShellError::CantConvert(
                "Closure".into(),
                v.get_type().to_string(),
                v.span()?,
                None,
            )),
        }
    }
}

impl FromValue for Block {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value::Block { val, .. } => Ok(Block { block_id: *val }),
            v => Err(ShellError::CantConvert(
                "Block".into(),
                v.get_type().to_string(),
                v.span()?,
                None,
            )),
        }
    }
}

impl FromValue for Spanned<Closure> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value::Closure {
                val,
                captures,
                span,
            } => Ok(Spanned {
                item: Closure {
                    block_id: *val,
                    captures: captures.clone(),
                },
                span: *span,
            }),
            v => Err(ShellError::CantConvert(
                "Closure".into(),
                v.get_type().to_string(),
                v.span()?,
                None,
            )),
        }
    }
}
