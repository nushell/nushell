use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};

pub struct Command;

#[async_trait]
impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "random"
    }

    fn signature(&self) -> Signature {
        Signature::build("random")
    }

    fn usage(&self) -> &str {
        "Generate random values"
    }

    async fn run(
        &self,
        _args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::one(Ok(ReturnSuccess::Value(
            UntaggedValue::string(crate::commands::help::get_help(&Command, &registry.clone()))
                .into_value(Tag::unknown()),
        ))))
    }
}
