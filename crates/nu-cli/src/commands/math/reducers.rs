use crate::data::value::{compare_values, compute_values};
use nu_errors::ShellError;
use nu_protocol::hir::Operator;
use nu_protocol::{UntaggedValue, Value};
use nu_source::{SpannedItem, Tag};

// Re-usable error messages
const ERR_EMPTY_DATA: &str = "Cannot perform aggregate math operation on empty data";

fn formula(
    acc_begin: Value,
    calculator: Box<dyn Fn(Vec<Value>) -> Result<Value, ShellError> + Send + Sync + 'static>,
) -> Box<dyn Fn(Value, Vec<Value>) -> Result<Value, ShellError> + Send + Sync + 'static> {
    Box::new(move |acc, datax| -> Result<Value, ShellError> {
        let result = match compute_values(Operator::Multiply, &acc, &acc_begin) {
            Ok(v) => v.into_untagged_value(),
            Err((left_type, right_type)) => {
                return Err(ShellError::coerce_error(
                    left_type.spanned_unknown(),
                    right_type.spanned_unknown(),
                ))
            }
        };

        match calculator(datax) {
            Ok(total) => Ok(match compute_values(Operator::Plus, &result, &total) {
                Ok(v) => v.into_untagged_value(),
                Err((left_type, right_type)) => {
                    return Err(ShellError::coerce_error(
                        left_type.spanned_unknown(),
                        right_type.spanned_unknown(),
                    ))
                }
            }),
            Err(reason) => Err(reason),
        }
    })
}

pub fn reducer_for(
    command: Reduce,
) -> Box<dyn Fn(Value, Vec<Value>) -> Result<Value, ShellError> + Send + Sync + 'static> {
    match command {
        Reduce::Summation | Reduce::Default => Box::new(formula(
            UntaggedValue::int(0).into_untagged_value(),
            Box::new(sum),
        )),
        Reduce::Minimum => Box::new(|_, values| min(values)),
        Reduce::Maximum => Box::new(|_, values| max(values)),
    }
}

pub enum Reduce {
    Summation,
    Minimum,
    Maximum,
    Default,
}

pub fn sum(data: Vec<Value>) -> Result<Value, ShellError> {
    let mut acc = UntaggedValue::int(0).into_untagged_value();
    for value in data {
        match value.value {
            UntaggedValue::Primitive(_) => {
                acc = match compute_values(Operator::Plus, &acc, &value) {
                    Ok(v) => v.into_untagged_value(),
                    Err((left_type, right_type)) => {
                        return Err(ShellError::coerce_error(
                            left_type.spanned_unknown(),
                            right_type.spanned_unknown(),
                        ))
                    }
                };
            }
            _ => {
                return Err(ShellError::labeled_error(
                    "Attempted to compute the sum of a value that cannot be summed.",
                    "value appears here",
                    value.tag.span,
                ))
            }
        }
    }
    Ok(acc)
}

pub fn max(data: Vec<Value>) -> Result<Value, ShellError> {
    let mut biggest = data
        .first()
        .ok_or_else(|| ShellError::unexpected(ERR_EMPTY_DATA))?
        .value
        .clone();

    for value in data.iter() {
        if let Ok(greater_than) = compare_values(Operator::GreaterThan, &value.value, &biggest) {
            if greater_than {
                biggest = value.value.clone();
            }
        } else {
            return Err(ShellError::unexpected(format!(
                "Could not compare\nleft: {:?}\nright: {:?}",
                biggest, value.value
            )));
        }
    }
    Ok(Value {
        value: biggest,
        tag: Tag::unknown(),
    })
}

pub fn min(data: Vec<Value>) -> Result<Value, ShellError> {
    let mut smallest = data
        .first()
        .ok_or_else(|| ShellError::unexpected(ERR_EMPTY_DATA))?
        .value
        .clone();

    for value in data.iter() {
        if let Ok(greater_than) = compare_values(Operator::LessThan, &value.value, &smallest) {
            if greater_than {
                smallest = value.value.clone();
            }
        } else {
            return Err(ShellError::unexpected(format!(
                "Could not compare\nleft: {:?}\nright: {:?}",
                smallest, value.value
            )));
        }
    }
    Ok(Value {
        value: smallest,
        tag: Tag::unknown(),
    })
}
