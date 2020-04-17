use crate::data::value;
use nu_protocol::hir::Operator;
use nu_protocol::{Primitive, ShellTypeName, UntaggedValue, Value};
use std::ops::Not;

pub fn apply_operator(
    op: Operator,
    left: &Value,
    right: &Value,
) -> Result<UntaggedValue, (&'static str, &'static str)> {
    match op {
        Operator::Equal
        | Operator::NotEqual
        | Operator::LessThan
        | Operator::GreaterThan
        | Operator::LessThanOrEqual
        | Operator::GreaterThanOrEqual => {
            value::compare_values(op, left, right).map(UntaggedValue::boolean)
        }
        Operator::Contains => contains(left, right).map(UntaggedValue::boolean),
        Operator::NotContains => contains(left, right)
            .map(Not::not)
            .map(UntaggedValue::boolean),
        Operator::Plus => value::compute_values(op, left, right),
        Operator::Minus => value::compute_values(op, left, right),
        Operator::Multiply => value::compute_values(op, left, right),
        Operator::Divide => value::compute_values(op, left, right),
    }
}

fn contains(
    left: &UntaggedValue,
    right: &UntaggedValue,
) -> Result<bool, (&'static str, &'static str)> {
    match (left, right) {
        (
            UntaggedValue::Primitive(Primitive::String(l)),
            UntaggedValue::Primitive(Primitive::String(r)),
        ) => Ok(l.contains(r)),
        (
            UntaggedValue::Primitive(Primitive::Line(l)),
            UntaggedValue::Primitive(Primitive::String(r)),
        ) => Ok(l.contains(r)),
        (
            UntaggedValue::Primitive(Primitive::String(l)),
            UntaggedValue::Primitive(Primitive::Line(r)),
        ) => Ok(l.contains(r)),
        (
            UntaggedValue::Primitive(Primitive::Line(l)),
            UntaggedValue::Primitive(Primitive::Line(r)),
        ) => Ok(l.contains(r)),
        _ => Err((left.type_name(), right.type_name())),
    }
}
