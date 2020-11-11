use super::operate;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use std::path::Path;

pub struct PathExtension;

#[derive(Deserialize)]
struct PathExtensionArguments {
    replace: Option<Tagged<String>>,
    rest: Vec<ColumnPath>,
}

#[async_trait]
impl WholeStreamCommand for PathExtension {
    fn name(&self) -> &str {
        "path extension"
    }

    fn signature(&self) -> Signature {
        Signature::build("path extension")
            .named(
                "replace",
                SyntaxShape::String,
                "Replace extension with this string",
                Some('r'),
            )
            .rest(SyntaxShape::ColumnPath, "optionally operate by path")
    }

    fn usage(&self) -> &str {
        "gets the extension of a path"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let (PathExtensionArguments { replace, rest }, input) =
            args.process(&registry).await?;
        let arg = Arc::new(replace.map(|v| v.item));
        operate(input, rest, &action, tag.span, arg).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get extension of a path",
                example: "echo 'test.txt' | path extension",
                result: Some(vec![Value::from("txt")]),
            },
            Example {
                description: "You get an empty string if there is no extension",
                example: "echo 'test' | path extension",
                result: Some(vec![Value::from("")]),
            },
            Example {
                description: "Replace an extension with a custom string",
                example: "echo 'test.txt' | path extension -r md",
                result: Some(vec![Value::from("test.md")]),
            },
            Example {
                description: "To replace more complex extensions:",
                example: "echo 'test.tar.gz' | path filestem | path extension -r txt",
                result: Some(vec![Value::from("test.txt")]),
            },
        ]
    }
}

fn action(path: &Path, replace_with: Arc<Option<String>>) -> UntaggedValue {
    match &*replace_with {
        Some(extension) => {
            UntaggedValue::string(
                path.with_extension(extension).to_string_lossy()
            )
        },
        None => {
            UntaggedValue::string(match path.extension() {
                Some(extension) => extension.to_string_lossy(),
                _ => "".into(),
            })
        },
    }
}

#[cfg(test)]
mod tests {
    use super::PathExtension;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        Ok(test_examples(PathExtension {})?)
    }
}
