use super::{operate, PathSubcommandArguments};
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use std::path::Path;

pub struct PathBasename;

#[derive(Deserialize)]
struct PathBasenameArguments {
    replace: Option<Tagged<String>>,
    rest: Vec<ColumnPath>,
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
            .named(
                "replace",
                SyntaxShape::String,
                "Return original path with basename replaced by this string",
                Some('r'),
            )
            .rest(SyntaxShape::ColumnPath, "Optionally operate by column path")
    }

    fn usage(&self) -> &str {
        "Gets the final component of a path"
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let (PathBasenameArguments { replace, rest }, input) = args.process()?;
        let args = Arc::new(PathBasenameArguments { replace, rest });
        Ok(operate(input, &action, tag.span, args))
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

#[allow(clippy::unnecessary_wraps)]
fn action(path: &Path, tag: Tag, args: &PathBasenameArguments) -> Result<Value, ShellError> {
    let untagged = match args.replace {
        Some(ref basename) => UntaggedValue::filepath(path.with_file_name(&basename.item)),
        None => UntaggedValue::string(match path.file_name() {
            Some(filename) => filename.to_string_lossy(),
            None => "".into(),
        }),
    };

    Ok(untagged.into_value(tag))
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
