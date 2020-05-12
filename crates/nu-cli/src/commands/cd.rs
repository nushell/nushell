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

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, cd)?.run()
    }

    fn examples(&self) -> &[Example] {
        &[
            Example {
                description: "Change to a new directory called 'dirname'",
                example: "cd dirname",
            },
            Example {
                description: "Change to your home directory",
                example: "cd",
            },
            Example {
                description: "Change to your home directory (alternate version)",
                example: "cd ~",
            },
            Example {
                description: "Change to the previous directory",
                example: "cd -",
            },
        ]
    }
}

fn cd(args: CdArgs, context: RunnableContext) -> Result<OutputStream, ShellError> {
    context.shell_manager.cd(args, &context)
}
