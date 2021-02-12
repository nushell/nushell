use super::{operate, DefaultArguments};
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
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
                "Return original path with extension replaced by this string",
                Some('r'),
            )
            .rest(SyntaxShape::ColumnPath, "Optionally operate by column path")
    }

    fn usage(&self) -> &str {
        "Gets the extension of a path"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let (PathExtensionArguments { replace, rest }, input) = args.process().await?;
        let args = Arc::new(DefaultArguments {
            replace: replace.map(|v| v.item),
            prefix: None,
            suffix: None,
            num_levels: None,
            paths: rest,
        });
        operate(input, &action, tag.span, args).await
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
                result: Some(vec![Value::from(UntaggedValue::filepath("test.md"))]),
            },
            Example {
                description: "To replace more complex extensions:",
                example: "echo 'test.tar.gz' | path extension -r '' | path extension -r txt",
                result: Some(vec![Value::from(UntaggedValue::filepath("test.txt"))]),
            },
        ]
    }
}

fn action(path: &Path, args: Arc<DefaultArguments>) -> UntaggedValue {
    match args.replace {
        Some(ref extension) => UntaggedValue::filepath(path.with_extension(extension)),
        None => UntaggedValue::string(match path.extension() {
            Some(extension) => extension.to_string_lossy(),
            None => "".into(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::PathExtension;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(PathExtension {})
    }
}
