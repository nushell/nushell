use crate::prelude::*;

use crate::commands::math::reducers::{reducer_for, Reduce};
use crate::commands::math::utils::run_with_function;
use nu_engine::WholeStreamCommand;

use nu_errors::ShellError;
use nu_protocol::{
    hir::{convert_number_to_u64, Number, Operator},
    Primitive, Signature, UntaggedValue, Value,
};

use bigdecimal::FromPrimitive;

pub struct SubCommand;

#[async_trait]
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run_with_function(
            RunnableContext {
                input: args.input,
                scope: args.scope.clone(),
                shell_manager: args.shell_manager,
                host: args.host,
                ctrl_c: args.ctrl_c,
                current_errors: args.current_errors,
                name: args.call_info.name_tag,
            },
            average,
        )
        .await
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
            Some(UntaggedValue::Primitive(Primitive::Filesize(num.clone())).into_untagged_value())
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
                    } => UntaggedValue::int(num.clone()).into_untagged_value(),
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
            let left = UntaggedValue::from(Primitive::Int(num));
            let result = nu_data::value::compute_values(Operator::Divide, &left, &total_rows);

            match result {
                Ok(UntaggedValue::Primitive(Primitive::Decimal(result))) => {
                    let number = Number::Decimal(result);
                    let number = convert_number_to_u64(&number);
                    Ok(UntaggedValue::filesize(number).into_value(name))
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
