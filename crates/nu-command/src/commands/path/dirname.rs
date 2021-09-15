use super::{column_paths_from_args, operate, PathSubcommandArguments};
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use std::path::Path;

pub struct PathDirname;

struct PathDirnameArguments {
    columns: Vec<ColumnPath>,
    replace: Option<Tagged<String>>,
    num_levels: Option<Tagged<u32>>,
}

impl PathSubcommandArguments for PathDirnameArguments {
    fn get_column_paths(&self) -> &Vec<ColumnPath> {
        &self.columns
    }
}

impl WholeStreamCommand for PathDirname {
    fn name(&self) -> &str {
        "path dirname"
    }

    fn signature(&self) -> Signature {
        Signature::build("path dirname")
            .named(
                "columns",
                SyntaxShape::Table,
                "Optionally operate by column path",
                Some('c'),
            )
            .named(
                "replace",
                SyntaxShape::String,
                "Return original path with dirname replaced by this string",
                Some('r'),
            )
            .named(
                "num-levels",
                SyntaxShape::Int,
                "Number of directories to walk up",
                Some('n'),
            )
    }

    fn usage(&self) -> &str {
        "Get the parent directory of a path"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let cmd_args = Arc::new(PathDirnameArguments {
            columns: column_paths_from_args(&args)?,
            replace: args.get_flag("replace")?,
            num_levels: args.get_flag("num-levels")?,
        });

        Ok(operate(args.input, &action, tag.span, cmd_args))
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get dirname of a path",
                example: "'C:\\Users\\joe\\code\\test.txt' | path dirname",
                result: Some(vec![Value::from(UntaggedValue::filepath(
                    "C:\\Users\\joe\\code",
                ))]),
            },
            Example {
                description: "Get dirname of a path in a column",
                example: "ls ('.' | path expand) | path dirname -c [ name ]",
                result: None,
            },
            Example {
                description: "Walk up two levels",
                example: "'C:\\Users\\joe\\code\\test.txt' | path dirname -n 2",
                result: Some(vec![Value::from(UntaggedValue::filepath("C:\\Users\\joe"))]),
            },
            Example {
                description: "Replace the part that would be returned with a custom path",
                example:
                    "'C:\\Users\\joe\\code\\test.txt' | path dirname -n 2 -r C:\\Users\\viking",
                result: Some(vec![Value::from(UntaggedValue::filepath(
                    "C:\\Users\\viking\\code\\test.txt",
                ))]),
            },
        ]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get dirname of a path",
                example: "'/home/joe/code/test.txt' | path dirname",
                result: Some(vec![Value::from(UntaggedValue::filepath("/home/joe/code"))]),
            },
            Example {
                description: "Get dirname of a path in a column",
                example: "ls ('.' | path expand) | path dirname -c [ name ]",
                result: None,
            },
            Example {
                description: "Walk up two levels",
                example: "'/home/joe/code/test.txt' | path dirname -n 2",
                result: Some(vec![Value::from(UntaggedValue::filepath("/home/joe"))]),
            },
            Example {
                description: "Replace the part that would be returned with a custom path",
                example: "'/home/joe/code/test.txt' | path dirname -n 2 -r /home/viking",
                result: Some(vec![Value::from(UntaggedValue::filepath(
                    "/home/viking/code/test.txt",
                ))]),
            },
        ]
    }
}

fn action(path: &Path, tag: Tag, args: &PathDirnameArguments) -> Value {
    let num_levels = args.num_levels.as_ref().map_or(1, |tagged| tagged.item);

    let mut dirname = path;
    let mut reached_top = false; // end early if somebody passes -n 99999999
    for _ in 0..num_levels {
        dirname = dirname.parent().unwrap_or_else(|| {
            reached_top = true;
            dirname
        });
        if reached_top {
            break;
        }
    }

    let untagged = match args.replace {
        Some(ref newdir) => {
            let remainder = path.strip_prefix(dirname).unwrap_or(dirname);
            if !remainder.as_os_str().is_empty() {
                UntaggedValue::filepath(Path::new(&newdir.item).join(remainder))
            } else {
                UntaggedValue::filepath(Path::new(&newdir.item))
            }
        }
        None => UntaggedValue::filepath(dirname),
    };

    untagged.into_value(tag)
}

#[cfg(test)]
mod tests {
    use super::PathDirname;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(PathDirname {})
    }
}
