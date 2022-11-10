use inflector::cases::kebabcase::to_kebab_case;
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
        "str kebab-case"
    }

    fn signature(&self) -> Signature {
        Signature::build("str kebab-case")
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
        "Convert a string to kebab-case"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "style", "hyphens", "convention"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        operate(engine_state, stack, call, input, &to_kebab_case)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "convert a string to kebab-case",
                example: "'NuShell' | str kebab-case",
                result: Some(Value::String {
                    val: "nu-shell".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "convert a string to kebab-case",
                example: "'thisIsTheFirstCase' | str kebab-case",
                result: Some(Value::String {
                    val: "this-is-the-first-case".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "convert a string to kebab-case",
                example: "'THIS_IS_THE_SECOND_CASE' | str kebab-case",
                result: Some(Value::String {
                    val: "this-is-the-second-case".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "convert a column from a table to kebab-case",
                example: r#"[[lang, gems]; [nuTest, 100]] | str kebab-case lang"#,
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        span: Span::test_data(),
                        cols: vec!["lang".to_string(), "gems".to_string()],
                        vals: vec![
                            Value::String {
                                val: "nu-test".to_string(),
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
mod tests {
    use super::*;
    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
