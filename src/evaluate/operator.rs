use crate::data::value;
use nu_parser::Operator;
use nu_protocol::{Primitive, ShellTypeName, UntaggedValue, Value};
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
        | Operator::GreaterThanOrEqual => {
            value::compare_values(op, left, right).map(value::boolean)
        }
        Operator::Dot => Ok(value::boolean(false)),
        Operator::Contains => contains(left, right).map(value::boolean),
        Operator::NotContains => contains(left, right).map(Not::not).map(value::boolean),
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
