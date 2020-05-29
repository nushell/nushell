use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{CommandAction, ReturnSuccess, Signature};

use crate::commands::WholeStreamCommand;

pub struct Previous;

#[async_trait]
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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        previous(args, registry)
    }
}

fn previous(_args: CommandArgs, _registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    Ok(vec![Ok(ReturnSuccess::Action(CommandAction::PreviousShell))].into())
}

#[cfg(test)]
mod tests {
    use super::Previous;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Previous {})
    }
}
