use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, UntaggedValue, Signature};

pub struct Autoenv;

#[async_trait]
impl WholeStreamCommand for Autoenv {
    fn name(&self) -> &str {
        "autoenv"
    }
    fn usage(&self) -> &str {
        // "Mark a .nu-env file in a directory as trusted. Needs to be re-run after each change to the file or its filepath."
        "Manage directory specific environments"
    }
    fn signature(&self) -> Signature {
        Signature::build("autoenv")
    }
    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let registry = registry.clone();
        Ok(OutputStream::one(ReturnSuccess::value(
            UntaggedValue::string(crate::commands::help::get_help(&Autoenv, &registry))
                .into_value(Tag::unknown()),
        )))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Allow .nu-env file in current directory",
            example: "autoenv trust",
            result: None
        }]
    }
}
