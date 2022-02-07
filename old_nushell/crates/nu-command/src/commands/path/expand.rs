use super::{column_paths_from_args, operate, PathSubcommandArguments};
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_path::{canonicalize, expand_path};
use nu_protocol::{ColumnPath, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Span;
use std::path::Path;

pub struct PathExpand;

struct PathExpandArguments {
    strict: bool,
    columns: Vec<ColumnPath>,
}

impl PathSubcommandArguments for PathExpandArguments {
    fn get_column_paths(&self) -> &Vec<ColumnPath> {
        &self.columns
    }
}

impl WholeStreamCommand for PathExpand {
    fn name(&self) -> &str {
        "path expand"
    }

    fn signature(&self) -> Signature {
        Signature::build("path expand")
            .switch(
                "strict",
                "Throw an error if the path could not be expanded",
                Some('s'),
            )
            .named(
                "columns",
                SyntaxShape::Table,
                "Optionally operate by column path",
                Some('c'),
            )
    }

    fn usage(&self) -> &str {
        "Try to expand a path to its absolute form"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let cmd_args = Arc::new(PathExpandArguments {
            strict: args.has_flag("strict"),
            columns: column_paths_from_args(&args)?,
        });

        Ok(operate(args.input, &action, tag.span, cmd_args))
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Expand an absolute path",
                example: r"'C:\Users\joe\foo\..\bar' | path expand",
                result: Some(vec![
                    UntaggedValue::filepath(r"C:\Users\joe\bar").into_value(Span::new(0, 25))
                ]),
            },
            Example {
                description: "Expand a path in a column",
                example: "ls | path expand -c [ name ]",
                result: None,
            },
            Example {
                description: "Expand a relative path",
                example: r"'foo\..\bar' | path expand",
                result: Some(vec![
                    UntaggedValue::filepath("bar").into_value(Span::new(0, 12))
                ]),
            },
        ]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Expand an absolute path",
                example: "'/home/joe/foo/../bar' | path expand",
                result: Some(vec![
                    UntaggedValue::filepath("/home/joe/bar").into_value(Span::new(0, 22))
                ]),
            },
            Example {
                description: "Expand a path in a column",
                example: "ls | path expand -c [ name ]",
                result: None,
            },
            Example {
                description: "Expand a relative path",
                example: "'foo/../bar' | path expand",
                result: Some(vec![
                    UntaggedValue::filepath("bar").into_value(Span::new(0, 12))
                ]),
            },
        ]
    }
}

fn action(path: &Path, tag: Tag, args: &PathExpandArguments) -> Value {
    if let Ok(p) = canonicalize(path) {
        UntaggedValue::filepath(p).into_value(tag)
    } else if args.strict {
        Value::error(ShellError::labeled_error(
            "Could not expand path",
            "could not be expanded (path might not exist, non-final \
                    component is not a directory, or other cause)",
            tag.span,
        ))
    } else {
        UntaggedValue::filepath(expand_path(path)).into_value(tag)
    }
}

#[cfg(test)]
mod tests {
    use super::PathExpand;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(PathExpand {})
    }
}
