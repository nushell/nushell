use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, UntaggedValue, Value};

pub struct Autoenv;

#[async_trait]
impl WholeStreamCommand for Autoenv {
    fn name(&self) -> &str {
        "autoenv"
    }
    fn usage(&self) -> &str {
        "Mark a .nu-env file in a directory as trusted. Needs to be re-made after each change to the file."
    }
    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        allow(args, registry).await
    }
    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Allow .nu-env file in current directory",
            example: "autoenv trust",
            result: "Current "
        }]
    }
}