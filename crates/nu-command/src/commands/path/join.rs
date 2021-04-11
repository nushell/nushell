use super::{handle_value, join_path, PathSubcommandArguments};
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use std::path::Path;

pub struct PathJoin;

struct PathJoinArguments {
    rest: Vec<ColumnPath>,
    appendix: Option<Tagged<String>>,
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
                "appendix",
                SyntaxShape::String,
                "Path to append to the input",
                Some('a'),
            )
    }

    fn usage(&self) -> &str {
        "Join a structured path or a list of path parts."
    }

    fn extra_usage(&self) -> &str {
        "Optionally, append additional to the result. It is designed to accept the output of 'path
parse' and 'path split' subdommands."
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let args = args.evaluate_once()?;
        let cmd_args = Arc::new(PathJoinArguments {
            rest: args.rest_args()?,
            appendix: args.get_flag("appendix").transpose()?
        });

        Ok(operate_join(args.input, &action, tag, cmd_args))
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Append a filename to a path",
            example: "echo 'C:\\Users\\viking' | path join -a spam.txt",
            result: Some(vec![Value::from(UntaggedValue::filepath(
                "C:\\Users\\viking\\spam.txt",
            ))]),
        }]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Append a filename to a path",
            example: "echo '/home/viking' | path join -a spam.txt",
            result: Some(vec![Value::from(UntaggedValue::filepath(
                "/home/viking/spam.txt",
            ))]),
        }]
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
    F: Fn(&Path, Tag, &T) -> Result<Value, ShellError> + Send + Sync + 'static,
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
                .map(move |v| {
                    ReturnSuccess::value(handle_value(&action, &v, span, Arc::clone(&args))?)
                })
                .to_output_stream()
        } else {
            // join the whole input stream
            match join_path(&parts.collect_vec()) {
                Ok(path_buf) => {
                    let joined_value = UntaggedValue::filepath(path_buf).into_value(&tag);
                    OutputStream::one(
                        handle_value(&action, &joined_value, span, Arc::clone(&args))
                            .and_then(|v| ReturnSuccess::value(v))
                    )
                },
                Err(err) => OutputStream::one(Err(err)),
            }
        }
    } else {
        input
            .map(move |v| {
                let mut ret = v;

                for path in args.get_column_paths() {
                    let cloned_args = Arc::clone(&args);
                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| handle_value(&action, &old, span, cloned_args)),
                    )?;
                }

                ReturnSuccess::value(ret)
            })
            .to_output_stream()
    }
}

#[allow(clippy::unnecessary_wraps)]
fn action(path: &Path, tag: Tag, args: &PathJoinArguments) -> Result<Value, ShellError> {
    if let Some(ref appendix) = args.appendix {
        Ok(UntaggedValue::filepath(path.join(&appendix.item)).into_value(tag))
    } else {
        Ok(UntaggedValue::filepath(path).into_value(tag))
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
