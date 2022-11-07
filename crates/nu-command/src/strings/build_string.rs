use nu_engine::eval_expression;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Type, Value,
};

#[derive(Clone)]
pub struct BuildString;

impl Command for BuildString {
    fn name(&self) -> &str {
        "build-string"
    }

    fn usage(&self) -> &str {
        "Create a string from the arguments."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["concatenate", "join"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("build-string")
            .rest("rest", SyntaxShape::String, "list of string")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .category(Category::Strings)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "build-string a b c",
                description: "Builds a string from letters a b c",
                result: Some(Value::String {
                    val: "abc".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                example: r#"build-string $"(1 + 2)" = one ' ' plus ' ' two"#,
                description: "Builds a string from subexpression separating words with spaces",
                result: Some(Value::String {
                    val: "3=one plus two".to_string(),
                    span: Span::test_data(),
                }),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let config = engine_state.get_config();
        let output = call
            .positional_iter()
            .map(|expr| {
                eval_expression(engine_state, stack, expr).map(|val| val.into_string(", ", config))
            })
            .collect::<Result<Vec<String>, ShellError>>()?;

        Ok(Value::String {
            val: output.join(""),
            span: call.head,
        }
        .into_pipeline_data())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(BuildString {})
    }
}
