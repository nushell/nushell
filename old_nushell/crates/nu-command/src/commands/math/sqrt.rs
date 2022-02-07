use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, Signature, UntaggedValue, Value};

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "math sqrt"
    }

    fn signature(&self) -> Signature {
        Signature::build("math sqrt")
    }

    fn usage(&self) -> &str {
        "Applies the square root function to a list of numbers"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        Ok(operate(args))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Apply the square root function to a list of numbers",
            example: "echo [9 16] | math sqrt",
            result: Some(vec![
                UntaggedValue::int(3).into(),
                UntaggedValue::int(4).into(),
            ]),
        }]
    }
}

fn operate(args: CommandArgs) -> OutputStream {
    let mapped = args.input.map(move |val| match val.value {
        UntaggedValue::Primitive(Primitive::Int(val)) => sqrt_big_decimal(BigDecimal::from(val)),
        UntaggedValue::Primitive(Primitive::Decimal(val)) => sqrt_big_decimal(val),
        other => sqrt_default(other),
    });
    mapped.into_output_stream()
}

fn sqrt_big_decimal(val: BigDecimal) -> Value {
    let squared = val.sqrt();
    match squared {
        None => UntaggedValue::Error(ShellError::untagged_runtime_error(
            "Can't square root a negative number",
        ))
        .into(),
        Some(val) if !val.is_integer() => UntaggedValue::decimal(val.normalized()).into(),
        Some(val) => match val.to_i64() {
            Some(x) => UntaggedValue::int(x).into(),
            None => UntaggedValue::Error(ShellError::untagged_runtime_error(
                "Value too large to convert to 64-bit integer",
            ))
            .into(),
        },
    }
}

fn sqrt_default(_: UntaggedValue) -> Value {
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
