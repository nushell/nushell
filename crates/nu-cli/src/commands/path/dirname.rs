use super::{operate, DefaultArguments};
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use std::path::Path;

pub struct PathDirname;

#[derive(Deserialize)]
struct PathDirnameArguments {
    replace: Option<Tagged<String>>,
    #[serde(rename = "num-leveles")]
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
                "Replace extension with this string",
                Some('r'),
            )
            .named(
                "num-levels",
                SyntaxShape::Int,
                "Number of directories to walk up",
                Some('n'),
            )
            .rest(SyntaxShape::ColumnPath, "optionally operate by path")
    }

    fn usage(&self) -> &str {
        "gets the dirname of a path"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let (PathDirnameArguments { replace, num_levels, rest }, input) =
            args.process(&registry).await?;
        let args = Arc::new(DefaultArguments {
            replace: replace.map(|v| v.item),
            extension: None,
            num_levels: num_levels.map(|v| v.item),
            paths: rest,
        });
        operate(input, &action, tag.span, args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get dirname of a path",
                example: "echo '/home/joe/code/test.txt' | path dirname",
                result: Some(vec![Value::from("/home/joe/code")]),
            },
            Example {
                description: "Set how many levels up to skip",
                example: "echo '/home/joe/code/test.txt' | path dirname -n 2",
                result: Some(vec![Value::from("/home/joe")]),
            },
            Example {
                description: "Replace the part that would be returned with custom string",
                example: "echo '/home/joe/code/test.txt' | path dirname -n 2 -r /home/viking",
                result: Some(vec![Value::from("/home/viking/code/test.txt")]),
            },
        ]
    }
}

fn action(path: &Path, args: Arc<DefaultArguments>) -> UntaggedValue {
    let num_levels = args.num_levels.unwrap_or(1);

    let mut dirname = path;
    let mut reached_top = false;  // end early if somebody passes -n 99999999
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
            UntaggedValue::string(Path::new(newdir).join(remainder).to_string_lossy())
        },
        None => UntaggedValue::string(dirname.to_string_lossy()),
    }
}

#[cfg(test)]
mod tests {
    use super::PathDirname;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        Ok(test_examples(PathDirname {})?)
    }
}
