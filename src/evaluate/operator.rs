use crate::data::base::{Primitive, UntaggedValue, Value};
use crate::parser::Operator;
use crate::traits::ShellTypeName;
use std::ops::Not;

pub fn apply_operator(
    op: &Operator,
    left: &Value,
    right: &Value,
) -> Result<UntaggedValue, (&'static str, &'static str)> {
    match *op {
        Operator::Equal
        | Operator::NotEqual
        | Operator::LessThan
        | Operator::GreaterThan
        | Operator::LessThanOrEqual
        | Operator::GreaterThanOrEqual => left.compare(op, right).map(UntaggedValue::boolean),
        Operator::Dot => Ok(UntaggedValue::boolean(false)),
        Operator::Contains => contains(left, right).map(UntaggedValue::boolean),
        Operator::NotContains => contains(left, right)
            .map(Not::not)
            .map(UntaggedValue::boolean),
    }
}

fn contains(
    left: &UntaggedValue,
    right: &UntaggedValue,
) -> Result<bool, (&'static str, &'static str)> {
    if let (
        UntaggedValue::Primitive(Primitive::String(l)),
        UntaggedValue::Primitive(Primitive::String(r)),
    ) = (left, right)
    {
        Ok(l.contains(r))
    } else {
        Err((left.type_name(), right.type_name()))
    }
}
