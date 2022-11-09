use super::{operator, url};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, Signature, Span, SyntaxShape, Type, Value};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "url host"
    }

    fn signature(&self) -> Signature {
        Signature::build("url host")
            .input_output_types(vec![(Type::String, Type::String)])
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "optionally operate by cell path",
            )
            .category(Category::Network)
    }

    fn usage(&self) -> &str {
        "Get the host of a URL"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["hostname"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        operator(engine_state, stack, call, input, &host)
    }

    fn examples(&self) -> Vec<Example> {
        let span = Span::test_data();
        vec![Example {
            description: "Get host of a url",
            example: "echo 'http://www.example.com/foo/bar' | url host",
            result: Some(Value::String {
                val: "www.example.com".to_string(),
                span,
            }),
        }]
    }
}

fn host(url: &url::Url) -> &str {
    url.host_str().unwrap_or("")
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
