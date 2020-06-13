use crate::commands::math::utils::calculate;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use crate::utils::data_processing::{reducer_for, Reduce};
use bigdecimal::FromPrimitive;
use nu_errors::ShellError;
use nu_protocol::{Primitive, Signature, UntaggedValue, Value};
struct MaximumArgs {}

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "math max"
    }

    fn signature(&self) -> Signature {
        Signature::build("math max")
    }

    fn usage(&self) -> &str {
        "Get the maximum of a list of numbers or tables"
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
            maximum,
        )
        .await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Find the maximum of list of numbers",
            example: "echo [-50 100.0 25] | math maximum",
            result: Some(vec![UntaggedValue::decimal(25).into()]),
        }]
    }
}

pub fn maximum(values: &[Value], _name: &Tag) -> Result<Value, ShellError> {
    let max_func = reducer_for(Reduce::Maximum);
    max_func(Value::nothing(), values.to_vec())
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
