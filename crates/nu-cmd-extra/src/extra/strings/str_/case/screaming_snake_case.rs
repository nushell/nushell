use super::operate;
use heck::ToShoutySnakeCase;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct StrScreamingSnakeCase;

impl Command for StrScreamingSnakeCase {
    fn name(&self) -> &str {
        "str screaming-snake-case"
    }

    fn signature(&self) -> Signature {
        Signature::build("str screaming-snake-case")
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
        "Convert a string to SCREAMING_SNAKE_CASE."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "style", "underscore", "convention"]
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
            &ToShoutySnakeCase::to_shouty_snake_case,
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "convert a string to SCREAMING_SNAKE_CASE",
                example: r#" "NuShell" | str screaming-snake-case"#,
                result: Some(Value::test_string("NU_SHELL")),
            },
            Example {
                description: "convert a string to SCREAMING_SNAKE_CASE",
                example: r#" "this_is_the_second_case" | str screaming-snake-case"#,
                result: Some(Value::test_string("THIS_IS_THE_SECOND_CASE")),
            },
            Example {
                description: "convert a string to SCREAMING_SNAKE_CASE",
                example: r#""this-is-the-first-case" | str screaming-snake-case"#,
                result: Some(Value::test_string("THIS_IS_THE_FIRST_CASE")),
            },
            Example {
                description: "convert a column from a table to SCREAMING_SNAKE_CASE",
                example: r#"[[lang, gems]; [nu_test, 100]] | str screaming-snake-case lang"#,
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "lang" =>  Value::test_string("NU_TEST"),
                    "gems" =>  Value::test_int(100),
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

        test_examples(StrScreamingSnakeCase {})
    }
}
