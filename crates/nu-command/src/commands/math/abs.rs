use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, Signature, UntaggedValue, Value};

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "math abs"
    }

    fn signature(&self) -> Signature {
        Signature::build("math abs")
    }

    fn usage(&self) -> &str {
        "Returns absolute values of a list of numbers"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let mapped = args.input.map(move |val| match val.value {
            UntaggedValue::Primitive(Primitive::Int(val)) => {
                UntaggedValue::int(val.magnitude().clone()).into()
            }
            UntaggedValue::Primitive(Primitive::Decimal(val)) => {
                UntaggedValue::decimal(val.abs()).into()
            }
            UntaggedValue::Primitive(Primitive::Duration(val)) => {
                UntaggedValue::duration(val.magnitude().clone()).into()
            }
            other => abs_default(other),
        });
        Ok(OutputStream::from_input(mapped))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get absolute of each value in a list of numbers",
            example: "echo [-50 -100.0 25] | math abs",
            result: Some(vec![
                UntaggedValue::int(50).into(),
                UntaggedValue::decimal_from_float(100.0, Span::default()).into(),
                UntaggedValue::int(25).into(),
            ]),
        }]
    }
}

fn abs_default(_: UntaggedValue) -> Value {
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
