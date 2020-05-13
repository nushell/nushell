use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tagged;
use std::path::PathBuf;

pub struct Cpy;

#[derive(Deserialize)]
pub struct CopyArgs {
    pub src: Tagged<PathBuf>,
    pub dst: Tagged<PathBuf>,
    pub recursive: Tagged<bool>,
}

impl WholeStreamCommand for Cpy {
    fn name(&self) -> &str {
        "cp"
    }

    fn signature(&self) -> Signature {
        Signature::build("cp")
            .required("src", SyntaxShape::Pattern, "the place to copy from")
            .required("dst", SyntaxShape::Path, "the place to copy to")
            .switch(
                "recursive",
                "copy recursively through subdirectories",
                Some('r'),
            )
    }

    fn usage(&self) -> &str {
        "Copy files."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        cp(args, registry)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Copy myfile to dir_b",
                example: "cp myfile dir_b",
                result: None,
            },
            Example {
                description: "Recursively copy dir_a to dir_b",
                example: "cp -r dir_a dir_b",
                result: None,
            },
        ]
    }
}

pub fn cp(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let shell_manager = args.shell_manager.clone();
        let name = args.call_info.name_tag.clone();
        let (args, _) = args.process(&registry).await?;
        let mut result = shell_manager.cp(args, name)?;

        while let Some(item) = result.next().await {
            yield item;
        }
    };

    Ok(stream.to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Cpy;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Cpy {})
    }
}
