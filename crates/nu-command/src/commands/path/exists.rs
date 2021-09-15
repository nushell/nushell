use super::{column_paths_from_args, operate, PathSubcommandArguments};
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Signature, SyntaxShape, UntaggedValue, Value};
use std::path::Path;

pub struct PathExists;

struct PathExistsArguments {
    columns: Vec<ColumnPath>,
}

impl PathSubcommandArguments for PathExistsArguments {
    fn get_column_paths(&self) -> &Vec<ColumnPath> {
        &self.columns
    }
}

impl WholeStreamCommand for PathExists {
    fn name(&self) -> &str {
        "path exists"
    }

    fn signature(&self) -> Signature {
        Signature::build("path exists").named(
            "columns",
            SyntaxShape::Table,
            "Optionally operate by column path",
            Some('c'),
        )
    }

    fn usage(&self) -> &str {
        "Check whether a path exists"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let cmd_args = Arc::new(PathExistsArguments {
            columns: column_paths_from_args(&args)?,
        });

        Ok(operate(args.input, &action, tag.span, cmd_args))
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Check if a file exists",
                example: "'C:\\Users\\joe\\todo.txt' | path exists",
                result: Some(vec![Value::from(UntaggedValue::boolean(false))]),
            },
            Example {
                description: "Check if a file exists in a column",
                example: "ls | path exists -c [ name ]",
                result: None,
            },
        ]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Check if a file exists",
                example: "'/home/joe/todo.txt' | path exists",
                result: Some(vec![Value::from(UntaggedValue::boolean(false))]),
            },
            Example {
                description: "Check if a file exists in a column",
                example: "ls | path exists -c [ name ]",
                result: None,
            },
        ]
    }
}

fn action(path: &Path, tag: Tag, _args: &PathExistsArguments) -> Value {
    UntaggedValue::boolean(path.exists()).into_value(tag)
}

#[cfg(test)]
mod tests {
    use super::PathExists;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(PathExists {})
    }
}
