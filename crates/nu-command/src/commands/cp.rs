use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};

pub struct Cpy;

#[async_trait]
impl WholeStreamCommand for Cpy {
    fn name(&self) -> &str {
        "cp"
    }

    fn signature(&self) -> Signature {
        Signature::build("cp")
            .required("src", SyntaxShape::GlobPattern, "the place to copy from")
            .required("dst", SyntaxShape::FilePath, "the place to copy to")
            .switch(
                "recursive",
                "copy recursively through subdirectories",
                Some('r'),
            )
    }

    fn usage(&self) -> &str {
        "Copy files."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let shell_manager = args.shell_manager.clone();
        let name = args.call_info.name_tag.clone();
        let (args, _) = args.process().await?;
        shell_manager.cp(args, name)
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

#[cfg(test)]
mod tests {
    use super::Cpy;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Cpy {})
    }
}
