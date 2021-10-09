use nu_engine::eval_expression;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{Example, ShellError, Signature, Span, SyntaxShape, Value};

pub struct BuildString;

impl Command for BuildString {
    fn name(&self) -> &str {
        "build-string"
    }

    fn usage(&self) -> &str {
        "Create a string from the arguments."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("build-string").rest("rest", SyntaxShape::String, "list of string")
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "build-string a b c",
                description: "Builds a string from letters a b c",
                result: Some(Value::String {
                    val: "abc".to_string(),
                    span: Span::unknown(),
                }),
            },
            Example {
                example: "build-string (1 + 2) = one ' ' plus ' ' two",
                description: "Builds a string from letters a b c",
                result: Some(Value::String {
                    val: "3=one plus two".to_string(),
                    span: Span::unknown(),
                }),
            },
        ]
    }

    fn run(
        &self,
        context: &EvaluationContext,
        call: &Call,
        _input: Value,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        let output = call
            .positional
            .iter()
            .map(|expr| eval_expression(context, expr).map(|val| val.into_string()))
            .collect::<Result<Vec<String>, ShellError>>()?;

        Ok(Value::String {
            val: output.join(""),
            span: call.head,
        })
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
