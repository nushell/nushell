use super::operate;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use std::path::Path;

pub struct PathBasename;

#[derive(Deserialize)]
struct PathBasenameArguments {
    replace: Option<Tagged<String>>,
    rest: Vec<ColumnPath>,
}

#[async_trait]
impl WholeStreamCommand for PathBasename {
    fn name(&self) -> &str {
        "path basename"
    }

    fn signature(&self) -> Signature {
        Signature::build("path basename")
            .named(
                "replace",
                SyntaxShape::String,
                "Replace extension with this string",
                Some('r'),
            )
            .rest(SyntaxShape::ColumnPath, "optionally operate by path")
    }

    fn usage(&self) -> &str {
        "gets the filename of a path"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let (PathBasenameArguments { replace, rest }, input) =
            args.process(&registry).await?;
        let arg = Arc::new(replace.map(|v| v.item));
        operate(input, rest, &action, tag.span, arg).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get basename of a path",
                example: "echo '/home/joe/test.txt' | path basename",
                result: Some(vec![Value::from("test.txt")]),
            },
            Example {
                description: "Replace basename of a path",
                example: "echo '/home/joe/test.txt' | path basename -r 'spam.png'",
                result: Some(vec![Value::from("/home/joe/spam.png")]),
            },
        ]
    }
}

fn action(path: &Path, replace_with: Arc<Option<String>>) -> UntaggedValue {
    match &*replace_with {
        Some(basename) => {
            UntaggedValue::string(
                path.with_file_name(basename).to_string_lossy()
            )
        },
        None => {
            UntaggedValue::string(match path.file_name() {
                Some(filename) => filename.to_string_lossy(),
                _ => "".into(),
            })
        },
    }
}

#[cfg(test)]
mod tests {
    use super::PathBasename;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        Ok(test_examples(PathBasename {})?)
    }
}
