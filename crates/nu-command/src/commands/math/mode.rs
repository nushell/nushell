use crate::commands::math::utils::run_with_function;
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue, Value};
use std::cmp::Ordering;

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "math mode"
    }

    fn signature(&self) -> Signature {
        Signature::build("math mode")
    }

    fn usage(&self) -> &str {
        "Gets the most frequent element(s) from a list of numbers or tables"
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
            mode,
        )
        .await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get the mode(s) of a list of numbers",
            example: "echo [3 3 9 12 12 15] | math mode",
            result: Some(vec![
                UntaggedValue::int(3).into_untagged_value(),
                UntaggedValue::int(12).into_untagged_value(),
            ]),
        }]
    }
}

pub fn mode(values: &[Value], name: &Tag) -> Result<Value, ShellError> {
    let mut frequency_map = std::collections::HashMap::new();
    for v in values {
        let counter = frequency_map.entry(v.value.clone()).or_insert(0);
        *counter += 1;
    }

    let mut max_freq = -1;
    let mut modes = Vec::<Value>::new();
    for (value, frequency) in frequency_map.iter() {
        match max_freq.cmp(&frequency) {
            Ordering::Less => {
                max_freq = *frequency;
                modes.clear();
                modes.push(value.clone().into_value(name));
            }
            Ordering::Equal => {
                modes.push(value.clone().into_value(name));
            }
            Ordering::Greater => (),
        }
    }

    crate::commands::sort_by::sort(&mut modes, &[], name, false)?;
    Ok(UntaggedValue::Table(modes).into_value(name))
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
