use inflector::cases::pascalcase::to_pascal_case;
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
        "str pascal-case"
    }

    fn signature(&self) -> Signature {
        Signature::build("str pascal-case")
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
        "Convert a string to PascalCase"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "style", "caps", "upper", "convention"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        operate(engine_state, stack, call, input, &to_pascal_case)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "convert a string to PascalCase",
                example: "'nu-shell' | str pascal-case",
                result: Some(Value::String {
                    val: "NuShell".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "convert a string to PascalCase",
                example: "'this-is-the-first-case' | str pascal-case",
                result: Some(Value::String {
                    val: "ThisIsTheFirstCase".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "convert a string to PascalCase",
                example: "'this_is_the_second_case' | str pascal-case",
                result: Some(Value::String {
                    val: "ThisIsTheSecondCase".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "convert a column from a table to PascalCase",
                example: r#"[[lang, gems]; [nu_test, 100]] | str pascal-case lang"#,
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        span: Span::test_data(),
                        cols: vec!["lang".to_string(), "gems".to_string()],
                        vals: vec![
                            Value::String {
                                val: "NuTest".to_string(),
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
