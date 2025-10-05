use super::operate;
use heck::ToSnakeCase;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct StrSnakeCase;

impl Command for StrSnakeCase {
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
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, convert strings at the given cell paths.",
            )
            .category(Category::Strings)
    }

    fn description(&self) -> &str {
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
        operate(
            engine_state,
            stack,
            call,
            input,
            &ToSnakeCase::to_snake_case,
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
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
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "lang" => Value::test_string("nu_test"),
                    "gems" => Value::test_int(100),
                })])),
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

        test_examples(StrSnakeCase {})
    }
}
