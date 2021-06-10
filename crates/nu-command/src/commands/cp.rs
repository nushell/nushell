use crate::prelude::*;
use nu_engine::{shell::CopyArgs, WholeStreamCommand};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};

pub struct Cpy;

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

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        let shell_manager = args.shell_manager();
        let name = args.call_info.name_tag.clone();

        let args = CopyArgs {
            src: args.req(0)?,
            dst: args.req(1)?,
            recursive: args.has_flag("recursive"),
        };
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
