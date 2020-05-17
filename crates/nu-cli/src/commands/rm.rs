use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tagged;
use std::path::PathBuf;

pub struct Remove;

#[derive(Deserialize)]
pub struct RemoveArgs {
    pub rest: Vec<Tagged<PathBuf>>,
    pub recursive: Tagged<bool>,
    #[allow(unused)]
    pub trash: Tagged<bool>,
}

impl WholeStreamCommand for Remove {
    fn name(&self) -> &str {
        "rm"
    }

    fn signature(&self) -> Signature {
        Signature::build("rm")
            .switch(
                "trash",
                "use the platform's recycle bin instead of permanently deleting",
                Some('t'),
            )
            .switch("recursive", "delete subdirectories recursively", Some('r'))
            .rest(SyntaxShape::Pattern, "the file path(s) to remove")
    }

    fn usage(&self) -> &str {
        "Remove file(s)"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        rm(args, registry)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Delete a file",
                example: "rm file.txt",
                result: None,
            },
            Example {
                description: "Move a file to the system trash",
                example: "rm --trash file.txt",
                result: None,
            },
        ]
    }
}

fn rm(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let name = args.call_info.name_tag.clone();
        let shell_manager = args.shell_manager.clone();
        let (args, _): (RemoveArgs, _) = args.process(&registry).await?;
        let mut result = shell_manager.rm(args, name)?;
        while let Some(item) = result.next().await {
            yield item;
        }
    };

    Ok(stream.to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Remove;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Remove {})
    }
}
