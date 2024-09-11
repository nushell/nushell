use nu_engine::command_prelude::*;

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
                    SyntaxShape::Closure(Some(vec![SyntaxShape::Any, SyntaxShape::Any])),
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
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Sort by the given columns, in increasing order."
    }

    fn examples(&self) -> Vec<Example> {
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
        let metadata = input.metadata();
        let mut vec: Vec<_> = input.into_iter_strict(head)?.collect();

        if comparator_vals.is_empty() {
            return Err(ShellError::MissingParameter {
                param_name: "comparator".into(),
                span: head,
            });
        }

        let mut comparators = vec![];
        for val in comparator_vals.into_iter() {
            match val {
                Value::CellPath { val, .. } => {
                    comparators.push(Comparator::CellPath(val));
                }
                Value::Closure { val, .. } => {
                    comparators.push(Comparator::Closure(
                        *val,
                        engine_state.clone(),
                        stack.clone(),
                    ));
                }
                _ => {
                    return Err(ShellError::TypeMismatch {
                        err_message:
                            "Cannot sort using a value which is not a cell path or closure".into(),
                        span: val.span(),
                    })
                }
            }
        }

        crate::sort_by(&mut vec, comparators, head, insensitive, natural)?;

        if reverse {
            vec.reverse()
        }

        let iter = vec.into_iter();
        Ok(iter.into_pipeline_data_with_metadata(head, engine_state.signals().clone(), metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SortBy {})
    }
}
