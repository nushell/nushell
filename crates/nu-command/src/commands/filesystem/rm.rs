use crate::prelude::*;
use nu_engine::shell::RemoveArgs;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};

pub struct Remove;

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
            .switch(
                "permanent",
                "don't use recycle bin, delete permanently",
                Some('p'),
            )
            .switch("recursive", "delete subdirectories recursively", Some('r'))
            .switch("force", "suppress error when no file", Some('f'))
            .rest(SyntaxShape::GlobPattern, "the file path(s) to remove")
    }

    fn usage(&self) -> &str {
        "Remove file(s)."
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        rm(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Delete or move a file to the system trash (depending on 'rm_always_trash' config option)",
                example: "rm file.txt",
                result: None,
            },
            Example {
                description: "Move a file to the system trash",
                example: "rm --trash file.txt",
                result: None,
            },
            Example {
                description: "Delete a file permanently",
                example: "rm --permanent file.txt",
                result: None,
            },
            Example {
                description: "Delete a file, and suppress errors if no file is found",
                example: "rm --force file.txt",
                result: None,
            }
        ]
    }
}

fn rm(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let name = args.call_info.name_tag.clone();
    let shell_manager = args.shell_manager();

    let args = RemoveArgs {
        rest: args.rest(0)?,
        recursive: args.has_flag("recursive"),
        trash: args.has_flag("trash"),
        permanent: args.has_flag("permanent"),
        force: args.has_flag("force"),
    };

    if args.trash && args.permanent {
        return Ok(ActionStream::one(Err(ShellError::labeled_error(
            "only one of --permanent and --trash can be used",
            "conflicting flags",
            name,
        ))));
    }

    shell_manager.rm(args, name)
}

#[cfg(test)]
mod tests {
    use super::Remove;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Remove {})
    }
}
