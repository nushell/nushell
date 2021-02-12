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
        "math min"
    }

    fn signature(&self) -> Signature {
        Signature::build("math min")
    }

    fn usage(&self) -> &str {
        "Finds the minimum within a list of numbers or tables"
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
            minimum,
        )
        .await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get the minimum of a list of numbers",
            example: "echo [-50 100 25] | math min",
            result: Some(vec![UntaggedValue::int(-50).into()]),
        }]
    }
}

pub fn minimum(values: &[Value], _name: &Tag) -> Result<Value, ShellError> {
    let min_func = reducer_for(Reduce::Minimum);
    min_func(Value::nothing(), values.to_vec())
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
