use crate::commands::math::utils::run_with_numerical_functions_on_stream;
use crate::prelude::*;
use bigdecimal::One;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue, Value};

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "math floor"
    }

    fn signature(&self) -> Signature {
        Signature::build("math floor")
    }

    fn usage(&self) -> &str {
        "Applies the floor function to a list of numbers"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run_with_numerical_functions_on_stream(
            RunnableContext {
                input: args.input,
                scope: args.scope.clone(),
                shell_manager: args.shell_manager,
                host: args.host,
                ctrl_c: args.ctrl_c,
                current_errors: args.current_errors,
                name: args.call_info.name_tag,
            },
            floor_big_int,
            floor_big_decimal,
            floor_default,
        )
        .await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Apply the floor function to a list of numbers",
            example: "echo [1.5 2.3 -3.1] | math floor",
            result: Some(vec![
                UntaggedValue::int(1).into(),
                UntaggedValue::int(2).into(),
                UntaggedValue::int(-4).into(),
            ]),
        }]
    }
}

fn floor_big_int(val: BigInt) -> Value {
    UntaggedValue::int(val).into()
}

fn floor_big_decimal(val: BigDecimal) -> Value {
    let mut maybe_floored = val.round(0);
    if maybe_floored > val {
        maybe_floored -= BigDecimal::one();
    }
    let (floored, _) = maybe_floored.into_bigint_and_exponent();
    UntaggedValue::int(floored).into()
}

fn floor_default(_: UntaggedValue) -> Value {
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

        test_examples(SubCommand {})
    }
}
