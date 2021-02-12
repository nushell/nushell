use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};

pub struct Mv;

#[async_trait]
impl WholeStreamCommand for Mv {
    fn name(&self) -> &str {
        "mv"
    }

    fn signature(&self) -> Signature {
        Signature::build("mv")
            .required(
                "source",
                SyntaxShape::GlobPattern,
                "the location to move files/directories from",
            )
            .required(
                "destination",
                SyntaxShape::FilePath,
                "the location to move files/directories to",
            )
    }

    fn usage(&self) -> &str {
        "Move files or directories."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        mv(args).await
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

async fn mv(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.clone();
    let shell_manager = args.shell_manager.clone();
    let (args, _) = args.process().await?;

    shell_manager.mv(args, name)
}

#[cfg(test)]
mod tests {
    use super::Mv;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Mv {})
    }
}
