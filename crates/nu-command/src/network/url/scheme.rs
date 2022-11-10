use super::{operator, url};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, Signature, Span, SyntaxShape, Type, Value};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "url scheme"
    }

    fn signature(&self) -> Signature {
        Signature::build("url scheme")
            .input_output_types(vec![(Type::String, Type::String)])
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "optionally operate by cell path",
            )
            .category(Category::Network)
    }

    fn usage(&self) -> &str {
        "Get the scheme (e.g. http, file) of a URL"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["protocol"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        operator(engine_state, stack, call, input, &url::Url::scheme)
    }

    fn examples(&self) -> Vec<Example> {
        let span = Span::test_data();
        vec![
            Example {
                description: "Get the scheme of a URL",
                example: "echo 'http://www.example.com' | url scheme",
                result: Some(Value::String {
                    val: "http".to_string(),
                    span,
                }),
            },
            Example {
                description: "You get an empty string if there is no scheme",
                example: "echo 'test' | url scheme",
                result: Some(Value::String {
                    val: "".to_string(),
                    span,
                }),
            },
        ]
    }
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
