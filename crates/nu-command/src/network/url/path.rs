use super::{operator, url};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, Signature, Span, SyntaxShape, Type, Value};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "url path"
    }

    fn signature(&self) -> Signature {
        Signature::build("url path")
            .input_output_types(vec![(Type::String, Type::String)])
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "optionally operate by cell path",
            )
            .category(Category::Network)
    }

    fn usage(&self) -> &str {
        "Get the path of a URL"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        operator(engine_state, stack, call, input, &url::Url::path)
    }

    fn examples(&self) -> Vec<Example> {
        let span = Span::test_data();
        vec![
            Example {
                description: "Get path of a url",
                example: "echo 'http://www.example.com/foo/bar' | url path",
                result: Some(Value::String {
                    val: "/foo/bar".to_string(),
                    span,
                }),
            },
            Example {
                description: "A trailing slash will be reflected in the path",
                example: "echo 'http://www.example.com' | url path",
                result: Some(Value::String {
                    val: "/".to_string(),
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
