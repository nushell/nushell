use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_protocol::{Dictionary, Signature, UntaggedValue};

pub struct Clear;

impl WholeStreamCommand for Clear {
    fn name(&self) -> &str {
        "clear"
    }
    fn signature(&self) -> Signature {
        Signature::build("clear")
    }
    fn usage(&self) -> &str {
        "clears the terminal"
    }
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        clear(args, registry)
    }
}
pub fn clear(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let shell_manager = args.shell_manager.clone();
    let args = args.evaluate_once(registry)?;
    println!("\x1b[2J")
}
