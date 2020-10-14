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
        "math ceil"
    }

    fn signature(&self) -> Signature {
        Signature::build("math celi")
    }

    fn usage(&self) -> &str {
        "Applies the ceil function to a list of numbers"
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
            ceil,
        )
        .await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Apply the ceil function to a list of numbers",
            example: "echo [1.5 2.3 -3.1] | math ceil",
            result: Some(vec![
                UntaggedValue::int(2).into(),
                UntaggedValue::int(3).into(),
                UntaggedValue::int(-3).into(),
            ]),
        }]
    }
}

fn ceil_big_decimal(val: &BigDecimal) -> BigInt {
    let mut maybe_ceiled = val.round(0);
    if &maybe_ceiled < val {
        maybe_ceiled += BigDecimal::one();
    }
    let (ceiled, _) = maybe_ceiled.into_bigint_and_exponent();
    ceiled
}

/// Calculate product of given values
pub fn ceil(values: &[Value], _name: &Tag) -> Result<Value, ShellError> {
    if !values.iter()
    .all(|val|matches!(val.value, UntaggedValue::Primitive(Primitive::Int(_))|UntaggedValue::Primitive(Primitive::Decimal(_))))
    {
        return Err(ShellError::unexpected("Only numerical values are supported"));
    }
    let ceiled: Vec<Value> = values
        .iter()
        .map(|val| match &val.value {
            UntaggedValue::Primitive(Primitive::Int(val)) => UntaggedValue::int(val.clone()).into(),
            UntaggedValue::Primitive(Primitive::Decimal(val)) => {
                UntaggedValue::int(ceil_big_decimal(val)).into()
            }
            _ => UntaggedValue::int(0).into(),
        })
        .collect();
    Ok(UntaggedValue::table(&ceiled).into())
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
