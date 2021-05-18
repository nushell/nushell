use super::{handle_value, join_path, operate_column_paths, PathSubcommandArguments};
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use std::path::{Path, PathBuf};

pub struct PathJoin;

struct PathJoinArguments {
    rest: Vec<ColumnPath>,
    append: Option<Tagged<PathBuf>>,
}

impl PathSubcommandArguments for PathJoinArguments {
    fn get_column_paths(&self) -> &Vec<ColumnPath> {
        &self.rest
    }
}

impl WholeStreamCommand for PathJoin {
    fn name(&self) -> &str {
        "path join"
    }

    fn signature(&self) -> Signature {
        Signature::build("path join")
            .rest(SyntaxShape::ColumnPath, "Optionally operate by column path")
            .named(
                "append",
                SyntaxShape::FilePath,
                "Path to append to the input",
                Some('a'),
            )
    }

    fn usage(&self) -> &str {
        "Join a structured path or a list of path parts."
    }

    fn extra_usage(&self) -> &str {
        r#"Optionally, append an additional path to the result. It is designed to accept
the output of 'path parse' and 'path split' subcommands."#
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let args = args.evaluate_once()?;
        let cmd_args = Arc::new(PathJoinArguments {
            rest: args.rest_args()?,
            append: args.get_flag("append")?,
        });

        Ok(operate_join(args.input, &action, tag, cmd_args))
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Append a filename to a path",
                example: r"echo 'C:\Users\viking' | path join -a spam.txt",
                result: Some(vec![Value::from(UntaggedValue::filepath(
                    r"C:\Users\viking\spam.txt",
                ))]),
            },
            Example {
                description: "Join a list of parts into a path",
                example: r"echo [ 'C:' '\' 'Users' 'viking' 'spam.txt' ] | path join",
                result: Some(vec![Value::from(UntaggedValue::filepath(
                    r"C:\Users\viking\spam.txt",
                ))]),
            },
            Example {
                description: "Join a structured path into a path",
                example: r"echo [ [parent stem extension]; ['C:\Users\viking' 'spam' 'txt']] | path join",
                result: Some(vec![Value::from(UntaggedValue::filepath(
                    r"C:\Users\viking\spam.txt",
                ))]),
            },
        ]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Append a filename to a path",
                example: r"echo '/home/viking' | path join -a spam.txt",
                result: Some(vec![Value::from(UntaggedValue::filepath(
                    r"/home/viking/spam.txt",
                ))]),
            },
            Example {
                description: "Join a list of parts into a path",
                example: r"echo [ '/' 'home' 'viking' 'spam.txt' ] | path join",
                result: Some(vec![Value::from(UntaggedValue::filepath(
                    r"/home/viking/spam.txt",
                ))]),
            },
            Example {
                description: "Join a structured path into a path",
                example: r"echo [[ parent stem extension ]; [ '/home/viking' 'spam' 'txt' ]] | path join",
                result: Some(vec![Value::from(UntaggedValue::filepath(
                    r"/home/viking/spam.txt",
                ))]),
            },
        ]
    }
}

fn operate_join<F, T>(
    input: crate::InputStream,
    action: &'static F,
    tag: Tag,
    args: Arc<T>,
) -> OutputStream
where
    T: PathSubcommandArguments + Send + Sync + 'static,
    F: Fn(&Path, Tag, &T) -> Value + Send + Sync + 'static,
{
    let span = tag.span;

    if args.get_column_paths().is_empty() {
        let mut parts = input.peekable();
        let has_rows = matches!(
            parts.peek(),
            Some(&Value {
                value: UntaggedValue::Row(_),
                ..
            })
        );

        if has_rows {
            // operate one-by-one like the other path subcommands
            parts
                .into_iter()
                .map(
                    move |v| match handle_value(&action, &v, span, Arc::clone(&args)) {
                        Ok(v) => v,
                        Err(e) => Value::error(e),
                    },
                )
                .to_output_stream()
        } else {
            // join the whole input stream
            match join_path(&parts.collect_vec(), &span) {
                Ok(path_buf) => OutputStream::one(action(&path_buf, tag, &args)),
                Err(e) => OutputStream::one(Value::error(e)),
            }
        }
    } else {
        operate_column_paths(input, action, span, args)
    }
}

fn action(path: &Path, tag: Tag, args: &PathJoinArguments) -> Value {
    if let Some(ref append) = args.append {
        UntaggedValue::filepath(path.join(&append.item)).into_value(tag)
    } else {
        UntaggedValue::filepath(path).into_value(tag)
    }
}

#[cfg(test)]
mod tests {
    use super::PathJoin;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(PathJoin {})
    }
}
