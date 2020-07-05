use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;

pub struct Precision;

#[derive(Deserialize)]
pub struct PrecisionArgs {
    digits: Tagged<u64>,
}

#[async_trait]
impl WholeStreamCommand for Precision {
    fn name(&self) -> &str {
        "precision"
    }

    fn signature(&self) -> Signature {
        Signature::build("precision").required(
            "points",
            SyntaxShape::Int,
            "number of decimal points to which to round",
        )
    }

    fn usage(&self) -> &str {
        "Rounds decimal number to specified number of decimal digits."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        precision(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "round to four digits",
            example: "= 1.324161342 | precision 4",
            result: Some(vec![UntaggedValue::decimal(1.3242).into()]),
        }]
    }
}

async fn precision(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let (PrecisionArgs { digits: points }, input) = args.process(&registry.clone()).await?;

    Ok(input
        .map(move |val| {
            ReturnSuccess::value(
                if let UntaggedValue::Primitive(Primitive::Decimal(decimal)) = val.value {
                    UntaggedValue::decimal(as_rounded_decimal(&decimal, points.item))
                } else {
                    val.value
                },
            )
        })
        .to_output_stream())
}

pub fn as_rounded_decimal(decimal: &BigDecimal, mut digits: u64) -> BigDecimal {
    let mut mag = decimal.clone();
    while mag >= BigDecimal::from(1) {
        mag = mag / 10;
        digits += 1;
    }

    decimal.with_prec(digits)
}

#[cfg(test)]
mod tests {
    use super::Precision;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Precision {})
    }
}
