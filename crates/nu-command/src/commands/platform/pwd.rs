use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::Signature;

pub struct Pwd;

impl WholeStreamCommand for Pwd {
    fn name(&self) -> &str {
        "pwd"
    }

    fn signature(&self) -> Signature {
        Signature::build("pwd")
    }

    fn usage(&self) -> &str {
        "Output the current working directory."
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        pwd(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Print the current working directory",
            example: "pwd",
            result: None,
        }]
    }
}

pub fn pwd(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let shell_manager = args.shell_manager();

    shell_manager.pwd(args)
}

#[cfg(test)]
mod tests {
    use super::Pwd;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Pwd {})
    }
}
