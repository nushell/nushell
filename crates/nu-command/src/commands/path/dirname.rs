use super::{operate, PathSubcommandArguments};
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use std::path::Path;

pub struct PathDirname;

#[derive(Deserialize)]
struct PathDirnameArguments {
    replace: Option<Tagged<String>>,
    #[serde(rename = "num-levels")]
    num_levels: Option<Tagged<u32>>,
    rest: Vec<ColumnPath>,
}

impl PathSubcommandArguments for PathDirnameArguments {
    fn get_column_paths(&self) -> &Vec<ColumnPath> {
        &self.rest
    }
}

impl WholeStreamCommand for PathDirname {
    fn name(&self) -> &str {
        "path dirname"
    }

    fn signature(&self) -> Signature {
        Signature::build("path dirname")
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
            .rest(SyntaxShape::ColumnPath, "Optionally operate by column path")
    }

    fn usage(&self) -> &str {
        "Gets the parent directory of a path"
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let (
            PathDirnameArguments {
                replace,
                num_levels,
                rest,
            },
            input,
        ) = args.process()?;
        let args = Arc::new(PathDirnameArguments {
            replace,
            num_levels,
            rest,
        });
        Ok(operate(input, &action, tag.span, args))
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get dirname of a path",
                example: "echo 'C:\\Users\\joe\\code\\test.txt' | path dirname",
                result: Some(vec![Value::from(UntaggedValue::filepath(
                    "C:\\Users\\joe\\code",
                ))]),
            },
            Example {
                description: "Set how many levels up to skip",
                example: "echo 'C:\\Users\\joe\\code\\test.txt' | path dirname -n 2",
                result: Some(vec![Value::from(UntaggedValue::filepath("C:\\Users\\joe"))]),
            },
            Example {
                description: "Replace the part that would be returned with custom string",
                example:
                    "echo 'C:\\Users\\joe\\code\\test.txt' | path dirname -n 2 -r C:\\Users\\viking",
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
                example: "echo '/home/joe/code/test.txt' | path dirname",
                result: Some(vec![Value::from(UntaggedValue::filepath("/home/joe/code"))]),
            },
            Example {
                description: "Set how many levels up to skip",
                example: "echo '/home/joe/code/test.txt' | path dirname -n 2",
                result: Some(vec![Value::from(UntaggedValue::filepath("/home/joe"))]),
            },
            Example {
                description: "Replace the part that would be returned with custom string",
                example: "echo '/home/joe/code/test.txt' | path dirname -n 2 -r /home/viking",
                result: Some(vec![Value::from(UntaggedValue::filepath(
                    "/home/viking/code/test.txt",
                ))]),
            },
        ]
    }
}

#[allow(clippy::unnecessary_wraps)]
fn action(path: &Path, tag: Tag, args: &PathDirnameArguments) -> Result<Value, ShellError> {
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

    Ok(untagged.into_value(tag))
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
