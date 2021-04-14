use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "math round"
    }

    fn signature(&self) -> Signature {
        Signature::build("math round").named(
            "precision",
            SyntaxShape::Number,
            "digits of precision",
            Some('p'),
        )
    }

    fn usage(&self) -> &str {
        "Applies the round function to a list of numbers"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        operate(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Apply the round function to a list of numbers",
                example: "echo [1.5 2.3 -3.1] | math round",
                result: Some(vec![
                    UntaggedValue::int(2).into(),
                    UntaggedValue::int(2).into(),
                    UntaggedValue::int(-3).into(),
                ]),
            },
            Example {
                description: "Apply the round function with precision specified",
                example: "echo [1.555 2.333 -3.111] | math round -p 2",
                result: Some(vec![
                    UntaggedValue::decimal_from_float(1.56, Span::default()).into(),
                    UntaggedValue::decimal_from_float(2.33, Span::default()).into(),
                    UntaggedValue::decimal_from_float(-3.11, Span::default()).into(),
                ]),
            },
        ]
    }
}

fn operate(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once()?;
    let precision: Option<Result<Tagged<i16>, ShellError>> = args.get_flag("precision");
    let input = args.input;
    let precision = if let Some(precision) = precision {
        precision?.item
    } else {
        0
    };
    let mapped = input.map(move |val| match val.value {
        UntaggedValue::Primitive(Primitive::Int(val)) => round_big_int(val),
        UntaggedValue::Primitive(Primitive::Decimal(val)) => {
            round_big_decimal(val, precision.into())
        }
        other => round_default(other),
    });
    Ok(mapped.to_output_stream())
}

fn round_big_int(val: BigInt) -> Value {
    UntaggedValue::int(val).into()
}

fn round_big_decimal(val: BigDecimal, precision: i64) -> Value {
    if precision > 0 {
        UntaggedValue::decimal(val.with_scale(precision + 1).round(precision)).into()
    } else {
        let (rounded, _) = val
            .with_scale(precision + 1)
            .round(precision)
            .as_bigint_and_exponent();
        UntaggedValue::int(rounded).into()
    }
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

        test_examples(SubCommand {})
    }
}
