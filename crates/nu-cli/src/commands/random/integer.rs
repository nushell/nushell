use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;
use rand::prelude::{thread_rng, Rng};

pub struct SubCommand;

#[derive(Deserialize)]
pub struct IntegerArgs {
    min: Option<Tagged<u64>>,
    max: Option<Tagged<u64>>,
}

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "random integer"
    }

    fn signature(&self) -> Signature {
        Signature::build("random integer")
            .named("min", SyntaxShape::Int, "Minimum value", Some('m'))
            .named("max", SyntaxShape::Int, "Maximum value", Some('x'))
    }

    fn usage(&self) -> &str {
        "Generate a random integer [--min <m>] [--max <x>]"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        integer(args, registry).await
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
                example: "random integer --max 500",
                result: None,
            },
            Example {
                description: "Generate a random integer greater than or equal to 100000",
                example: "random integer --min 100000",
                result: None,
            },
            Example {
                description: "Generate a random integer between 1 and 10",
                example: "random integer --min 1 --max 10",
                result: None,
            },
        ]
    }
}

pub async fn integer(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let (IntegerArgs { min, max }, _) = args.process(&registry).await?;

    let min = if let Some(min_tagged) = min {
        *min_tagged
    } else {
        0
    };

    let max = if let Some(max_tagged) = max {
        *max_tagged
    } else {
        u64::MAX
    };

    let mut thread_rng = thread_rng();
    let result: u64 = thread_rng.gen_range(min, max);

    let untagged_result = UntaggedValue::int(result).into_value(Tag::unknown());

    Ok(OutputStream::one(ReturnSuccess::value(untagged_result)))
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
