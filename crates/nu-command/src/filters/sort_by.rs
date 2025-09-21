use nu_engine::{ClosureEval, command_prelude::*};

use crate::Comparator;

#[derive(Clone)]
pub struct SortBy;

impl Command for SortBy {
    fn name(&self) -> &str {
        "sort-by"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("sort-by")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (Type::record(), Type::table()),
                (Type::table(), Type::table()),
            ])
            .rest(
                "comparator",
                SyntaxShape::OneOf(vec![
                    SyntaxShape::CellPath,
                    SyntaxShape::Closure(Some(vec![SyntaxShape::Any])), // key closure
                    SyntaxShape::Closure(Some(vec![SyntaxShape::Any, SyntaxShape::Any])), // custom closure
                ]),
                "The cell path(s) or closure(s) to compare elements by.",
            )
            .switch("reverse", "Sort in reverse order", Some('r'))
            .switch(
                "ignore-case",
                "Sort string-based data case-insensitively",
                Some('i'),
            )
            .switch(
                "natural",
                "Sort alphanumeric string-based data naturally (1, 9, 10, 99, 100, ...)",
                Some('n'),
            )
            .switch(
                "custom",
                "Use closures to specify a custom sort order, rather than to compute a comparison key",
                Some('c'),
            )
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Sort by the given cell path or closure."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Sort files by modified date",
                example: "ls | sort-by modified",
                result: None,
            },
            Example {
                description: "Sort files by name (case-insensitive)",
                example: "ls | sort-by name --ignore-case",
                result: None,
            },
            Example {
                description: "Sort a table by a column (reversed order)",
                example: "[[fruit count]; [apple 9] [pear 3] [orange 7]] | sort-by fruit --reverse",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "fruit" => Value::test_string("pear"),
                        "count" => Value::test_int(3),
                    }),
                    Value::test_record(record! {
                        "fruit" => Value::test_string("orange"),
                        "count" => Value::test_int(7),
                    }),
                    Value::test_record(record! {
                        "fruit" => Value::test_string("apple"),
                        "count" => Value::test_int(9),
                    }),
                ])),
            },
            Example {
                description: "Sort by a nested value",
                example: "[[name info]; [Cairo {founded: 969}] [Kyoto {founded: 794}]] | sort-by info.founded",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                    "name" => Value::test_string("Kyoto"),
                    "info" => Value::test_record(
                        record! { "founded" => Value::test_int(794) },
                    )}),
                    Value::test_record(record! {
                    "name" => Value::test_string("Cairo"),
                    "info" => Value::test_record(
                        record! { "founded" => Value::test_int(969) },
                    )}),
                ])),
            },
            Example {
                description: "Sort by the last value in a list",
                example: "[[2 50] [10 1]] | sort-by { last }",
                result: Some(Value::test_list(vec![
                    Value::test_list(vec![Value::test_int(10), Value::test_int(1)]),
                    Value::test_list(vec![Value::test_int(2), Value::test_int(50)]),
                ])),
            },
            Example {
                description: "Sort in a custom order",
                example: "[7 3 2 8 4] | sort-by -c {|a, b| $a < $b}",
                result: Some(Value::test_list(vec![
                    Value::test_int(2),
                    Value::test_int(3),
                    Value::test_int(4),
                    Value::test_int(7),
                    Value::test_int(8),
                ])),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let comparator_vals: Vec<Value> = call.rest(engine_state, stack, 0)?;
        let reverse = call.has_flag(engine_state, stack, "reverse")?;
        let insensitive = call.has_flag(engine_state, stack, "ignore-case")?;
        let natural = call.has_flag(engine_state, stack, "natural")?;
        let custom = call.has_flag(engine_state, stack, "custom")?;
        let metadata = input.metadata();
        let mut vec: Vec<_> = input.into_iter_strict(head)?.collect();

        if comparator_vals.is_empty() {
            return Err(ShellError::MissingParameter {
                param_name: "comparator".into(),
                span: head,
            });
        }

        let comparators = comparator_vals
            .into_iter()
            .map(|val| match val {
                Value::CellPath { val, .. } => Ok(Comparator::CellPath(val)),
                Value::Closure { val, .. } => {
                    let closure_eval = ClosureEval::new(engine_state, stack, *val);
                    if custom {
                        Ok(Comparator::CustomClosure(closure_eval))
                    } else {
                        Ok(Comparator::KeyClosure(closure_eval))
                    }
                }
                _ => Err(ShellError::TypeMismatch {
                    err_message: "Cannot sort using a value which is not a cell path or closure"
                        .into(),
                    span: val.span(),
                }),
            })
            .collect::<Result<_, _>>()?;

        crate::sort_by(&mut vec, comparators, head, insensitive, natural)?;

        if reverse {
            vec.reverse()
        }

        let val = Value::list(vec, head);
        Ok(val.into_pipeline_data_with_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use crate::{Last, test_examples_with_commands};

    use super::*;

    #[test]
    fn test_examples() {
        test_examples_with_commands(SortBy {}, &[&Last]);
    }
}
