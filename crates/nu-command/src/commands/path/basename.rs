use super::{operate, PathSubcommandArguments};
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use std::path::Path;

pub struct PathBasename;

struct PathBasenameArguments {
    rest: Vec<ColumnPath>,
    replace: Option<Tagged<String>>,
}

impl PathSubcommandArguments for PathBasenameArguments {
    fn get_column_paths(&self) -> &Vec<ColumnPath> {
        &self.rest
    }
}

impl WholeStreamCommand for PathBasename {
    fn name(&self) -> &str {
        "path basename"
    }

    fn signature(&self) -> Signature {
        Signature::build("path basename")
            .rest(SyntaxShape::ColumnPath, "Optionally operate by column path")
            .named(
                "replace",
                SyntaxShape::String,
                "Return original path with basename replaced by this string",
                Some('r'),
            )
    }

    fn usage(&self) -> &str {
        "Get the final component of a path"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let cmd_args = Arc::new(PathBasenameArguments {
            rest: args.rest(0)?,
            replace: args.get_flag("replace")?,
        });

        Ok(operate(args.input, &action, tag.span, cmd_args))
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get basename of a path",
                example: "echo 'C:\\Users\\joe\\test.txt' | path basename",
                result: Some(vec![Value::from("test.txt")]),
            },
            Example {
                description: "Replace basename of a path",
                example: "echo 'C:\\Users\\joe\\test.txt' | path basename -r 'spam.png'",
                result: Some(vec![Value::from(UntaggedValue::filepath(
                    "C:\\Users\\joe\\spam.png",
                ))]),
            },
        ]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get basename of a path",
                example: "echo '/home/joe/test.txt' | path basename",
                result: Some(vec![Value::from("test.txt")]),
            },
            Example {
                description: "Replace basename of a path",
                example: "echo '/home/joe/test.txt' | path basename -r 'spam.png'",
                result: Some(vec![Value::from(UntaggedValue::filepath(
                    "/home/joe/spam.png",
                ))]),
            },
        ]
    }
}

fn action(path: &Path, tag: Tag, args: &PathBasenameArguments) -> Value {
    let untagged = match args.replace {
        Some(ref basename) => UntaggedValue::filepath(path.with_file_name(&basename.item)),
        None => UntaggedValue::string(match path.file_name() {
            Some(filename) => filename.to_string_lossy(),
            None => "".into(),
        }),
    };

    untagged.into_value(tag)
}

#[cfg(test)]
mod tests {
    use super::PathBasename;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(PathBasename {})
    }
}
