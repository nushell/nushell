use crate::prelude::*;

use crate::commands::math::reducers::{reducer_for, Reduce};
use crate::commands::math::utils::run_with_function;
use nu_engine::WholeStreamCommand;

use nu_errors::ShellError;
use nu_protocol::{hir::Operator, Primitive, Signature, UntaggedValue, Value};

use bigdecimal::FromPrimitive;

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "math avg"
    }

    fn signature(&self) -> Signature {
        Signature::build("math avg")
    }

    fn usage(&self) -> &str {
        "Finds the average of a list of numbers or tables"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run_with_function(args, average)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get the average of a list of numbers",
            example: "echo [-50 100.0 25] | math avg",
            result: Some(vec![UntaggedValue::decimal_from_float(
                25.0,
                Span::unknown(),
            )
            .into()]),
        }]
    }
}

fn to_byte(value: &Value) -> Option<Value> {
    match &value.value {
        UntaggedValue::Primitive(Primitive::Int(num)) => {
            Some(UntaggedValue::Primitive(Primitive::Filesize(*num as u64)).into_untagged_value())
        }
        _ => None,
    }
}

pub fn average(values: &[Value], name: &Tag) -> Result<Value, ShellError> {
    let sum = reducer_for(Reduce::Summation);

    let number = BigDecimal::from_usize(values.len()).ok_or_else(|| {
        ShellError::labeled_error("nothing to average", "nothing to average", &name.span)
    })?;

    let total_rows = UntaggedValue::decimal(number);

    let are_bytes = values
        .get(0)
        .ok_or_else(|| {
            ShellError::unexpected("Cannot perform aggregate math operation on empty data")
        })?
        .is_filesize();

    let total = if are_bytes {
        to_byte(&sum(
            UntaggedValue::int(0).into_untagged_value(),
            values
                .to_vec()
                .iter()
                .map(|v| match v {
                    Value {
                        value: UntaggedValue::Primitive(Primitive::Filesize(num)),
                        ..
                    } => UntaggedValue::int(*num as i64).into_untagged_value(),
                    other => other.clone(),
                })
                .collect::<Vec<_>>(),
        )?)
        .ok_or_else(|| {
            ShellError::labeled_error(
                "could not convert to big decimal",
                "could not convert to big decimal",
                &name.span,
            )
        })
    } else {
        sum(UntaggedValue::int(0).into_untagged_value(), values.to_vec())
    }?;

    match total {
        Value {
            value: UntaggedValue::Primitive(Primitive::Filesize(num)),
            ..
        } => {
            let left = UntaggedValue::from(Primitive::Int(num as i64));
            let result = nu_data::value::compute_values(Operator::Divide, &left, &total_rows);

            match result {
                Ok(UntaggedValue::Primitive(Primitive::Decimal(result))) => match result.to_u64() {
                    Some(number) => Ok(UntaggedValue::filesize(number).into_value(name)),
                    None => Err(ShellError::labeled_error(
                        "could not calculate average of non-integer or unrelated types",
                        "source",
                        name,
                    )),
                },
                Ok(_) => Err(ShellError::labeled_error(
                    "could not calculate average of non-integer or unrelated types",
                    "source",
                    name,
                )),
                Err((left_type, right_type)) => Err(ShellError::coerce_error(
                    left_type.spanned(name.span),
                    right_type.spanned(name.span),
                )),
            }
        }
        Value {
            value: UntaggedValue::Primitive(Primitive::Duration(duration)),
            ..
        } => {
            let left = UntaggedValue::from(Primitive::Duration(duration));
            let result = nu_data::value::compute_values(Operator::Divide, &left, &total_rows);

            match result {
                Ok(UntaggedValue::Primitive(Primitive::Duration(result))) => {
                    Ok(UntaggedValue::duration(result).into_value(name))
                }
                Ok(_) => Err(ShellError::labeled_error(
                    "could not calculate average of non-integer or unrelated types",
                    "source",
                    name,
                )),
                Err((left_type, right_type)) => Err(ShellError::coerce_error(
                    left_type.spanned(name.span),
                    right_type.spanned(name.span),
                )),
            }
        }
        Value {
            value: UntaggedValue::Primitive(other),
            ..
        } => {
            let left = UntaggedValue::from(other);
            let result = nu_data::value::compute_values(Operator::Divide, &left, &total_rows);

            match result {
                Ok(value) => Ok(value.into_value(name)),
                Err((left_type, right_type)) => Err(ShellError::coerce_error(
                    left_type.spanned(name.span),
                    right_type.spanned(name.span),
                )),
            }
        }
        _ => Err(ShellError::labeled_error(
            "could not calculate average of non-integer or unrelated types",
            "source",
            name,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::SubCommand;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }
}
