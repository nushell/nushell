use inflector::cases::titlecase::to_title_case;
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
        "str title-case"
    }

    fn signature(&self) -> Signature {
        Signature::build("str title-case")
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
        "Convert a string to Title Case"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "style", "convention"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        operate(engine_state, stack, call, input, &to_title_case)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "convert a string to Title Case",
                example: "'nu-shell' | str title-case",
                result: Some(Value::String {
                    val: "Nu Shell".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "convert a string to Title Case",
                example: "'this is a test case' | str title-case",
                result: Some(Value::String {
                    val: "This Is A Test Case".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "convert a column from a table to Title Case",
                example: r#"[[title, count]; ['nu test', 100]] | str title-case title"#,
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        span: Span::test_data(),
                        cols: vec!["title".to_string(), "count".to_string()],
                        vals: vec![
                            Value::String {
                                val: "Nu Test".to_string(),
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
