use url::Url;

use super::{operate, DefaultArguments};
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};

pub struct UrlPath;

#[async_trait]
impl WholeStreamCommand for UrlPath {
    fn name(&self) -> &str {
        "url path"
    }

    fn signature(&self) -> Signature {
        Signature::build("url path")
            .rest(SyntaxShape::ColumnPath, "optionally operate by column path")
    }

    fn usage(&self) -> &str {
        "gets the path of a url"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let (DefaultArguments { rest }, input) = args.process().await?;
        operate(input, rest, &Url::path).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get path of a url",
                example: "echo 'http://www.example.com/foo/bar' | url path",
                result: Some(vec![Value::from("/foo/bar")]),
            },
            Example {
                description: "A trailing slash will be reflected in the path",
                example: "echo 'http://www.example.com' | url path",
                result: Some(vec![Value::from("/")]),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::UrlPath;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(UrlPath {})
    }
}
