use crate::commands::WholeStreamCommand;
use crate::prelude::*;
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
        "clears the terminal"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        clear(args, registry)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Clear the screen",
            example: "clear",
            result: None,
        }]
    }
}

fn clear(_args: CommandArgs, _registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    if cfg!(windows) {
        Command::new("cmd")
            .args(&["/C", "cls"])
            .status()
            .expect("failed to execute process");
    } else if cfg!(unix) {
        Command::new("/bin/sh")
            .args(&["-c", "clear"])
            .status()
            .expect("failed to execute process");
    }
    Ok(OutputStream::empty())
}

#[cfg(test)]
mod tests {
    use super::Clear;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Clear {})
    }
}
