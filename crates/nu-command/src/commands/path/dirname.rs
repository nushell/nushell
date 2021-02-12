use super::{operate, DefaultArguments};
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

#[async_trait]
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let (
            PathDirnameArguments {
                replace,
                num_levels,
                rest,
            },
            input,
        ) = args.process().await?;
        let args = Arc::new(DefaultArguments {
            replace: replace.map(|v| v.item),
            prefix: None,
            suffix: None,
            num_levels: num_levels.map(|v| v.item),
            paths: rest,
        });
        operate(input, &action, tag.span, args).await
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

fn action(path: &Path, args: Arc<DefaultArguments>) -> UntaggedValue {
    let num_levels = args.num_levels.unwrap_or(1);

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

    match args.replace {
        Some(ref newdir) => {
            let remainder = path.strip_prefix(dirname).unwrap_or(dirname);
            if !remainder.as_os_str().is_empty() {
                UntaggedValue::filepath(Path::new(newdir).join(remainder))
            } else {
                UntaggedValue::filepath(Path::new(newdir))
            }
        }
        None => UntaggedValue::filepath(dirname),
    }
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
