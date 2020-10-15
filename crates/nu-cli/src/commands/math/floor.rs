use crate::commands::math::utils::run_with_function;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use bigdecimal::One;
use nu_errors::ShellError;
use nu_protocol::{Primitive, Signature, UntaggedValue, Value};

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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        run_with_function(
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
            floor,
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

fn floor_big_decimal(val: &BigDecimal) -> BigInt {
    let mut maybe_floored = val.round(0);
    if &maybe_floored > val {
        maybe_floored -= BigDecimal::one();
    }
    let (floored, _) = maybe_floored.into_bigint_and_exponent();
    floored
}

/// Applies floor function to given values
pub fn floor(values: &[Value], _name: &Tag) -> Result<Value, ShellError> {
    let mut floored = Vec::with_capacity(values.len());
    for value in values {
        match &value.value {
            UntaggedValue::Primitive(Primitive::Int(val)) => {
                floored.push(UntaggedValue::int(val.clone()).into())
            }
            UntaggedValue::Primitive(Primitive::Decimal(val)) => {
                floored.push(UntaggedValue::int(floor_big_decimal(val)).into())
            }
            _ => {
                return Err(ShellError::unexpected(
                    "Only numerical values are supported",
                ))
            }
        }
    }
    Ok(UntaggedValue::table(&floored).into())
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
