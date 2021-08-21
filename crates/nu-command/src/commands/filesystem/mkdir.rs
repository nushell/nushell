use crate::prelude::*;
use nu_engine::{shell::MkdirArgs, WholeStreamCommand};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
pub struct Mkdir;

impl WholeStreamCommand for Mkdir {
    fn name(&self) -> &str {
        "mkdir"
    }

    fn signature(&self) -> Signature {
        Signature::build("mkdir")
            .rest(
                "rest",
                SyntaxShape::FilePath,
                "the name(s) of the path(s) to create",
            )
            .switch("show-created-paths", "show the path(s) created.", Some('s'))
    }

    fn usage(&self) -> &str {
        "Make directories, creates intermediary directories as required."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        mkdir(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Make a directory named foo",
            example: "mkdir foo",
            result: None,
        }]
    }
}

fn mkdir(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.clone();
    let shell_manager = args.shell_manager();

    let args = MkdirArgs {
        rest: args.rest(0)?,
        show_created_paths: args.has_flag("show-created-paths"),
    };

    shell_manager.mkdir(args, name)
}

#[cfg(test)]
mod tests {
    use super::Mkdir;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Mkdir {})
    }
}
