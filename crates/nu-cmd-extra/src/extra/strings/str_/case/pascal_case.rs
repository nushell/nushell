use super::operate;
use heck::ToUpperCamelCase;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct StrPascalCase;

impl Command for StrPascalCase {
    fn name(&self) -> &str {
        "str pascal-case"
    }

    fn signature(&self) -> Signature {
        Signature::build("str pascal-case")
            .input_output_types(vec![
                (Type::String, Type::String),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::String)),
                ),
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
        "Convert a string to PascalCase."
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
        operate(
            engine_state,
            stack,
            call,
            input,
            &ToUpperCamelCase::to_upper_camel_case,
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "convert a string to PascalCase",
                example: "'nu-shell' | str pascal-case",
                result: Some(Value::test_string("NuShell")),
            },
            Example {
                description: "convert a string to PascalCase",
                example: "'this-is-the-first-case' | str pascal-case",
                result: Some(Value::test_string("ThisIsTheFirstCase")),
            },
            Example {
                description: "convert a string to PascalCase",
                example: "'this_is_the_second_case' | str pascal-case",
                result: Some(Value::test_string("ThisIsTheSecondCase")),
            },
            Example {
                description: "convert a column from a table to PascalCase",
                example: r#"[[lang, gems]; [nu_test, 100]] | str pascal-case lang"#,
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "lang" => Value::test_string("NuTest"),
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

        test_examples(StrPascalCase {})
    }
}
