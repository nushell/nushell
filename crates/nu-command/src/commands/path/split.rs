use super::{column_paths_from_args, handle_value, operate_column_paths, PathSubcommandArguments};
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Signature, SyntaxShape, UntaggedValue, Value};
use std::path::Path;

pub struct PathSplit;

struct PathSplitArguments {
    columns: Vec<ColumnPath>,
}

impl PathSubcommandArguments for PathSplitArguments {
    fn get_column_paths(&self) -> &Vec<ColumnPath> {
        &self.columns
    }
}

impl WholeStreamCommand for PathSplit {
    fn name(&self) -> &str {
        "path split"
    }

    fn signature(&self) -> Signature {
        Signature::build("path split").named(
            "columns",
            SyntaxShape::Table,
            "Optionally operate by column path",
            Some('c'),
        )
    }

    fn usage(&self) -> &str {
        "Split a path into parts by a separator."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let cmd_args = Arc::new(PathSplitArguments {
            columns: column_paths_from_args(&args)?,
        });

        Ok(operate_split(args.input, &action, tag.span, cmd_args))
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Split a path into parts",
                example: r"'C:\Users\viking\spam.txt' | path split",
                result: Some(vec![
                    Value::from(UntaggedValue::string("C:")),
                    Value::from(UntaggedValue::string(r"\")),
                    Value::from(UntaggedValue::string("Users")),
                    Value::from(UntaggedValue::string("viking")),
                    Value::from(UntaggedValue::string("spam.txt")),
                ]),
            },
            Example {
                description: "Split all paths under the 'name' column",
                example: r"ls ('.' | path expand) | path split -c [ name ]",
                result: None,
            },
        ]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Split a path into parts",
                example: r"'/home/viking/spam.txt' | path split",
                result: Some(vec![
                    Value::from(UntaggedValue::string("/")),
                    Value::from(UntaggedValue::string("home")),
                    Value::from(UntaggedValue::string("viking")),
                    Value::from(UntaggedValue::string("spam.txt")),
                ]),
            },
            Example {
                description: "Split all paths under the 'name' column",
                example: r"ls ('.' | path expand) | path split -c [ name ]",
                result: None,
            },
        ]
    }
}

fn operate_split<F, T>(
    input: crate::InputStream,
    action: &'static F,
    span: Span,
    args: Arc<T>,
) -> OutputStream
where
    T: PathSubcommandArguments + Send + Sync + 'static,
    F: Fn(&Path, Tag, &T) -> Value + Send + Sync + 'static,
{
    if args.get_column_paths().is_empty() {
        // Do not wrap result into a table
        input
            .flat_map(move |v| {
                let split_result = handle_value(&action, &v, span, Arc::clone(&args));

                match split_result {
                    Ok(Value {
                        value: UntaggedValue::Table(parts),
                        ..
                    }) => parts.into_iter().into_output_stream(),
                    Err(e) => OutputStream::one(Value::error(e)),
                    _ => OutputStream::one(Value::error(ShellError::labeled_error(
                        "Internal Error",
                        "unexpected result from the split function",
                        span,
                    ))),
                }
            })
            .into_output_stream()
    } else {
        operate_column_paths(input, action, span, args)
    }
}

fn action(path: &Path, tag: Tag, _args: &PathSplitArguments) -> Value {
    let parts: Vec<Value> = path
        .components()
        .map(|comp| {
            let s = comp.as_os_str().to_string_lossy();
            UntaggedValue::string(s).into_value(&tag)
        })
        .collect();

    UntaggedValue::table(&parts).into_value(tag)
}

#[cfg(test)]
mod tests {
    use super::PathSplit;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(PathSplit {})
    }
}
