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
        cd(args, registry)
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

fn cd(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let name = args.call_info.name_tag.clone();
        let shell_manager = args.shell_manager.clone();

        let args: CdArgs = args.process_raw(&registry).await?;
        let result = shell_manager.cd(args, name)?;
        for item in result.next().await {
            yield item;
        }
    };

    Ok(stream.to_output_stream())
}
