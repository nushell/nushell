use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue};

pub struct Command;

#[async_trait]
impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "str"
    }

    fn signature(&self) -> Signature {
        Signature::build("str").rest(
            SyntaxShape::ColumnPath,
            "optionally convert by column paths",
        )
    }

    fn usage(&self) -> &str {
        "Apply string function."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::one(ReturnSuccess::value(
            UntaggedValue::string(get_help(&Command, &args.scope)).into_value(Tag::unknown()),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::Command;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Command {})
    }
}
