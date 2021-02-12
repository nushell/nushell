use crate::prelude::*;
use nu_engine::deserializer::NumericRange;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;
use rand::prelude::{thread_rng, Rng};
use std::cmp::Ordering;

pub struct SubCommand;

#[derive(Deserialize)]
pub struct DecimalArgs {
    range: Option<Tagged<NumericRange>>,
}

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "random decimal"
    }

    fn signature(&self) -> Signature {
        Signature::build("random decimal").optional("range", SyntaxShape::Range, "Range of values")
    }

    fn usage(&self) -> &str {
        "Generate a random decimal within a range [min..max]"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        decimal(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Generate a default decimal value between 0 and 1",
                example: "random decimal",
                result: None,
            },
            Example {
                description: "Generate a random decimal less than or equal to 500",
                example: "random decimal ..500",
                result: None,
            },
            Example {
                description: "Generate a random decimal greater than or equal to 100000",
                example: "random decimal 100000..",
                result: None,
            },
            Example {
                description: "Generate a random decimal between 1 and 10",
                example: "random decimal 1..10",
                result: None,
            },
        ]
    }
}

pub async fn decimal(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let (DecimalArgs { range }, _) = args.process().await?;

    let (min, max) = if let Some(range) = &range {
        (range.item.min() as f64, range.item.max() as f64)
    } else {
        (0.0, 1.0)
    };

    match min.partial_cmp(&max) {
        Some(Ordering::Greater) => Err(ShellError::labeled_error(
            format!("Invalid range {}..{}", min, max),
            "expected a valid range",
            range
                .expect("Unexpected ordering error in random decimal")
                .span(),
        )),
        Some(Ordering::Equal) => {
            let untagged_result = UntaggedValue::decimal_from_float(min, Span::new(64, 64));
            Ok(OutputStream::one(ReturnSuccess::value(untagged_result)))
        }
        _ => {
            let mut thread_rng = thread_rng();
            let result: f64 = thread_rng.gen_range(min, max);

            let untagged_result = UntaggedValue::decimal_from_float(result, Span::new(64, 64));

            Ok(OutputStream::one(ReturnSuccess::value(untagged_result)))
        }
    }
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
