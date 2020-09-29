use url::Url;

use super::{operate, DefaultArguments};
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};

pub struct UrlScheme;

#[async_trait]
impl WholeStreamCommand for UrlScheme {
    fn name(&self) -> &str {
        "url scheme"
    }

    fn signature(&self) -> Signature {
        Signature::build("url scheme").rest(SyntaxShape::ColumnPath, "optionally operate by path")
    }

    fn usage(&self) -> &str {
        "gets the scheme (eg http, file) of a url"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let (DefaultArguments { rest }, input) = args.process(&registry).await?;
        operate(input, rest, &Url::scheme).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get scheme of a url",
                example: "echo 'http://www.example.com' | url scheme",
                result: Some(vec![Value::from("http")]),
            },
            Example {
                description: "You get an empty string if there is no scheme",
                example: "echo 'test' | url scheme",
                result: Some(vec![Value::from("")]),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::UrlScheme;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(UrlScheme {})
    }
}
