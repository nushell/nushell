use super::{operate, PathSubcommandArguments};
use crate::prelude::*;
use nu_engine::filesystem::path::absolutize;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Span;
use std::env::current_dir;
use std::path::Path;

pub struct PathExpand;

struct PathExpandArguments {
    strict: bool,
    rest: Vec<ColumnPath>,
}

impl PathSubcommandArguments for PathExpandArguments {
    fn get_column_paths(&self) -> &Vec<ColumnPath> {
        &self.rest
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
                "throw an error if the path could not be expanded",
                Some('s'),
            )
            .rest(SyntaxShape::ColumnPath, "Optionally operate by column path")
    }

    fn usage(&self) -> &str {
        "Expand a path to its absolute form"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let args = args.evaluate_once()?;
        let cmd_args = Arc::new(PathExpandArguments {
            strict: args.has_flag("strict"),
            rest: args.rest(0)?,
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
                description: "Expand a relative path",
                example: r"'foo\..\bar' | path expand",
                result: None, // don't know where cwd is
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
                description: "Expand a relative path",
                example: "'foo/../bar' | path expand",
                result: None, // don't know where cwd is
            },
        ]
    }
}

fn action(path: &Path, tag: Tag, args: &PathExpandArguments) -> Value {
    let ps = path.to_string_lossy();
    let expanded = shellexpand::tilde(&ps);
    let path: &Path = expanded.as_ref().as_ref();

    if let Ok(p) = dunce::canonicalize(path) {
        UntaggedValue::filepath(p).into_value(tag)
    } else if args.strict {
        Value::error(ShellError::labeled_error(
            "Could not expand path",
            "could not be expanded (path might not exist, non-final \
                component is not a directory, or another cause)",
            tag.span,
        ))
    } else {
        match current_dir() {
            Ok(cwd) => UntaggedValue::filepath(absolutize(cwd, path)).into_value(tag),
            Err(_) => Value::error(ShellError::untagged_runtime_error(
                "Could not find current working directory. \
                It might not exist or has insufficient permissions.",
            )),
        }
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
