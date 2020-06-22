use crate::commands::math::utils::calculate;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use crate::utils::data_processing::{reducer_for, Reduce};
use nu_errors::ShellError;
use nu_protocol::{Dictionary, Signature, UntaggedValue, Value};
use num_traits::identities::Zero;

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "math sum"
    }

    fn signature(&self) -> Signature {
        Signature::build("math sum")
    }

    fn usage(&self) -> &str {
        "Finds the sum of a list of numbers or tables"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        calculate(
            RunnableContext {
                input: args.input,
                registry: registry.clone(),
                shell_manager: args.shell_manager,
                host: args.host,
                ctrl_c: args.ctrl_c,
                current_errors: args.current_errors,
                name: args.call_info.name_tag,
                raw_input: args.raw_input,
            },
            summation,
        )
        .await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Sum a list of numbers",
                example: "echo [1 2 3] | math sum",
                result: Some(vec![UntaggedValue::int(6).into()]),
            },
            Example {
                description: "Get the disk usage for the current directory",
                example: "ls --all --du | get size | math sum",
                result: None,
            },
        ]
    }
}

pub fn summation(values: &[Value], name: &Tag) -> Result<Value, ShellError> {
    let sum = reducer_for(Reduce::Summation);

    if values.iter().all(|v| v.is_primitive()) {
        Ok(sum(Value::zero(), values.to_vec())?)
    } else {
        let mut column_values = IndexMap::new();

        for value in values {
            if let UntaggedValue::Row(row_dict) = value.value.clone() {
                for (key, value) in row_dict.entries.iter() {
                    column_values
                        .entry(key.clone())
                        .and_modify(|v: &mut Vec<Value>| v.push(value.clone()))
                        .or_insert(vec![value.clone()]);
                }
            };
        }

        let mut column_totals = IndexMap::new();

        for (col_name, col_vals) in column_values {
            let sum = sum(Value::zero(), col_vals)?;

            column_totals.insert(col_name, sum);
        }

        Ok(UntaggedValue::Row(Dictionary {
            entries: column_totals,
        })
        .into_value(name))
    }
}

#[cfg(test)]
mod tests {
    use super::SubCommand;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }
}
