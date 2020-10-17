use crate::commands::math::utils::run_with_numerical_functions_on_stream;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue, Value};

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "math round"
    }

    fn signature(&self) -> Signature {
        Signature::build("math round")
    }

    fn usage(&self) -> &str {
        "Applies the round function to a list of numbers"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        run_with_numerical_functions_on_stream(
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
            round_big_int,
            round_big_decimal,
            round_default,
        )
        .await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Apply the round function to a list of numbers",
            example: "echo [1.5 2.3 -3.1] | math round",
            result: Some(vec![
                UntaggedValue::int(2).into(),
                UntaggedValue::int(2).into(),
                UntaggedValue::int(-3).into(),
            ]),
        }]
    }
}

fn round_big_int(val: BigInt) -> Value {
    UntaggedValue::int(val).into()
}

fn round_big_decimal(val: BigDecimal) -> Value {
    let (rounded, _) = val.round(0).as_bigint_and_exponent();
    UntaggedValue::int(rounded).into()
}

fn round_default(_: UntaggedValue) -> Value {
    UntaggedValue::Error(ShellError::unexpected(
        "Only numerical values are supported",
    ))
    .into()
}
#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::SubCommand;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        Ok(test_examples(SubCommand {})?)
    }
}
