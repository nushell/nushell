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
pub struct IntegerArgs {
    range: Option<Tagged<NumericRange>>,
}

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "random integer"
    }

    fn signature(&self) -> Signature {
        Signature::build("random integer").optional("range", SyntaxShape::Range, "Range of values")
    }

    fn usage(&self) -> &str {
        "Generate a random integer [min..max]"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        integer(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Generate an unconstrained random integer",
                example: "random integer",
                result: None,
            },
            Example {
                description: "Generate a random integer less than or equal to 500",
                example: "random integer ..500",
                result: None,
            },
            Example {
                description: "Generate a random integer greater than or equal to 100000",
                example: "random integer 100000..",
                result: None,
            },
            Example {
                description: "Generate a random integer between 1 and 10",
                example: "random integer 1..10",
                result: None,
            },
        ]
    }
}

pub async fn integer(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let (IntegerArgs { range }, _) = args.process().await?;

    let (min, max) = if let Some(range) = &range {
        (range.item.min(), range.item.max())
    } else {
        (0, u64::MAX)
    };

    match min.cmp(&max) {
        Ordering::Greater => Err(ShellError::labeled_error(
            format!("Invalid range {}..{}", min, max),
            "expected a valid range",
            range
                .expect("Unexpected ordering error in random integer")
                .span(),
        )),
        Ordering::Equal => {
            let untagged_result = UntaggedValue::int(min).into_value(Tag::unknown());
            Ok(OutputStream::one(ReturnSuccess::value(untagged_result)))
        }
        _ => {
            let mut thread_rng = thread_rng();
            // add 1 to max, because gen_range is right-exclusive
            let max = max.saturating_add(1);
            let result: u64 = thread_rng.gen_range(min, max);

            let untagged_result = UntaggedValue::int(result).into_value(Tag::unknown());

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
