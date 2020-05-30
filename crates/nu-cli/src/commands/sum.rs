use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use crate::utils::data_processing::{reducer_for, Reduce};
use nu_errors::ShellError;
use nu_protocol::{Dictionary, ReturnSuccess, ReturnValue, Signature, UntaggedValue, Value};
use num_traits::identities::Zero;

use indexmap::map::IndexMap;

pub struct Sum;

#[async_trait]
impl WholeStreamCommand for Sum {
    fn name(&self) -> &str {
        "sum"
    }

    fn signature(&self) -> Signature {
        Signature::build("sum")
    }

    fn usage(&self) -> &str {
        "Sums the values."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        sum(RunnableContext {
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
        vec![
            Example {
                description: "Sum a list of numbers",
                example: "echo [1 2 3] | sum",
                result: Some(vec![UntaggedValue::int(6).into()]),
            },
            Example {
                description: "Get the disk usage for the current directory",
                example: "ls --all --du | get size | sum",
                result: None,
            },
        ]
    }
}

fn sum(RunnableContext { mut input, .. }: RunnableContext) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let mut values: Vec<Value> = input.drain_vec().await;
        let action = reducer_for(Reduce::Sum);

        if values.iter().all(|v| if let UntaggedValue::Primitive(_) = v.value {true} else {false}) {
            let total = action(Value::zero(), values)?;
            yield ReturnSuccess::value(total)
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
                let sum = action(Value::zero(), col_vals);
                match sum {
                    Ok(value) => {
                        column_totals.insert(col_name, value);
                    },
                    Err(err) => yield Err(err),
                };
            }
            yield ReturnSuccess::value(
                UntaggedValue::Row(Dictionary {entries: column_totals}).into_untagged_value())
        }
    };

    let stream: BoxStream<'static, ReturnValue> = stream.boxed();

    Ok(stream.to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Sum;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Sum {})
    }
}
