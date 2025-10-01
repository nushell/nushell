use nu_engine::{ClosureEval, command_prelude::*};
use nu_protocol::engine::{Closure, CommandType};

#[derive(Clone)]
pub struct Where;

impl Command for Where {
    fn name(&self) -> &str {
        "where"
    }

    fn description(&self) -> &str {
        "Filter values of an input list based on a condition."
    }

    fn extra_description(&self) -> &str {
        r#"A condition is evaluated for each element of the input, and only elements which meet the condition are included in the output.

A condition can be either a "row condition" or a closure. A row condition is a special short-hand syntax to makes accessing fields easier.
Each element of the input can be accessed through the `$it` variable.

On the left hand side of a row condition, any field name is automatically expanded to use `$it`.
For example, `where type == dir` is equivalent to `where $it.type == dir`. This expansion does not happen when passing a subexpression or closure to `where`.

When using a closure, the element is passed as an argument and as pipeline input (`$in`) to the closure. Unlike row conditions, the `$it` variable isn't available inside closures.

Row conditions cannot be stored in a variable. To pass a condition with a variable, use a closure instead."#
    }

    fn command_type(&self) -> CommandType {
        CommandType::Keyword
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
                "condition",
                SyntaxShape::RowCondition,
                "Filter row condition or closure.",
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
            .filter_map(move |value| {
                match closure
                    .run_with_value(value.clone())
                    .and_then(|data| data.into_value(head))
                {
                    Ok(cond) => cond.is_true().then_some(value),
                    Err(err) => Some(Value::error(err, head)),
                }
            })
            .into_pipeline_data_with_metadata(head, engine_state.signals().clone(), metadata))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Filter rows of a table according to a condition",
                example: "[{a: 1} {a: 2}] | where a > 1",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "a" => Value::test_int(2),
                })])),
            },
            Example {
                description: "List only the files in the current directory",
                example: "ls | where type == file",
                result: None,
            },
            Example {
                description: "List all files in the current directory with sizes greater than 2kb",
                example: "ls | where size > 2kb",
                result: None,
            },
            Example {
                description: r#"List all files with names that contain "Car""#,
                example: r#"ls | where name =~ "Car""#,
                result: None,
            },
            Example {
                description: "List all files that were modified in the last two weeks",
                example: "ls | where modified >= (date now) - 2wk",
                result: None,
            },
            Example {
                description: "Filter items of a list with a row condition",
                example: "[1 2 3 4 5] | where $it > 2",
                result: Some(Value::test_list(vec![
                    Value::test_int(3),
                    Value::test_int(4),
                    Value::test_int(5),
                ])),
            },
            Example {
                description: "Filter items of a list with a closure",
                example: "[1 2 3 4 5] | where {|x| $x > 2 }",
                result: Some(Value::test_list(vec![
                    Value::test_int(3),
                    Value::test_int(4),
                    Value::test_int(5),
                ])),
            },
            Example {
                description: "Find files whose filenames don't begin with the correct sequential number",
                example: "ls | where type == file | sort-by name --natural | enumerate | where {|e| $e.item.name !~ $'^($e.index + 1)' } | get item",
                result: None,
            },
            Example {
                description: r#"Find case-insensitively files called "readme", with a subexpression inside the row condition"#,
                example: "ls | where ($it.name | str downcase) =~ readme",
                result: None,
            },
            Example {
                description: r#"Find case-insensitively files called "readme", with regex only"#,
                example: "ls | where name =~ '(?i)readme'",
                result: None,
            },
            Example {
                description: "Filter rows of a table according to a stored condition",
                example: "let cond = {|x| $x.a > 1}; [{a: 1} {a: 2}] | where $cond",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "a" => Value::test_int(2),
                })])),
            },
            Example {
                description: "List all numbers above 3, using an existing closure condition",
                example: "let a = {$in > 3}; [1, 2, 5, 6] | where $a",
                result: Some(Value::test_list(vec![
                    Value::test_int(5),
                    Value::test_int(6),
                ])),
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

        test_examples(Where {})
    }
}
