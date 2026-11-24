use nu_engine::command_prelude::*;
use nu_protocol::ast::PathMember;

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
                "Also compact empty items like \"\", {}, and [].",
                Some('e'),
            )
            .rest(
                "columns",
                SyntaxShape::CellPath,
                "The columns (or cell paths) to compact from the table.",
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
        let columns: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        for path in &columns {
            if let Some(PathMember::Int { span, .. }) = path.members.first() {
                return Err(ShellError::TypeMismatch {
                    err_message: "compact needs a column name; quote a number to match a column, like \"2024\"".into(),
                    span: *span,
                });
            }
        }

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
                description: "Filter out all records where a nested value is null",
                example: "[{labels: {c: 1}} {labels: {c: null}}] | compact labels.c",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "labels" => Value::test_record(record! {
                        "c" => Value::test_int(1),
                    }),
                })])),
            },
            Example {
                description: "Filter out all instances of null from a list",
                example: "[1, null, 2] | compact",
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
                example: "{a: 1, b: null, c: 3} | compact",
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

fn do_keep_row(row: &Value, compact_empties: bool, columns: &[CellPath]) -> bool {
    do_keep_value(row, compact_empties)
        && row.as_record().map_or(true, |record| {
            columns.iter().all(|col| match col.members.as_slice() {
                // A plain column name is one record lookup, no path walk needed.
                // A missing member, optional or not, drops the row, as in the path walk
                // where an optional miss becomes null and null is dropped.
                [PathMember::String { val, casing, .. }] => record
                    .cased(*casing)
                    .get(val)
                    .is_some_and(|value| do_keep_value(value, compact_empties)),
                _ => row
                    .follow_cell_path(&col.members)
                    .is_ok_and(|value| do_keep_value(&value, compact_empties)),
            })
        })
}

#[cfg(test)]
mod tests {
    use super::Compact;

    #[test]
    fn examples_work_as_expected() -> nu_test_support::Result {
        nu_test_support::test().examples(Compact)
    }
}
