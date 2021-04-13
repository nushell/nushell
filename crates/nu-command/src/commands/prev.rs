use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{CommandAction, ReturnSuccess, Signature};

use nu_engine::WholeStreamCommand;

pub struct Previous;

impl WholeStreamCommand for Previous {
    fn name(&self) -> &str {
        "p"
    }

    fn signature(&self) -> Signature {
        Signature::build("p")
    }

    fn usage(&self) -> &str {
        "Go to previous shell."
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        Ok(previous(args))
    }
}

fn previous(_args: CommandArgs) -> ActionStream {
    vec![Ok(ReturnSuccess::Action(CommandAction::PreviousShell))].into()
}

#[cfg(test)]
mod tests {
    use super::Previous;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Previous {})
    }
}
