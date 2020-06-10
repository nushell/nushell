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
    #[serde(rename = "show-created-paths")]
    pub show_created_paths: bool,
}

#[async_trait]
impl WholeStreamCommand for Mkdir {
    fn name(&self) -> &str {
        "mkdir"
    }

    fn signature(&self) -> Signature {
        Signature::build("mkdir")
            .rest(SyntaxShape::Path, "the name(s) of the path(s) to create")
            .switch("show-created-paths", "show the path(s) created.", Some('s'))
    }

    fn usage(&self) -> &str {
        "Make directories, creates intermediary directories as required."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        mkdir(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Make a directory named foo",
            example: "mkdir foo",
            result: None,
        }]
    }
}

async fn mkdir(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let name = args.call_info.name_tag.clone();
    let shell_manager = args.shell_manager.clone();
    let (args, _) = args.process(&registry).await?;

    shell_manager.mkdir(args, name)
}

#[cfg(test)]
mod tests {
    use super::Mkdir;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Mkdir {})
    }
}
