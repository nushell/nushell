use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
pub struct Mkdir;

#[async_trait]
impl WholeStreamCommand for Mkdir {
    fn name(&self) -> &str {
        "mkdir"
    }

    fn signature(&self) -> Signature {
        Signature::build("mkdir")
            .rest(
                SyntaxShape::FilePath,
                "the name(s) of the path(s) to create",
            )
            .switch("show-created-paths", "show the path(s) created.", Some('s'))
    }

    fn usage(&self) -> &str {
        "Make directories, creates intermediary directories as required."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        mkdir(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Make a directory named foo",
            example: "mkdir foo",
            result: None,
        }]
    }
}

async fn mkdir(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.clone();
    let shell_manager = args.shell_manager.clone();
    let (args, _) = args.process().await?;

    shell_manager.mkdir(args, name)
}

#[cfg(test)]
mod tests {
    use super::Mkdir;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Mkdir {})
    }
}
