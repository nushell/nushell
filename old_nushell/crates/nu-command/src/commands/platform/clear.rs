use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::Signature;
use std::process::Command;

pub struct Clear;

impl WholeStreamCommand for Clear {
    fn name(&self) -> &str {
        "clear"
    }

    fn signature(&self) -> Signature {
        Signature::build("clear")
    }

    fn usage(&self) -> &str {
        "Clears the terminal."
    }

    fn run(&self, _: CommandArgs) -> Result<InputStream, ShellError> {
        if cfg!(windows) {
            Command::new("cmd")
                .args(["/C", "cls"])
                .status()
                .expect("failed to execute process");
        } else if cfg!(unix) {
            Command::new("/bin/sh")
                .args(["-c", "clear"])
                .status()
                .expect("failed to execute process");
        }
        Ok(InputStream::empty())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Clear the screen",
            example: "clear",
            result: None,
        }]
    }
}
