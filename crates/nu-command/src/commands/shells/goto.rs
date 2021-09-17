use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{CommandAction, ReturnSuccess, Signature, SyntaxShape};

pub struct Goto;

impl WholeStreamCommand for Goto {
    fn name(&self) -> &str {
        "g"
    }

    fn signature(&self) -> Signature {
        Signature::build("g").required("index", SyntaxShape::Int, "the shell's index to go to")
    }

    fn usage(&self) -> &str {
        "Go to specified shell."
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        goto(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Enter the first shell",
            example: "g 0",
            result: None,
        }]
    }
}

fn goto(args: CommandArgs) -> Result<ActionStream, ShellError> {
    Ok(ActionStream::one(ReturnSuccess::action(
        CommandAction::GotoShell(args.req(0)?),
    )))
}

#[cfg(test)]
mod tests {
    use super::Goto;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Goto {})
    }
}
