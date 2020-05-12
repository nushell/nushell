use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::Signature;

pub struct To;

impl WholeStreamCommand for To {
    fn name(&self) -> &str {
        "to"
    }

    fn signature(&self) -> Signature {
        Signature::build("to")
    }

    fn usage(&self) -> &str {
        "Convert table into an output format (based on subcommand, like csv, html, json, yaml)."
    }

    fn run(
        &self,
        _args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        Ok(crate::commands::help::get_help(self, registry).into())
    }
}
