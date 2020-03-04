use crate::data::value;
use nu_parser::CompareOperator;
use nu_protocol::{Primitive, ShellTypeName, UntaggedValue, Value};
use std::ops::Not;

pub fn apply_operator(
    op: CompareOperator,
    left: &Value,
    right: &Value,
) -> Result<UntaggedValue, (&'static str, &'static str)> {
    match op {
        CompareOperator::Equal
        | CompareOperator::NotEqual
        | CompareOperator::LessThan
        | CompareOperator::GreaterThan
        | CompareOperator::LessThanOrEqual
        | CompareOperator::GreaterThanOrEqual => {
            value::compare_values(op, left, right).map(UntaggedValue::boolean)
        }
        CompareOperator::Contains => contains(left, right).map(UntaggedValue::boolean),
        CompareOperator::NotContains => contains(left, right)
            .map(Not::not)
            .map(UntaggedValue::boolean),
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
