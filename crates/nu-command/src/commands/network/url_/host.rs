use url::Url;

use super::operate;
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Signature, SyntaxShape, Value};

pub struct UrlHost;

impl WholeStreamCommand for UrlHost {
    fn name(&self) -> &str {
        "url host"
    }

    fn signature(&self) -> Signature {
        Signature::build("url host").rest(
            "rest",
            SyntaxShape::ColumnPath,
            "optionally operate by column path",
        )
    }

    fn usage(&self) -> &str {
        "gets the host of a url"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let rest: Vec<ColumnPath> = args.rest(0)?;
        let input = args.input;

        Ok(operate(input, rest, &host))
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
    use super::ShellError;
    use super::UrlHost;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(UrlHost {})
    }
}
