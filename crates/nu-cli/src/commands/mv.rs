use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tagged;
use std::path::PathBuf;

pub struct Move;

#[derive(Deserialize)]
pub struct MoveArgs {
    pub src: Tagged<PathBuf>,
    pub dst: Tagged<PathBuf>,
}

impl WholeStreamCommand for Move {
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

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        mv(args, registry)
    }

    fn examples(&self) -> &[Example] {
        &[
            Example {
                description: "Rename a file",
                example: "mv before.txt after.txt",
            },
            Example {
                description: "Move a file into a directory",
                example: "mv test.txt my/subdirectory",
            },
            Example {
                description: "Move many files into a directory",
                example: "mv *.txt my/subdirectory",
            },
        ]
    }
}

fn mv(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let name = args.call_info.name_tag.clone();
        let shell_manager = args.shell_manager.clone();
        let (args, _) = args.process(&registry).await?;
        let result = shell_manager.mv(args, name)?;

        for item in result.next().await {
            yield item;
        }
    };

    Ok(stream.to_output_stream())
}
