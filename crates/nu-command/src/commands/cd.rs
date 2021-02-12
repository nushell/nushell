use crate::prelude::*;
use nu_engine::WholeStreamCommand;

use nu_engine::shell::CdArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};

pub struct Cd;

#[async_trait]
impl WholeStreamCommand for Cd {
    fn name(&self) -> &str {
        "cd"
    }

    fn signature(&self) -> Signature {
        Signature::build("cd").optional(
            "directory",
            SyntaxShape::FilePath,
            "the directory to change to",
        )
    }

    fn usage(&self) -> &str {
        "Change to a new path."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let name = args.call_info.name_tag.clone();
        let shell_manager = args.shell_manager.clone();
        let (args, _): (CdArgs, _) = args.process().await?;
        shell_manager.cd(args, name)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Change to a new directory called 'dirname'",
                example: "cd dirname",
                result: None,
            },
            Example {
                description: "Change to your home directory",
                example: "cd",
                result: None,
            },
            Example {
                description: "Change to your home directory (alternate version)",
                example: "cd ~",
                result: None,
            },
            Example {
                description: "Change to the previous directory",
                example: "cd -",
                result: None,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::Cd;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Cd {})
    }
}
