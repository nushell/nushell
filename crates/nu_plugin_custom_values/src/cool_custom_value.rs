#![allow(clippy::result_large_err)]
use nu_protocol::{
    CustomValue, ShellError, Span, Type, Value,
    ast::{self, Math, Operator},
    casing::Casing,
};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CoolCustomValue {
    pub(crate) cool: String,
}

impl CoolCustomValue {
    pub fn new(content: &str) -> Self {
        Self {
            cool: content.to_owned(),
        }
    }

    pub fn into_value(self, span: Span) -> Value {
        Value::custom(Box::new(self), span)
    }

    pub fn try_from_value(value: &Value) -> Result<Self, ShellError> {
        let span = value.span();
        match value {
            Value::Custom { val, .. } => {
                if let Some(cool) = val.as_any().downcast_ref::<Self>() {
                    Ok(cool.clone())
                } else {
                    Err(ShellError::CantConvert {
                        to_type: "cool".into(),
                        from_type: "non-cool".into(),
                        span,
                        help: None,
                    })
                }
            }
            x => Err(ShellError::CantConvert {
                to_type: "cool".into(),
                from_type: x.get_type().to_string(),
                span,
                help: None,
            }),
        }
    }
}

#[typetag::serde]
impl CustomValue for CoolCustomValue {
    fn clone_value(&self, span: Span) -> Value {
        Value::custom(Box::new(self.clone()), span)
    }

    fn type_name(&self) -> String {
        self.typetag_name().to_string()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(Value::string(
            format!("I used to be a custom value! My data was ({})", self.cool),
            span,
        ))
    }

    fn follow_path_int(
        &self,
        _self_span: Span,
        index: usize,
        path_span: Span,
        optional: bool,
    ) -> Result<Value, ShellError> {
        match (index, optional) {
            (0, _) => Ok(Value::string(&self.cool, path_span)),
            (_, true) => Ok(Value::nothing(path_span)),
            _ => Err(ShellError::AccessBeyondEnd {
                max_idx: 0,
                span: path_span,
            }),
        }
    }

    fn follow_path_string(
        &self,
        self_span: Span,
        column_name: String,
        path_span: Span,
        optional: bool,
        casing: Casing,
    ) -> Result<Value, ShellError> {
        let column_name = match casing {
            Casing::Sensitive => column_name,
            Casing::Insensitive => column_name.to_lowercase(),
        };

        match (column_name.as_str(), optional) {
            ("cool", _) => Ok(Value::string(&self.cool, path_span)),
            (_, true) => Ok(Value::nothing(path_span)),
            _ => Err(ShellError::CantFindColumn {
                col_name: column_name,
                span: Some(path_span),
                src_span: self_span,
            }),
        }
    }

    fn partial_cmp(&self, other: &Value) -> Option<Ordering> {
        if let Value::Custom { val, .. } = other {
            val.as_any()
                .downcast_ref()
                .and_then(|other: &CoolCustomValue| PartialOrd::partial_cmp(self, other))
        } else {
            None
        }
    }

    fn operation(
        &self,
        lhs_span: Span,
        operator: ast::Operator,
        op_span: Span,
        right: &Value,
    ) -> Result<Value, ShellError> {
        match operator {
            // Append the string inside `cool`
            Operator::Math(Math::Concatenate) => {
                if let Some(right) = right
                    .as_custom_value()
                    .ok()
                    .and_then(|c| c.as_any().downcast_ref::<CoolCustomValue>())
                {
                    Ok(Value::custom(
                        Box::new(CoolCustomValue {
                            cool: format!("{}{}", self.cool, right.cool),
                        }),
                        op_span,
                    ))
                } else {
                    Err(ShellError::OperatorUnsupportedType {
                        op: Operator::Math(Math::Concatenate),
                        unsupported: right.get_type(),
                        op_span,
                        unsupported_span: right.span(),
                        help: None,
                    })
                }
            }
            _ => Err(ShellError::OperatorUnsupportedType {
                op: Operator::Math(Math::Concatenate),
                unsupported: Type::Custom(self.type_name().into()),
                op_span,
                unsupported_span: lhs_span,
                help: None,
            }),
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
