use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Drop;

impl Command for Drop {
    fn name(&self) -> &str {
        "drop"
    }

    fn signature(&self) -> Signature {
        Signature::build("drop")
            .input_output_types(vec![
                (Type::table(), Type::table()),
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
            ])
            .optional("rows", SyntaxShape::Int, "The number of items to remove.")
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Remove items/rows from the end of the input list/table. Counterpart of `skip`. Opposite of `last`."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["delete", "remove"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "[0,1,2,3] | drop",
                description: "Remove the last item of a list",
                result: Some(Value::test_list(vec![
                    Value::test_int(0),
                    Value::test_int(1),
                    Value::test_int(2),
                ])),
            },
            Example {
                example: "[0,1,2,3] | drop 0",
                description: "Remove zero item of a list",
                result: Some(Value::test_list(vec![
                    Value::test_int(0),
                    Value::test_int(1),
                    Value::test_int(2),
                    Value::test_int(3),
                ])),
            },
            Example {
                example: "[0,1,2,3] | drop 2",
                description: "Remove the last two items of a list",
                result: Some(Value::test_list(vec![
                    Value::test_int(0),
                    Value::test_int(1),
                ])),
            },
            Example {
                description: "Remove the last row in a table",
                example: "[[a, b]; [1, 2] [3, 4]] | drop 1",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "a" => Value::test_int(1),
                    "b" => Value::test_int(2),
                })])),
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
        let metadata = input.metadata();
        let rows: Option<Spanned<i64>> = call.opt(engine_state, stack, 0)?;
        let mut values = input.into_iter_strict(head)?.collect::<Vec<_>>();

        let rows_to_drop = if let Some(rows) = rows {
            if rows.item < 0 {
                return Err(ShellError::NeedsPositiveValue { span: rows.span });
            } else {
                rows.item as usize
            }
        } else {
            1
        };

        values.truncate(values.len().saturating_sub(rows_to_drop));
        Ok(Value::list(values, head).into_pipeline_data_with_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use crate::Drop;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Drop {})
    }
}
