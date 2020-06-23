use crate::commands::math::utils::calculate;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
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
            .switch(
                "all",
                "return a list of all modes, if there are multiple",
                Some('a'),
            )
            .switch(
                "min",
                "return the smallest all modes, if there are multiple",
                None,
            )
            .switch(
                "max",
                "return a list of all modes, if there are multiple",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Gets the mode of a list of numbers"
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
            mode,
        )
        .await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get the mode(s) of a list of numbers",
            example: "echo [3 3 9 12 12 15] | math mode",
            result: Some(vec![
                UntaggedValue::decimal(3).into(),
                UntaggedValue::decimal(12).into(),
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

    modes.sort();
    Ok(UntaggedValue::Table(modes).into_value(name))
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
