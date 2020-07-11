use super::variance::variance;
use crate::commands::math::utils::run_with_function;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, Signature, UntaggedValue, Value};
use std::str::FromStr;

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "math stddev"
    }

    fn signature(&self) -> Signature {
        Signature::build("math stddev")
    }

    fn usage(&self) -> &str {
        "Finds the stddev of a list of numbers or tables"
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
            stddev,
        )
        .await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get the stddev of a list of numbers",
            example: "echo [1 2 3 4 5] | math stddev",
            result: Some(vec![UntaggedValue::decimal(BigDecimal::from_str("1.414213562373095048801688724209698078569671875376948073176679737990732478462107038850387534327641573").unwrap()).into()]),
        }]
    }
}

pub fn stddev(values: &[Value], name: &Tag) -> Result<Value, ShellError> {
    let variance = variance(values, name)?.as_primitive()?;
    let sqrt_var = match variance {
        Primitive::Decimal(var) => var.sqrt(),
        _ => {
            return Err(ShellError::labeled_error(
                "Could not take square root of variance",
                "error occured here",
                name.span,
            ))
        }
    };
    match sqrt_var {
        Some(stddev) => Ok(UntaggedValue::from(Primitive::Decimal(stddev)).into_value(name)),
        None => Err(ShellError::labeled_error(
            "Could not calculate stddev",
            "error occured here",
            name.span,
        )),
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
