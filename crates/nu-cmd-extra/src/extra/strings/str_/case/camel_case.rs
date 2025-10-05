use super::operate;
use heck::ToLowerCamelCase;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct StrCamelCase;

impl Command for StrCamelCase {
    fn name(&self) -> &str {
        "str camel-case"
    }

    fn signature(&self) -> Signature {
        Signature::build("str camel-case")
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
        "Convert a string to camelCase."
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
        operate(
            engine_state,
            stack,
            call,
            input,
            &ToLowerCamelCase::to_lower_camel_case,
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "convert a string to camelCase",
                example: " 'NuShell' | str camel-case",
                result: Some(Value::test_string("nuShell")),
            },
            Example {
                description: "convert a string to camelCase",
                example: "'this-is-the-first-case' | str camel-case",
                result: Some(Value::test_string("thisIsTheFirstCase")),
            },
            Example {
                description: "convert a string to camelCase",
                example: " 'this_is_the_second_case' | str camel-case",
                result: Some(Value::test_string("thisIsTheSecondCase")),
            },
            Example {
                description: "convert a column from a table to camelCase",
                example: r#"[[lang, gems]; [nu_test, 100]] | str camel-case lang"#,
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "lang" => Value::test_string("nuTest"),
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

        test_examples(StrCamelCase {})
    }
}
