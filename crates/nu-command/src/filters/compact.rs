use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Compact;

impl Command for Compact {
    fn name(&self) -> &str {
        "compact"
    }

    fn signature(&self) -> Signature {
        Signature::build("compact")
            .input_output_types(vec![(
                Type::List(Box::new(Type::Any)),
                Type::List(Box::new(Type::Any)),
            )])
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

    fn search_terms(&self) -> Vec<&str> {
        vec!["empty", "remove"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let empty = call.has_flag(engine_state, stack, "empty")?;
        compact(engine_state, stack, call, input, empty)
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
        ]
    }
}

pub fn compact(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
    compact_empties: bool,
) -> Result<PipelineData, ShellError> {
    let columns: Vec<String> = call.rest(engine_state, stack, 0)?;
    let metadata = input.metadata();
    input
        .filter(
            move |item| {
                match item {
                    // Nothing is filtered out
                    Value::Nothing { .. } => false,
                    Value::Record { val, .. } => {
                        for column in columns.iter() {
                            match val.get(column) {
                                None => return false,
                                Some(x) => {
                                    if let Value::Nothing { .. } = x {
                                        return false;
                                    }
                                    if compact_empties {
                                        // check if the value is one of the empty value
                                        if match x {
                                            Value::String { val, .. } => val.is_empty(),
                                            Value::Record { val, .. } => val.is_empty(),
                                            Value::List { vals, .. } => vals.is_empty(),
                                            _ => false,
                                        } {
                                            // one of the empty value found so skip now
                                            return false;
                                        }
                                    }
                                }
                            }
                        }

                        if compact_empties && val.is_empty() {
                            return false;
                        }
                        // No defined columns contained Nothing
                        true
                    }
                    Value::List { vals, .. } => {
                        if compact_empties && vals.is_empty() {
                            return false;
                        }
                        true
                    }
                    Value::String { val, .. } => {
                        if compact_empties && val.is_empty() {
                            return false;
                        }
                        true
                    }
                    // Any non-Nothing, non-record should be kept
                    _ => true,
                }
            },
            engine_state.signals(),
        )
        .map(|m| m.set_metadata(metadata))
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
