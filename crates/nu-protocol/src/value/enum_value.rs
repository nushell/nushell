use crate::{
    CustomValue, Record, ShellError, Span, Value,
    ast::{Comparison, Operator},
    casing::Casing,
};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::cmp::Ordering;

/// A value of a user-declared `enum` type (`type Shape = enum<...>`).
///
/// Enum values are nominal: their [`Value::get_type`] is
/// `Type::Custom("<enum name>")`, which only compares equal to the same enum.
///
/// For display, serialization, cell-path access, and `match` destructuring an
/// enum value lowers to its [`base_record`](Self::base_record): the variant
/// name under `kind` and the payload — when there is one — under `payload`.
/// The encoding is uniform: a record payload stays a nested record, so the
/// base record is always `{kind, payload?}` and `match` can rely on the
/// `payload` key existing for every payload-carrying variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumValue {
    /// The name of the declared enum type, e.g. `Shape`.
    pub enum_name: String,
    /// The variant name, e.g. `circle`.
    pub variant: String,
    /// The variant payload, or `None` for unit variants.
    pub payload: Option<Box<Value>>,
}

impl EnumValue {
    pub fn new(
        enum_name: impl Into<String>,
        variant: impl Into<String>,
        payload: Option<Value>,
    ) -> Self {
        EnumValue {
            enum_name: enum_name.into(),
            variant: variant.into(),
            payload: payload.map(Box::new),
        }
    }

    /// The record this value lowers to: `{kind: "<variant>", payload:
    /// <value>}`, with no `payload` key for unit variants. Payloads are
    /// always nested under `payload` — never spread — so payload records
    /// may freely use `kind` or `payload` as field names.
    pub fn base_record(&self, span: Span) -> Record {
        let mut record = Record::new();
        record.push("kind", Value::string(self.variant.clone(), span));

        if let Some(payload) = &self.payload {
            record.push("payload", (**payload).clone());
        }

        record
    }
}

#[typetag::serde]
impl CustomValue for EnumValue {
    fn clone_value(&self, span: Span) -> Value {
        Value::custom(Box::new(self.clone()), span)
    }

    fn type_name(&self) -> String {
        self.enum_name.clone()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(Value::record(self.base_record(span), span))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }

    fn follow_path_string(
        &self,
        self_span: Span,
        column_name: String,
        path_span: Span,
        optional: bool,
        casing: Casing,
    ) -> Result<Value, ShellError> {
        let base = self.to_base_value(self_span)?;
        let result = base.follow_cell_path(&[crate::ast::PathMember::string(
            column_name,
            optional,
            casing,
            path_span,
        )])?;
        Ok(result.into_owned())
    }

    fn partial_cmp(&self, other: &Value) -> Option<Ordering> {
        let Value::Custom { val, .. } = other else {
            return None;
        };
        let other = val.as_any().downcast_ref::<EnumValue>()?;

        match self
            .enum_name
            .cmp(&other.enum_name)
            .then_with(|| self.variant.cmp(&other.variant))
        {
            Ordering::Equal => match (&self.payload, &other.payload) {
                (None, None) => Some(Ordering::Equal),
                (Some(lhs), Some(rhs)) => lhs.partial_cmp(rhs),
                (None, Some(_)) => Some(Ordering::Less),
                (Some(_), None) => Some(Ordering::Greater),
            },
            ord => Some(ord),
        }
    }

    fn operation(
        &self,
        lhs_span: Span,
        operator: Operator,
        op: Span,
        right: &Value,
    ) -> Result<Value, ShellError> {
        match operator {
            Operator::Comparison(Comparison::Equal) => Ok(Value::bool(
                matches!(self.partial_cmp(right), Some(Ordering::Equal)),
                op,
            )),
            Operator::Comparison(Comparison::NotEqual) => Ok(Value::bool(
                !matches!(self.partial_cmp(right), Some(Ordering::Equal)),
                op,
            )),
            _ => Err(ShellError::OperatorUnsupportedType {
                op: operator,
                unsupported: crate::Type::Custom(self.enum_name.clone().into()),
                op_span: op,
                unsupported_span: lhs_span,
                help: None,
            }),
        }
    }
}
