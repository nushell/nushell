use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::Signature;

pub struct From;

impl WholeStreamCommand for From {
    fn name(&self) -> &str {
        "from"
    }

    fn signature(&self) -> Signature {
        Signature::build("from")
    }

    fn usage(&self) -> &str {
        "Parse content (string or binary) as a table (input format based on subcommand, like csv, ini, json, toml)"
    }

    fn run(
        &self,
        _args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        Ok(crate::commands::help::get_help(&*self, registry).into())
    }
}
