use super::{column_paths_from_args, operate, PathSubcommandArguments};
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use std::path::{Path, PathBuf};

pub struct PathRelativeTo;

struct PathRelativeToArguments {
    path: Tagged<PathBuf>,
    columns: Vec<ColumnPath>,
}

impl PathSubcommandArguments for PathRelativeToArguments {
    fn get_column_paths(&self) -> &Vec<ColumnPath> {
        &self.columns
    }
}

impl WholeStreamCommand for PathRelativeTo {
    fn name(&self) -> &str {
        "path relative-to"
    }

    fn signature(&self) -> Signature {
        Signature::build("path relative-to")
            .required(
                "path",
                SyntaxShape::FilePath,
                "Parent shared with the input path",
            )
            .named(
                "columns",
                SyntaxShape::Table,
                "Optionally operate by column path",
                Some('c'),
            )
    }

    fn usage(&self) -> &str {
        "Get a path as relative to another path."
    }

    fn extra_usage(&self) -> &str {
        r#"Can be used only when the input and the argument paths are either both
absolute or both relative. The argument path needs to be a parent of the input
path."#
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let cmd_args = Arc::new(PathRelativeToArguments {
            path: args.req(0)?,
            columns: column_paths_from_args(&args)?,
        });

        Ok(operate(args.input, &action, tag.span, cmd_args))
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Find a relative path from two absolute paths",
                example: r"'C:\Users\viking' | path relative-to 'C:\Users'",
                result: Some(vec![Value::from(UntaggedValue::filepath(r"viking"))]),
            },
            Example {
                description: "Find a relative path from two absolute paths in a column",
                example: "ls ~ | path relative-to ~ -c [ name ]",
                result: None,
            },
            Example {
                description: "Find a relative path from two relative paths",
                example: r"'eggs\bacon\sausage\spam' | path relative-to 'eggs\bacon\sausage'",
                result: Some(vec![Value::from(UntaggedValue::filepath(r"spam"))]),
            },
        ]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Find a relative path from two absolute paths",
                example: r"'/home/viking' | path relative-to '/home'",
                result: Some(vec![Value::from(UntaggedValue::filepath(r"viking"))]),
            },
            Example {
                description: "Find a relative path from two absolute paths in a column",
                example: "ls ~ | path relative-to ~ -c [ name ]",
                result: None,
            },
            Example {
                description: "Find a relative path from two relative paths",
                example: r"'eggs/bacon/sausage/spam' | path relative-to 'eggs/bacon/sausage'",
                result: Some(vec![Value::from(UntaggedValue::filepath(r"spam"))]),
            },
        ]
    }
}

fn action(path: &Path, tag: Tag, args: &PathRelativeToArguments) -> Value {
    match path.strip_prefix(&args.path.item) {
        Ok(p) => UntaggedValue::filepath(p).into_value(tag),
        Err(_) => Value::error(ShellError::labeled_error_with_secondary(
            format!(
                "'{}' is not a subpath of '{}'",
                path.to_string_lossy(),
                &args.path.item.to_string_lossy()
            ),
            "should be a parent of the input path",
            args.path.tag.span,
            "originates from here",
            tag.span,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::PathRelativeTo;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(PathRelativeTo {})
    }
}
