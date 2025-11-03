use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Compact;

impl Command for Compact {
    fn name(&self) -> &str {
        "compact"
    }

    fn signature(&self) -> Signature {
        Signature::build("compact")
            .input_output_types(vec![
                (Type::record(), Type::record()),
                (Type::list(Type::Any), Type::list(Type::Any)),
            ])
            .switch(
                "empty",
                "also compact empty items like \"\", {}, and []",
                Some('e'),
            )
            .rest(
                "columns",
                SyntaxShape::Any,
                "The columns to compact from the table.",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Creates a table with non-empty rows."
    }

    fn extra_description(&self) -> &str {
        "Can be used to remove `null` or empty values from lists and records too."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["empty", "remove"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        mut input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let empty = call.has_flag(engine_state, stack, "empty")?;
        let columns: Vec<String> = call.rest(engine_state, stack, 0)?;

        match input {
            PipelineData::Value(Value::Record { ref mut val, .. }, ..) => {
                val.to_mut().retain(|_, val| do_keep_value(val, empty));
                Ok(input)
            }
            _ => input.filter(
                move |item| do_keep_row(item, empty, columns.as_slice()),
                engine_state.signals(),
            ),
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Filter out all records where 'Hello' is null",
                example: r#"[["Hello" "World"]; [null 3]] | compact Hello"#,
                result: Some(Value::test_list(vec![])),
            },
            Example {
                description: "Filter out all records where 'World' is null",
                example: r#"[["Hello" "World"]; [null 3]] | compact World"#,
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "Hello" => Value::nothing(Span::test_data()),
                    "World" => Value::test_int(3),
                })])),
            },
            Example {
                description: "Filter out all instances of null from a list",
                example: r#"[1, null, 2] | compact"#,
                result: Some(Value::test_list(vec![
                    Value::test_int(1),
                    Value::test_int(2),
                ])),
            },
            Example {
                description: "Filter out all instances of null and empty items from a list",
                example: r#"[1, null, 2, "", 3, [], 4, {}, 5] | compact --empty"#,
                result: Some(Value::test_list(vec![
                    Value::test_int(1),
                    Value::test_int(2),
                    Value::test_int(3),
                    Value::test_int(4),
                    Value::test_int(5),
                ])),
            },
            Example {
                description: "Filter out all instances of null from a record",
                example: r#"{a: 1, b: null, c: 3} | compact"#,
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(1),
                    "c" =>  Value::test_int(3),
                })),
            },
        ]
    }
}

fn do_keep_value(value: &Value, compact_empties: bool) -> bool {
    let remove = match compact_empties {
        true => value.is_empty(),
        false => value.is_nothing(),
    };
    !remove
}

fn do_keep_row(row: &Value, compact_empties: bool, columns: &[impl AsRef<str>]) -> bool {
    let do_keep = move |value| do_keep_value(value, compact_empties);

    do_keep(row)
        && row.as_record().map_or(true, |record| {
            columns
                .iter()
                .all(|col| record.get(col).map(do_keep).unwrap_or(false))
        })
}

#[cfg(test)]
mod tests {
    use super::Compact;

    #[test]
    fn examples_work_as_expected() {
        use crate::test_examples;
        test_examples(Compact {})
    }
}
