use crate::commands::math::reducers::{reducer_for, Reduce};
use crate::commands::math::utils::run_with_function;
use crate::prelude::*;
use bigdecimal::FromPrimitive;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    hir::{convert_number_to_u64, Number, Operator},
    Primitive, Signature, UntaggedValue, Value,
};

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "math median"
    }

    fn signature(&self) -> Signature {
        Signature::build("math median")
    }

    fn usage(&self) -> &str {
        "Gets the median of a list of numbers"
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
            median,
        )
        .await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get the median of a list of numbers",
            example: "echo [3 8 9 12 12 15] | math median",
            result: Some(vec![UntaggedValue::decimal_from_float(
                10.5,
                Span::unknown(),
            )
            .into()]),
        }]
    }
}

enum Pick {
    MedianAverage,
    Median,
}

pub fn median(values: &[Value], name: &Tag) -> Result<Value, ShellError> {
    let take = if values.len() % 2 == 0 {
        Pick::MedianAverage
    } else {
        Pick::Median
    };

    let mut sorted = vec![];

    for item in values {
        sorted.push(item.clone());
    }

    crate::commands::sort_by::sort(&mut sorted, &[], name, false)?;

    match take {
        Pick::Median => {
            let idx = (values.len() as f64 / 2.0).floor() as usize;
            let out = sorted.get(idx).ok_or_else(|| {
                ShellError::labeled_error(
                    "could not extract value",
                    "could not extract value",
                    &name.span,
                )
            })?;
            Ok(out.clone())
        }
        Pick::MedianAverage => {
            let idx_end = (values.len() / 2) as usize;
            let idx_start = idx_end - 1;

            let left = sorted
                .get(idx_start)
                .ok_or_else(|| {
                    ShellError::labeled_error(
                        "could not extract value",
                        "could not extract value",
                        &name.span,
                    )
                })?
                .clone();

            let right = sorted
                .get(idx_end)
                .ok_or_else(|| {
                    ShellError::labeled_error(
                        "could not extract value",
                        "could not extract value",
                        &name.span,
                    )
                })?
                .clone();

            compute_average(&[left, right], name)
        }
    }
}

fn compute_average(values: &[Value], name: impl Into<Tag>) -> Result<Value, ShellError> {
    let name = name.into();

    let sum = reducer_for(Reduce::Summation);
    let number = BigDecimal::from_usize(2).ok_or_else(|| {
        ShellError::labeled_error(
            "could not convert to big decimal",
            "could not convert to big decimal",
            &name,
        )
    })?;

    let total_rows = UntaggedValue::decimal(number);
    let total = sum(Value::nothing(), values.to_vec())?;

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
                    "could not calculate median of non-numeric or unrelated types",
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
            "could not calculate median of non-numeric or unrelated types",
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
