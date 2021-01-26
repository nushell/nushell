use nu_data::{value, value::compare_values};
use nu_errors::ShellError;
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
        Operator::Contains => string_contains(left, right).map(UntaggedValue::boolean),
        Operator::NotContains => string_contains(left, right)
            .map(Not::not)
            .map(UntaggedValue::boolean),
        Operator::Plus => value::compute_values(op, left, right),
        Operator::Minus => value::compute_values(op, left, right),
        Operator::Multiply => value::compute_values(op, left, right),
        Operator::Pow => value::compute_values(op, left, right),
        Operator::Divide => value::compute_values(op, left, right).map(|res| match res {
            UntaggedValue::Error(_) => UntaggedValue::Error(ShellError::labeled_error(
                "Evaluation error",
                "division by zero",
                &right.tag.span,
            )),
            _ => res,
        }),
        Operator::Modulo => value::compute_values(op, left, right).map(|res| match res {
            UntaggedValue::Error(_) => UntaggedValue::Error(ShellError::labeled_error(
                "Evaluation error",
                "division by zero",
                &right.tag.span,
            )),
            _ => res,
        }),
        Operator::In => table_contains(left, right).map(UntaggedValue::boolean),
        Operator::NotIn => table_contains(left, right).map(|x| UntaggedValue::boolean(!x)),
        Operator::And => match (left.as_bool(), right.as_bool()) {
            (Ok(left), Ok(right)) => Ok(UntaggedValue::boolean(left && right)),
            _ => Err((left.type_name(), right.type_name())),
        },
        Operator::Or => match (left.as_bool(), right.as_bool()) {
            (Ok(left), Ok(right)) => Ok(UntaggedValue::boolean(left || right)),
            _ => Err((left.type_name(), right.type_name())),
        },
    }
}

fn string_contains(
    left: &UntaggedValue,
    right: &UntaggedValue,
) -> Result<bool, (&'static str, &'static str)> {
    match (left, right) {
        (
            UntaggedValue::Primitive(Primitive::String(l)),
            UntaggedValue::Primitive(Primitive::String(r)),
        ) => Ok(l.contains(r)),
        (
            UntaggedValue::Primitive(Primitive::FilePath(l)),
            UntaggedValue::Primitive(Primitive::String(r)),
        ) => Ok(l.as_path().display().to_string().contains(r)),
        (
            UntaggedValue::Primitive(Primitive::String(l)),
            UntaggedValue::Primitive(Primitive::FilePath(r)),
        ) => Ok(l.contains(&r.as_path().display().to_string())),
        _ => Err((left.type_name(), right.type_name())),
    }
}

fn table_contains(
    left: &UntaggedValue,
    right: &UntaggedValue,
) -> Result<bool, (&'static str, &'static str)> {
    match right {
        UntaggedValue::Table(values) => {
            Ok(values
                .iter()
                .any(|x| match compare_values(Operator::Equal, &left, &x.value) {
                    Ok(coerced) => coerced,
                    _ => false,
                }))
        }
        _ => Err((left.type_name(), right.type_name())),
    }
}
