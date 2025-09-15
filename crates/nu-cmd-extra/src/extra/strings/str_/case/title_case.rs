use super::operate;
use heck::ToTitleCase;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct StrTitleCase;

impl Command for StrTitleCase {
    fn name(&self) -> &str {
        "str title-case"
    }

    fn signature(&self) -> Signature {
        Signature::build("str title-case")
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
        "Convert a string to Title Case."
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
        operate(
            engine_state,
            stack,
            call,
            input,
            &ToTitleCase::to_title_case,
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "convert a string to Title Case",
                example: "'nu-shell' | str title-case",
                result: Some(Value::test_string("Nu Shell")),
            },
            Example {
                description: "convert a string to Title Case",
                example: "'this is a test case' | str title-case",
                result: Some(Value::test_string("This Is A Test Case")),
            },
            Example {
                description: "convert a column from a table to Title Case",
                example: r#"[[title, count]; ['nu test', 100]] | str title-case title"#,
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "title" =>  Value::test_string("Nu Test"),
                    "count" =>  Value::test_int(100),
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

        test_examples(StrTitleCase {})
    }
}
