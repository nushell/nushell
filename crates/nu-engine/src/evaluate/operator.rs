use nu_data::{value, value::compare_values};
use nu_errors::ShellError;
use nu_protocol::hir::Operator;
use nu_protocol::{Primitive, ShellTypeName, UntaggedValue, Value};
use std::ops::Not;

#[cfg(feature = "dataframe")]
use nu_protocol::dataframe::{compute_between_dataframes, compute_series_single_value};

pub fn apply_operator(
    op: Operator,
    left: &Value,
    right: &Value,
) -> Result<UntaggedValue, (&'static str, &'static str)> {
    #[cfg(feature = "dataframe")]
    if let (UntaggedValue::DataFrame(_), UntaggedValue::DataFrame(_)) = (&left.value, &right.value)
    {
        return compute_between_dataframes(op, left, right);
    } else if let (UntaggedValue::DataFrame(_), UntaggedValue::Primitive(_)) =
        (&left.value, &right.value)
    {
        return compute_series_single_value(op, left, right);
    }

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
        Operator::In => inside_of(left, right).map(UntaggedValue::boolean),
        Operator::NotIn => inside_of(left, right).map(|x| UntaggedValue::boolean(!x)),
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

fn inside_of(
    left: &UntaggedValue,
    right: &UntaggedValue,
) -> Result<bool, (&'static str, &'static str)> {
    match (left, right) {
        (_, UntaggedValue::Table(values)) => {
            Ok(values
                .iter()
                .any(|x| match compare_values(Operator::Equal, left, &x.value) {
                    Ok(coerced) => coerced,
                    _ => false,
                }))
        }
        (
            UntaggedValue::Primitive(Primitive::String(lhs)),
            UntaggedValue::Primitive(Primitive::String(rhs)),
        ) => Ok(rhs.contains(lhs)),
        _ => Err((left.type_name(), right.type_name())),
    }
}
