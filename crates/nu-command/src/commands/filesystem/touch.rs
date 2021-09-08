use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tagged;
use std::fs::OpenOptions;
use std::path::PathBuf;

pub struct Touch;

impl WholeStreamCommand for Touch {
    fn name(&self) -> &str {
        "touch"
    }
    fn signature(&self) -> Signature {
        Signature::build("touch")
            .required(
                "filename",
                SyntaxShape::FilePath,
                "the path of the file you want to create",
            )
            .rest("rest", SyntaxShape::FilePath, "additional files to create")
    }
    fn usage(&self) -> &str {
        "Creates one or more files."
    }
    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        touch(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Creates \"fixture.json\"",
                example: "touch fixture.json",
                result: None,
            },
            Example {
                description: "Creates files a, b and c",
                example: "touch a b c",
                result: None,
            },
        ]
    }
}

fn touch(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let target: Tagged<PathBuf> = args.req(0)?;
    let rest: Vec<Tagged<PathBuf>> = args.rest(1)?;

    for item in vec![target].into_iter().chain(rest) {
        match OpenOptions::new().write(true).create(true).open(&item) {
            Ok(_) => continue,
            Err(err) => {
                return Err(ShellError::labeled_error(
                    "File Error",
                    err.to_string(),
                    &item.tag,
                ))
            }
        }
    }

    Ok(ActionStream::empty())
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::Touch;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Touch {})
    }
}
