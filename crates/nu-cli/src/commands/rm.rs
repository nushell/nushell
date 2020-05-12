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
        args.process(registry, rm)?.run()
    }

    fn examples(&self) -> &[Example] {
        &[
            Example {
                description: "Delete a file",
                example: "rm file.txt",
            },
            Example {
                description: "Move a file to the system trash",
                example: "rm --trash file.txt",
            },
        ]
    }
}

fn rm(args: RemoveArgs, context: RunnableContext) -> Result<OutputStream, ShellError> {
    let shell_manager = context.shell_manager.clone();
    shell_manager.rm(args, &context)
}
