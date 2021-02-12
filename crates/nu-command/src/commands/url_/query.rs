use url::Url;

use super::{operate, DefaultArguments};
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};

pub struct UrlQuery;

#[async_trait]
impl WholeStreamCommand for UrlQuery {
    fn name(&self) -> &str {
        "url query"
    }

    fn signature(&self) -> Signature {
        Signature::build("url query")
            .rest(SyntaxShape::ColumnPath, "optionally operate by column path")
    }

    fn usage(&self) -> &str {
        "gets the query of a url"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let (DefaultArguments { rest }, input) = args.process().await?;
        operate(input, rest, &query).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get query of a url",
                example: "echo 'http://www.example.com/?foo=bar&baz=quux' | url query",
                result: Some(vec![Value::from("foo=bar&baz=quux")]),
            },
            Example {
                description: "No query gives the empty string",
                example: "echo 'http://www.example.com/' | url query",
                result: Some(vec![Value::from("")]),
            },
        ]
    }
}

fn query(url: &Url) -> &str {
    url.query().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::UrlQuery;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(UrlQuery {})
    }
}
