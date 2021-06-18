use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{CommandAction, ReturnSuccess, Signature};

pub struct Next;

impl WholeStreamCommand for Next {
    fn name(&self) -> &str {
        "n"
    }

    fn signature(&self) -> Signature {
        Signature::build("n")
    }

    fn usage(&self) -> &str {
        "Go to next shell."
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        Ok(next(args))
    }
}

fn next(_args: CommandArgs) -> ActionStream {
    vec![Ok(ReturnSuccess::Action(CommandAction::NextShell))].into()
}

#[cfg(test)]
mod tests {
    use super::Next;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Next {})
    }
}
