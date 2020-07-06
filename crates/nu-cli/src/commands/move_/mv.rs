use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tagged;
use std::path::PathBuf;

pub struct Mv;

#[derive(Deserialize)]
pub struct Arguments {
    pub src: Tagged<PathBuf>,
    pub dst: Tagged<PathBuf>,
}

#[async_trait]
impl WholeStreamCommand for Mv {
    fn name(&self) -> &str {
        "mv"
    }

    fn signature(&self) -> Signature {
        Signature::build("mv")
            .required(
                "source",
                SyntaxShape::Pattern,
                "the location to move files/directories from",
            )
            .required(
                "destination",
                SyntaxShape::Path,
                "the location to move files/directories to",
            )
    }

    fn usage(&self) -> &str {
        "Move files or directories."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        mv(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Rename a file",
                example: "mv before.txt after.txt",
                result: None,
            },
            Example {
                description: "Move a file into a directory",
                example: "mv test.txt my/subdirectory",
                result: None,
            },
            Example {
                description: "Move many files into a directory",
                example: "mv *.txt my/subdirectory",
                result: None,
            },
        ]
    }
}

async fn mv(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let name = args.call_info.name_tag.clone();
    let shell_manager = args.shell_manager.clone();
    let (args, _) = args.process(&registry).await?;

    shell_manager.mv(args, name)
}

#[cfg(test)]
mod tests {
    use super::Mv;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Mv {})
    }
}
