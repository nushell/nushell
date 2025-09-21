use super::operate;
use heck::ToKebabCase;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct StrKebabCase;

impl Command for StrKebabCase {
    fn name(&self) -> &str {
        "str kebab-case"
    }

    fn signature(&self) -> Signature {
        Signature::build("str kebab-case")
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
        "Convert a string to kebab-case."
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
        operate(
            engine_state,
            stack,
            call,
            input,
            &ToKebabCase::to_kebab_case,
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "convert a string to kebab-case",
                example: "'NuShell' | str kebab-case",
                result: Some(Value::test_string("nu-shell")),
            },
            Example {
                description: "convert a string to kebab-case",
                example: "'thisIsTheFirstCase' | str kebab-case",
                result: Some(Value::test_string("this-is-the-first-case")),
            },
            Example {
                description: "convert a string to kebab-case",
                example: "'THIS_IS_THE_SECOND_CASE' | str kebab-case",
                result: Some(Value::test_string("this-is-the-second-case")),
            },
            Example {
                description: "convert a column from a table to kebab-case",
                example: r#"[[lang, gems]; [nuTest, 100]] | str kebab-case lang"#,
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "lang" =>  Value::test_string("nu-test"),
                    "gems" =>  Value::test_int(100),
                })])),
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

        test_examples(StrKebabCase {})
    }
}
