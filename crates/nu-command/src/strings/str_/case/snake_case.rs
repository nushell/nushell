use inflector::cases::snakecase::to_snake_case;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value};

use crate::operate;
#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str snake-case"
    }

    fn signature(&self) -> Signature {
        Signature::build("str snake-case").rest(
            "rest",
            SyntaxShape::CellPath,
            "optionally convert text to snake_case by column paths",
        )
    }
    fn usage(&self) -> &str {
        "converts a string to snake_case"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        operate(engine_state, stack, call, input, &to_snake_case)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "convert a string to camelCase",
                example: r#" "NuShell" | str snake-case"#,
                result: Some(Value::String {
                    val: "nu_shell".to_string(),
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "convert a string to camelCase",
                example: r#" "this_is_the_second_case" | str snake-case"#,
                result: Some(Value::String {
                    val: "this_is_the_second_case".to_string(),
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "convert a string to camelCase",
                example: r#""this-is-the-first-case" | str snake-case"#,
                result: Some(Value::String {
                    val: "this_is_the_first_case".to_string(),
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "convert a column from a table to snake-case",
                example: r#"[[lang, gems]; [nuTest, 100]] | str snake-case lang"#,
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        span: Span::unknown(),
                        cols: vec!["lang".to_string(), "gems".to_string()],
                        vals: vec![
                            Value::String {
                                val: "nu_test".to_string(),
                                span: Span::unknown(),
                            },
                            Value::test_int(100),
                        ],
                    }],
                    span: Span::unknown(),
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
