use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use crate::utils::data_processing::{reducer_for, Reduce};
use bigdecimal::FromPrimitive;
use nu_errors::ShellError;
use nu_protocol::hir::{convert_number_to_u64, Number, Operator};
use nu_protocol::{
    Dictionary, Primitive, ReturnSuccess, ReturnValue, Signature, UntaggedValue, Value,
};
use num_traits::identities::Zero;

use indexmap::map::IndexMap;

pub struct Average;

#[async_trait]
impl WholeStreamCommand for Average {
    fn name(&self) -> &str {
        "average"
    }

    fn signature(&self) -> Signature {
        Signature::build("average")
    }

    fn usage(&self) -> &str {
        "Average the values."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        average(RunnableContext {
            input: args.input,
            registry: registry.clone(),
            shell_manager: args.shell_manager,
            host: args.host,
            ctrl_c: args.ctrl_c,
            current_errors: args.current_errors,
            name: args.call_info.name_tag,
            raw_input: args.raw_input,
        })
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Average a list of numbers",
            example: "echo [100 0 100 0] | average",
            result: Some(vec![UntaggedValue::decimal(50).into()]),
        }]
    }
}

fn average(
    RunnableContext {
        mut input, name, ..
    }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let mut values: Vec<Value> = input.drain_vec().await;
        let action = reducer_for(Reduce::Sum);

        if values.iter().all(|v| if let UntaggedValue::Primitive(_) = v.value {true} else {false}) {
            match avg(&values, name) {
                Ok(result) => yield ReturnSuccess::value(result),
                Err(err) => yield Err(err),
            }
        } else {
            let mut column_values = IndexMap::new();
            for value in values {
                match value.value {
                    UntaggedValue::Row(row_dict) => {
                        for (key, value) in row_dict.entries.iter() {
                            column_values
                                .entry(key.clone())
                                .and_modify(|v: &mut Vec<Value>| v.push(value.clone()))
                                .or_insert(vec![value.clone()]);
                        }
                    },
                    table => {},
                };
            }

            let mut column_totals = IndexMap::new();
            for (col_name, col_vals) in column_values {
                match avg(&col_vals, &name) {
                    Ok(result) => {
                        column_totals.insert(col_name, result);
                    }
                    Err(err) => yield Err(err),
                }
            }
            yield ReturnSuccess::value(
                UntaggedValue::Row(Dictionary {entries: column_totals}).into_untagged_value())
        }
    };

    let stream: BoxStream<'static, ReturnValue> = stream.boxed();

    Ok(stream.to_output_stream())
}

fn avg(values: &[Value], name: impl Into<Tag>) -> Result<Value, ShellError> {
    let name = name.into();

    let sum = reducer_for(Reduce::Sum);

    let number = BigDecimal::from_usize(values.len()).expect("expected a usize-sized bigdecimal");

    let total_rows = UntaggedValue::decimal(number);
    let total = sum(Value::zero(), values.to_vec())?;

    match total {
        Value {
            value: UntaggedValue::Primitive(Primitive::Bytes(num)),
            ..
        } => {
            let left = UntaggedValue::from(Primitive::Int(num.into()));
            let result = crate::data::value::compute_values(Operator::Divide, &left, &total_rows);

            match result {
                Ok(UntaggedValue::Primitive(Primitive::Decimal(result))) => {
                    let number = Number::Decimal(result);
                    let number = convert_number_to_u64(&number);
                    Ok(UntaggedValue::bytes(number).into_value(name))
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
            let result = crate::data::value::compute_values(Operator::Divide, &left, &total_rows);

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
    use super::Average;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Average {})
    }
}
