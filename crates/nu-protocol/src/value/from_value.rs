use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

use crate::ast::{CellPath, MatchPattern, PathMember};
use crate::engine::{Block, Closure};
use crate::ShellError;
use crate::{Range, Spanned, SpannedValue};
use chrono::{DateTime, FixedOffset};

pub trait FromValue: Sized {
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError>;
}

impl FromValue for SpannedValue {
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        Ok(v.clone())
    }
}

impl FromValue for Spanned<i64> {
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        match v {
            SpannedValue::Int { val, span } => Ok(Spanned {
                item: *val,
                span: *span,
            }),
            SpannedValue::Filesize { val, span } => Ok(Spanned {
                item: *val,
                span: *span,
            }),
            SpannedValue::Duration { val, span } => Ok(Spanned {
                item: *val,
                span: *span,
            }),

            v => Err(ShellError::CantConvert {
                to_type: "integer".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for i64 {
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        match v {
            SpannedValue::Int { val, .. } => Ok(*val),
            SpannedValue::Filesize { val, .. } => Ok(*val),
            SpannedValue::Duration { val, .. } => Ok(*val),

            v => Err(ShellError::CantConvert {
                to_type: "integer".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for Spanned<f64> {
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        match v {
            SpannedValue::Int { val, span } => Ok(Spanned {
                item: *val as f64,
                span: *span,
            }),
            SpannedValue::Float { val, span } => Ok(Spanned {
                item: *val,
                span: *span,
            }),

            v => Err(ShellError::CantConvert {
                to_type: "float".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for f64 {
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        match v {
            SpannedValue::Float { val, .. } => Ok(*val),
            SpannedValue::Int { val, .. } => Ok(*val as f64),
            v => Err(ShellError::CantConvert {
                to_type: "float".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for Spanned<usize> {
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        match v {
            SpannedValue::Int { val, span } => {
                if val.is_negative() {
                    Err(ShellError::NeedsPositiveValue(*span))
                } else {
                    Ok(Spanned {
                        item: *val as usize,
                        span: *span,
                    })
                }
            }
            SpannedValue::Filesize { val, span } => {
                if val.is_negative() {
                    Err(ShellError::NeedsPositiveValue(*span))
                } else {
                    Ok(Spanned {
                        item: *val as usize,
                        span: *span,
                    })
                }
            }
            SpannedValue::Duration { val, span } => {
                if val.is_negative() {
                    Err(ShellError::NeedsPositiveValue(*span))
                } else {
                    Ok(Spanned {
                        item: *val as usize,
                        span: *span,
                    })
                }
            }

            v => Err(ShellError::CantConvert {
                to_type: "non-negative integer".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for usize {
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        match v {
            SpannedValue::Int { val, span } => {
                if val.is_negative() {
                    Err(ShellError::NeedsPositiveValue(*span))
                } else {
                    Ok(*val as usize)
                }
            }
            SpannedValue::Filesize { val, span } => {
                if val.is_negative() {
                    Err(ShellError::NeedsPositiveValue(*span))
                } else {
                    Ok(*val as usize)
                }
            }
            SpannedValue::Duration { val, span } => {
                if val.is_negative() {
                    Err(ShellError::NeedsPositiveValue(*span))
                } else {
                    Ok(*val as usize)
                }
            }

            v => Err(ShellError::CantConvert {
                to_type: "non-negative integer".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for String {
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        // FIXME: we may want to fail a little nicer here
        match v {
            SpannedValue::CellPath { val, .. } => Ok(val.into_string()),
            SpannedValue::String { val, .. } => Ok(val.clone()),
            v => Err(ShellError::CantConvert {
                to_type: "string".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for Spanned<String> {
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        Ok(Spanned {
            item: match v {
                SpannedValue::CellPath { val, .. } => val.into_string(),
                SpannedValue::String { val, .. } => val.clone(),
                v => {
                    return Err(ShellError::CantConvert {
                        to_type: "string".into(),
                        from_type: v.get_type().to_string(),
                        span: v.span(),
                        help: None,
                    })
                }
            },
            span: v.span(),
        })
    }
}

impl FromValue for Vec<String> {
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        // FIXME: we may want to fail a little nicer here
        match v {
            SpannedValue::List { vals, .. } => vals
                .iter()
                .map(|val| match val {
                    SpannedValue::String { val, .. } => Ok(val.clone()),
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
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        // FIXME: we may want to fail a little nicer here
        match v {
            SpannedValue::List { vals, .. } => vals
                .iter()
                .map(|val| match val {
                    SpannedValue::String { val, span } => Ok(Spanned {
                        item: val.clone(),
                        span: *span,
                    }),
                    c => Err(ShellError::CantConvert {
                        to_type: "string".into(),
                        from_type: c.get_type().to_string(),
                        span: c.span(),
                        help: None,
                    }),
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
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        match v {
            SpannedValue::List { vals, .. } => vals
                .iter()
                .map(|val| match val {
                    SpannedValue::Bool { val, .. } => Ok(*val),
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
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        let span = v.span();
        match v {
            SpannedValue::CellPath { val, .. } => Ok(val.clone()),
            SpannedValue::String { val, .. } => Ok(CellPath {
                members: vec![PathMember::String {
                    val: val.clone(),
                    span,
                    optional: false,
                }],
            }),
            SpannedValue::Int { val, span } => {
                if val.is_negative() {
                    Err(ShellError::NeedsPositiveValue(*span))
                } else {
                    Ok(CellPath {
                        members: vec![PathMember::Int {
                            val: *val as usize,
                            span: *span,
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
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        match v {
            SpannedValue::Bool { val, .. } => Ok(*val),
            v => Err(ShellError::CantConvert {
                to_type: "bool".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for Spanned<bool> {
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        match v {
            SpannedValue::Bool { val, span } => Ok(Spanned {
                item: *val,
                span: *span,
            }),
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
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        match v {
            SpannedValue::Date { val, .. } => Ok(*val),
            v => Err(ShellError::CantConvert {
                to_type: "date".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for Spanned<DateTime<FixedOffset>> {
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        match v {
            SpannedValue::Date { val, span } => Ok(Spanned {
                item: *val,
                span: *span,
            }),
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
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        match v {
            SpannedValue::Range { val, .. } => Ok((**val).clone()),
            v => Err(ShellError::CantConvert {
                to_type: "range".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for Spanned<Range> {
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        match v {
            SpannedValue::Range { val, span } => Ok(Spanned {
                item: (**val).clone(),
                span: *span,
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

impl FromValue for Vec<u8> {
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        match v {
            SpannedValue::Binary { val, .. } => Ok(val.clone()),
            SpannedValue::String { val, .. } => Ok(val.bytes().collect()),
            v => Err(ShellError::CantConvert {
                to_type: "binary data".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for Spanned<Vec<u8>> {
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        match v {
            SpannedValue::Binary { val, span } => Ok(Spanned {
                item: val.clone(),
                span: *span,
            }),
            SpannedValue::String { val, span } => Ok(Spanned {
                item: val.bytes().collect(),
                span: *span,
            }),
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
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        match v {
            SpannedValue::String { val, span } => Ok(Spanned {
                item: PathBuf::from_str(val)
                    .map_err(|err| ShellError::FileNotFoundCustom(err.to_string(), *span))?,
                span: *span,
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

impl FromValue for Vec<SpannedValue> {
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        // FIXME: we may want to fail a little nicer here
        match v {
            SpannedValue::List { vals, .. } => Ok(vals.clone()),
            v => Err(ShellError::CantConvert {
                to_type: "Vector of values".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

// A record
impl FromValue for (Vec<String>, Vec<SpannedValue>) {
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        match v {
            SpannedValue::Record { cols, vals, .. } => Ok((cols.clone(), vals.clone())),
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
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        match v {
            SpannedValue::Closure { val, captures, .. } => Ok(Closure {
                block_id: *val,
                captures: captures.clone(),
            }),
            SpannedValue::Block { val, .. } => Ok(Closure {
                block_id: *val,
                captures: HashMap::new(),
            }),
            v => Err(ShellError::CantConvert {
                to_type: "Closure".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for Block {
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        match v {
            SpannedValue::Block { val, .. } => Ok(Block { block_id: *val }),
            v => Err(ShellError::CantConvert {
                to_type: "Block".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for Spanned<Closure> {
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        match v {
            SpannedValue::Closure {
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
            v => Err(ShellError::CantConvert {
                to_type: "Closure".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for Spanned<MatchPattern> {
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        match v {
            SpannedValue::MatchPattern { val, span } => Ok(Spanned {
                item: *val.clone(),
                span: *span,
            }),
            v => Err(ShellError::CantConvert {
                to_type: "Match pattern".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

impl FromValue for MatchPattern {
    fn from_value(v: &SpannedValue) -> Result<Self, ShellError> {
        match v {
            SpannedValue::MatchPattern { val, .. } => Ok(*val.clone()),
            v => Err(ShellError::CantConvert {
                to_type: "Match pattern".into(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}
