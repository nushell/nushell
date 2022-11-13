use inflector::cases::camelcase::to_camel_case;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

use crate::operate;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str camel-case"
    }

    fn signature(&self) -> Signature {
        Signature::build("str camel-case")
            .input_output_types(vec![(Type::String, Type::String)])
            .vectorizes_over_list(true)
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, convert strings at the given cell paths",
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Convert a string to camelCase"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "style", "caps", "convention"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        operate(engine_state, stack, call, input, &to_camel_case)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "convert a string to camelCase",
                example: " 'NuShell' | str camel-case",
                result: Some(Value::String {
                    val: "nuShell".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "convert a string to camelCase",
                example: "'this-is-the-first-case' | str camel-case",
                result: Some(Value::String {
                    val: "thisIsTheFirstCase".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "convert a string to camelCase",
                example: " 'this_is_the_second_case' | str camel-case",
                result: Some(Value::String {
                    val: "thisIsTheSecondCase".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "convert a column from a table to camelCase",
                example: r#"[[lang, gems]; [nu_test, 100]] | str camel-case lang"#,
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        span: Span::test_data(),
                        cols: vec!["lang".to_string(), "gems".to_string()],
                        vals: vec![
                            Value::String {
                                val: "nuTest".to_string(),
                                span: Span::test_data(),
                            },
                            Value::test_int(100),
                        ],
                    }],
                    span: Span::test_data(),
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
