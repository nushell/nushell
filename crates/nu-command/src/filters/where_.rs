use nu_engine::{command_prelude::*, ClosureEval};
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct Where;

impl Command for Where {
    fn name(&self) -> &str {
        "where"
    }

    fn usage(&self) -> &str {
        "Filter values based on a row condition."
    }

    fn extra_usage(&self) -> &str {
        r#"This command works similar to 'filter' but allows extra shorthands for working with
tables, known as "row conditions". On the other hand, reading the condition from a variable is
not supported."#
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("where")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (Type::table(), Type::table()),
                (Type::Range, Type::Any),
            ])
            .required(
                "row_condition",
                SyntaxShape::RowCondition,
                "Filter condition.",
            )
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["filter", "find", "search", "condition"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let closure: Closure = call.req(engine_state, stack, 0)?;

        let mut closure = ClosureEval::new(engine_state, stack, closure);

        let metadata = input.metadata();
        Ok(input
            .into_iter_strict(head)?
            .filter_map(move |value| match closure.run_with_value(value.clone()) {
                Ok(data) => data.into_value(head).is_true().then_some(value),
                Err(err) => Some(Value::error(err, head)),
            })
            .into_pipeline_data_with_metadata(metadata, engine_state.ctrlc.clone()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Filter rows of a table according to a condition",
                example: "[{a: 1} {a: 2}] | where a > 1",
                result: Some(Value::test_list(
                    vec![Value::test_record(record! {
                        "a" => Value::test_int(2),
                    })],
                )),
            },
            Example {
                description: "Filter items of a list according to a condition",
                example: "[1 2] | where {|x| $x > 1}",
                result: Some(Value::test_list(
                    vec![Value::test_int(2)],
                )),
            },
            Example {
                description: "List all files in the current directory with sizes greater than 2kb",
                example: "ls | where size > 2kb",
                result: None,
            },
            Example {
                description: "List only the files in the current directory",
                example: "ls | where type == file",
                result: None,
            },
            Example {
                description: "List all files with names that contain \"Car\"",
                example: "ls | where name =~ \"Car\"",
                result: None,
            },
            Example {
                description: "List all files that were modified in the last two weeks",
                example: "ls | where modified >= (date now) - 2wk",
                result: None,
            },
            Example {
                description: "Find files whose filenames don't begin with the correct sequential number",
                example: "ls | where type == file | sort-by name --natural | enumerate | where {|e| $e.item.name !~ $'^($e.index + 1)' } | each {|| get item }",
                result: None,
            },
            Example {
                description: r#"Find case-insensitively files called "readme", without an explicit closure"#,
                example: "ls | where ($it.name | str downcase) =~ readme",
                result: None,
            },
            Example {
                description: "same as above but with regex only",
                example: "ls | where name =~ '(?i)readme'",
                result: None,
            }


        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Where {})
    }
}
