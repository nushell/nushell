use crate::commands::WholeStreamCommand;
use std::process::Command;
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
fn clear(_args: CommandArgs, _registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    if cfg!(windows) {
        Command::new("cmd")
            .args(&["/C", "cls"])
            .output()
            .expect("failed to execute process");
    } else if cfg!(unix) {
        println!("\x1b[2J");
    }
    return Ok(OutputStream::empty())
}
