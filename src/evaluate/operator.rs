use crate::data::Primitive;
use crate::data::Value;
use crate::parser::Operator;
use crate::traits::ShellTypeName;
use std::ops::Not;

pub fn apply_operator(
    op: &Operator,
    left: &Value,
    right: &Value,
) -> Result<Value, (&'static str, &'static str)> {
    match *op {
        Operator::Equal
        | Operator::NotEqual
        | Operator::LessThan
        | Operator::GreaterThan
        | Operator::LessThanOrEqual
        | Operator::GreaterThanOrEqual => left.compare(op, right).map(Value::boolean),
        Operator::Dot => Ok(Value::boolean(false)),
        Operator::Contains => contains(left, right).map(Value::boolean),
        Operator::NotContains => contains(left, right).map(Not::not).map(Value::boolean),
    }
}

fn contains(left: &Value, right: &Value) -> Result<bool, (&'static str, &'static str)> {
    if let (Value::Primitive(Primitive::String(l)), Value::Primitive(Primitive::String(r))) =
        (left, right)
    {
        Ok(l.contains(r))
    } else {
        Err((left.type_name(), right.type_name()))
    }
}
