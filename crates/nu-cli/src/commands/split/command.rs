use crate::commands::WholeStreamCommand;
use crate::prelude::*;
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

    async fn run(
        &self,
        _args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let registry = registry.clone();
        Ok(OutputStream::one(Ok(ReturnSuccess::Value(
            UntaggedValue::string(crate::commands::help::get_help(&Command, &registry))
                .into_value(Tag::unknown()),
        ))))
    }
}

#[cfg(test)]
mod tests {
    use super::Command;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Command {})
    }
}
