use crate::commands::math::reducers::{reducer_for, Reduce};
use crate::commands::math::utils::run_with_function;
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue, Value};

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
        "Finds the maximum within a list of numbers or tables"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run_with_function(RunnableContext::from_command_args(args), maximum).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Find the maximum of list of numbers",
            example: "echo [-50 100 25] | math max",
            result: Some(vec![UntaggedValue::int(100).into()]),
        }]
    }
}

pub fn maximum(values: &[Value], _name: &Tag) -> Result<Value, ShellError> {
    let max_func = reducer_for(Reduce::Maximum);
    max_func(Value::nothing(), values.to_vec())
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
