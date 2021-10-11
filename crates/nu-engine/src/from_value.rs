// use std::path::PathBuf;

// use nu_path::expand_path;
use nu_protocol::ast::{CellPath, PathMember};
use nu_protocol::ShellError;
use nu_protocol::{Range, Spanned, Value};

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
                // FIXME: error check that this fits
                item: *val as i64,
                span: *span,
            }),
            Value::Duration { val, span } => Ok(Spanned {
                // FIXME: error check that this fits
                item: *val as i64,
                span: *span,
            }),

            v => Err(ShellError::CantConvert("integer".into(), v.span()?)),
        }
    }
}

impl FromValue for i64 {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value::Int { val, .. } => Ok(*val),
            Value::Filesize { val, .. } => Ok(
                // FIXME: error check that this fits
                *val as i64,
            ),
            Value::Duration { val, .. } => Ok(
                // FIXME: error check that this fits
                *val as i64,
            ),

            v => Err(ShellError::CantConvert("integer".into(), v.span()?)),
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
                // FIXME: error check that this fits
                item: *val,
                span: *span,
            }),

            v => Err(ShellError::CantConvert("float".into(), v.span()?)),
        }
    }
}

impl FromValue for f64 {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value::Float { val, .. } => Ok(*val),
            Value::Int { val, .. } => Ok(*val as f64),
            v => Err(ShellError::CantConvert("float".into(), v.span()?)),
        }
    }
}

impl FromValue for String {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        // FIXME: we may want to fail a little nicer here
        Ok(v.clone().into_string())
    }
}

impl FromValue for Spanned<String> {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        Ok(Spanned {
            item: v.clone().into_string(),
            span: v.span()?,
        })
    }
}

//FIXME
/*
impl FromValue for ColumnPath {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value:: => Ok(c.clone()),
            v => Err(ShellError::type_error("column path", v.spanned_type_name())),
        }
    }
}

*/

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
            Value::Int { val, .. } => Ok(CellPath {
                members: vec![PathMember::Int {
                    val: *val as usize,
                    span,
                }],
            }),
            _ => Err(ShellError::CantConvert("cell path".into(), span)),
        }
    }
}

impl FromValue for bool {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value::Bool { val, .. } => Ok(*val),
            v => Err(ShellError::CantConvert("bool".into(), v.span()?)),
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
            v => Err(ShellError::CantConvert("bool".into(), v.span()?)),
        }
    }
}

// impl FromValue for DateTime<FixedOffset> {
//     fn from_value(v: &Value) -> Result<Self, ShellError> {
//         match v {
//             Value {
//                 value: UntaggedValue::Primitive(Primitive::Date(d)),
//                 ..
//             } => Ok(*d),
//             Value {
//                 value: UntaggedValue::Row(_),
//                 ..
//             } => {
//                 let mut shell_error = ShellError::type_error("date", v.spanned_type_name());
//                 shell_error.notes.push(
//                     "Note: you can access columns using dot. eg) $it.column or (ls).column".into(),
//                 );
//                 Err(shell_error)
//             }
//             v => Err(ShellError::type_error("date", v.spanned_type_name())),
//         }
//     }
// }

impl FromValue for Range {
    fn from_value(v: &Value) -> Result<Self, ShellError> {
        match v {
            Value::Range { val, .. } => Ok((**val).clone()),
            v => Err(ShellError::CantConvert("range".into(), v.span()?)),
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
            v => Err(ShellError::CantConvert("range".into(), v.span()?)),
        }
    }
}

// impl FromValue for Vec<u8> {
//     fn from_value(v: &Value) -> Result<Self, ShellError> {
//         match v {
//             Value {
//                 value: UntaggedValue::Primitive(Primitive::Binary(b)),
//                 ..
//             } => Ok(b.clone()),
//             Value {
//                 value: UntaggedValue::Primitive(Primitive::String(s)),
//                 ..
//             } => Ok(s.bytes().collect()),
//             Value {
//                 value: UntaggedValue::Row(_),
//                 ..
//             } => {
//                 let mut shell_error = ShellError::type_error("binary data", v.spanned_type_name());
//                 shell_error.notes.push(
//                     "Note: you can access columns using dot. eg) $it.column or (ls).column".into(),
//                 );
//                 Err(shell_error)
//             }
//             v => Err(ShellError::type_error("binary data", v.spanned_type_name())),
//         }
//     }
// }

// impl FromValue for Dictionary {
//     fn from_value(v: &Value) -> Result<Self, ShellError> {
//         match v {
//             Value {
//                 value: UntaggedValue::Row(r),
//                 ..
//             } => Ok(r.clone()),
//             v => Err(ShellError::type_error("row", v.spanned_type_name())),
//         }
//     }
// }

// impl FromValue for CapturedBlock {
//     fn from_value(v: &Value) -> Result<Self, ShellError> {
//         match v {
//             Value {
//                 value: UntaggedValue::Block(b),
//                 ..
//             } => Ok((**b).clone()),
//             Value {
//                 value: UntaggedValue::Row(_),
//                 ..
//             } => {
//                 let mut shell_error = ShellError::type_error("block", v.spanned_type_name());
//                 shell_error.notes.push(
//                     "Note: you can access columns using dot. eg) $it.column or (ls).column".into(),
//                 );
//                 Err(shell_error)
//             }
//             v => Err(ShellError::type_error("block", v.spanned_type_name())),
//         }
//     }
// }

// impl FromValue for Vec<Value> {
//     fn from_value(v: &Value) -> Result<Self, ShellError> {
//         match v {
//             Value {
//                 value: UntaggedValue::Table(t),
//                 ..
//             } => Ok(t.clone()),
//             Value {
//                 value: UntaggedValue::Row(_),
//                 ..
//             } => Ok(vec![v.clone()]),
//             v => Err(ShellError::type_error("table", v.spanned_type_name())),
//         }
//     }
// }
