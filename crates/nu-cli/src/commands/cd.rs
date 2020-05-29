use crate::commands::WholeStreamCommand;
use crate::prelude::*;

use std::path::PathBuf;

use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tagged;

#[derive(Deserialize)]
pub struct CdArgs {
    pub(crate) path: Option<Tagged<PathBuf>>,
}

pub struct Cd;

#[async_trait]
impl WholeStreamCommand for Cd {
    fn name(&self) -> &str {
        "cd"
    }

    fn signature(&self) -> Signature {
        Signature::build("cd").optional(
            "directory",
            SyntaxShape::Path,
            "the directory to change to",
        )
    }

    fn usage(&self) -> &str {
        "Change to a new path."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let name = args.call_info.name_tag.clone();
        let shell_manager = args.shell_manager.clone();
        let (args, _): (CdArgs, _) = args.process(&registry).await?;
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

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Cd {})
    }
}
