use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tagged;
use std::path::PathBuf;

pub struct Mkdir;

#[derive(Deserialize)]
pub struct MkdirArgs {
    pub rest: Vec<Tagged<PathBuf>>,
}

impl WholeStreamCommand for Mkdir {
    fn name(&self) -> &str {
        "mkdir"
    }

    fn signature(&self) -> Signature {
        Signature::build("mkdir").rest(SyntaxShape::Path, "the name(s) of the path(s) to create")
    }

    fn usage(&self) -> &str {
        "Make directories, creates intermediary directories as required."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        mkdir(args, registry)
    }

    fn examples(&self) -> &[Example] {
        &[Example {
            description: "Make a directory named foo",
            example: "mkdir foo",
        }]
    }
}

fn mkdir(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let name = args.call_info.name_tag.clone();
        let shell_manager = args.shell_manager.clone();
        let args = args.process_raw(&registry).await?;
        let result = shell_manager.mkdir(args, name)?;

        for item in result.next().await {
            yield item;
        }
    };

    Ok(stream.to_output_stream())
}
