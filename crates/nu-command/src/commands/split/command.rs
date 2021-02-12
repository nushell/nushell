use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};

#[derive(Clone)]
pub struct Command;

#[async_trait]
impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "split"
    }

    fn signature(&self) -> Signature {
        Signature::build("split")
    }

    fn usage(&self) -> &str {
        "split contents across desired subcommand (like row, column) via the separator."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::one(Ok(ReturnSuccess::Value(
            UntaggedValue::string(get_help(&Command, &args.scope)).into_value(Tag::unknown()),
        ))))
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
