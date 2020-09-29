use url::Url;

use super::{operate, DefaultArguments};
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};

pub struct UrlHost;

#[async_trait]
impl WholeStreamCommand for UrlHost {
    fn name(&self) -> &str {
        "url host"
    }

    fn signature(&self) -> Signature {
        Signature::build("url host")
            .rest(SyntaxShape::ColumnPath, "optionally operate by column path")
    }

    fn usage(&self) -> &str {
        "gets the host of a url"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let (DefaultArguments { rest }, input) = args.process(&registry).await?;
        operate(input, rest, &host).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get host of a url",
            example: "echo 'http://www.example.com/foo/bar' | url host",
            result: Some(vec![Value::from("www.example.com")]),
        }]
    }
}

fn host(url: &Url) -> &str {
    url.host_str().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::UrlHost;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(UrlHost {})
    }
}
