use super::{operator, url};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, Signature, Span, SyntaxShape, Type, Value};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "url query"
    }

    fn signature(&self) -> Signature {
        Signature::build("url query")
            .input_output_types(vec![(Type::String, Type::String)])
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "optionally operate by cell path",
            )
            .category(Category::Network)
    }

    fn usage(&self) -> &str {
        "Get the query string of a URL"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["parameter"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        operator(engine_state, stack, call, input, &query)
    }

    fn examples(&self) -> Vec<Example> {
        let span = Span::test_data();
        vec![
            Example {
                description: "Get a query string",
                example: "echo 'http://www.example.com/?foo=bar&baz=quux' | url query",
                result: Some(Value::String {
                    val: "foo=bar&baz=quux".to_string(),
                    span,
                }),
            },
            Example {
                description: "Returns an empty string if there is no query string",
                example: "echo 'http://www.example.com/' | url query",
                result: Some(Value::String {
                    val: "".to_string(),
                    span,
                }),
            },
        ]
    }
}

fn query(url: &url::Url) -> &str {
    url.query().unwrap_or("")
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
