use inflector::cases::snakecase::to_snake_case;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, Record, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

use super::operate;
#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str snake-case"
    }

    fn signature(&self) -> Signature {
        Signature::build("str snake-case")
            .input_output_types(vec![
                (Type::String, Type::String),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::String)),
                ),
                (Type::Table(vec![]), Type::Table(vec![])),
                (Type::Record(vec![]), Type::Record(vec![])),
            ])
            .allow_variants_without_examples(true)
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, convert strings at the given cell paths",
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Convert a string to snake_case."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "style", "underscore", "lower", "convention"]
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
                description: "convert a string to snake_case",
                example: r#" "NuShell" | str snake-case"#,
                result: Some(Value::test_string("nu_shell")),
            },
            Example {
                description: "convert a string to snake_case",
                example: r#" "this_is_the_second_case" | str snake-case"#,
                result: Some(Value::test_string("this_is_the_second_case")),
            },
            Example {
                description: "convert a string to snake_case",
                example: r#""this-is-the-first-case" | str snake-case"#,
                result: Some(Value::test_string("this_is_the_first_case")),
            },
            Example {
                description: "convert a column from a table to snake_case",
                example: r#"[[lang, gems]; [nuTest, 100]] | str snake-case lang"#,
                result: Some(Value::list(
                    vec![Value::test_record(Record {
                        cols: vec!["lang".to_string(), "gems".to_string()],
                        vals: vec![Value::test_string("nu_test"), Value::test_int(100)],
                    })],
                    Span::test_data(),
                )),
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
