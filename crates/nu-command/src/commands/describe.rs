use crate::prelude::*;
use nu_engine::WholeStreamCommand;

use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};

pub struct Describe;

#[derive(Deserialize)]
pub struct DescribeArgs {}

#[async_trait]
impl WholeStreamCommand for Describe {
    fn name(&self) -> &str {
        "describe"
    }

    fn signature(&self) -> Signature {
        Signature::build("describe")
    }

    fn usage(&self) -> &str {
        "Describes the objects in the stream."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        describe(args).await
    }
}

pub async fn describe(args: CommandArgs) -> Result<OutputStream, ShellError> {
    Ok(args
        .input
        .map(|row| {
            let name = value::format_type(&row, 100);
            ReturnSuccess::value(
                UntaggedValue::string(name).into_value(Tag::unknown_anchor(row.tag.span)),
            )
        })
        .to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Describe;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Describe {})
    }
}
