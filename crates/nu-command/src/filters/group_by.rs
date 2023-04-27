use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value};

use indexmap::IndexMap;

#[derive(Clone)]
pub struct GroupBy;

impl Command for GroupBy {
    fn name(&self) -> &str {
        "group-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("group-by")
            // TODO: It accepts Table also, but currently there is no Table
            // example. Perhaps Table should be a subtype of List, in which case
            // the current signature would suffice even when a Table example
            // exists.
            .input_output_types(vec![(
                Type::List(Box::new(Type::Any)),
                Type::Record(vec![]),
            )])
            .optional(
                "grouper",
                SyntaxShape::CellPath,
                "the path to the column to group on",
            )
    }

    fn usage(&self) -> &str {
        "Splits a list or table into groups, and returns a record containing those groups."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        group_by(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Group items by the \"type\" column's values",
                example: r#"ls | group-by type"#,
                result: None,
            },
            Example {
                description: "Group items by the \"foo\" column's values, ignoring records without a \"foo\" column",
                example: r#"open cool.json | group-by foo?"#,
                result: None,
            },
            Example {
                description: "You can also group by raw values by leaving out the argument",
                example: "['1' '3' '1' '3' '2' '1' '1'] | group-by",
                result: Some(Value::Record {
                    cols: vec!["1".to_string(), "3".to_string(), "2".to_string()],
                    vals: vec![
                        Value::List {
                            vals: vec![
                                Value::test_string("1"),
                                Value::test_string("1"),
                                Value::test_string("1"),
                                Value::test_string("1"),
                            ],
                            span: Span::test_data(),
                        },
                        Value::List {
                            vals: vec![Value::test_string("3"), Value::test_string("3")],
                            span: Span::test_data(),
                        },
                        Value::List {
                            vals: vec![Value::test_string("2")],
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

pub fn group_by(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;

    let cell_path: Option<CellPath> = call.opt(engine_state, stack, 0)?;
    let values: Vec<Value> = input.into_iter().collect();

    if values.is_empty() {
        return Err(ShellError::GenericError(
            "expected table from pipeline".into(),
            "requires a table input".into(),
            Some(span),
            None,
            Vec::new(),
        ));
    }

    let group_value = group(&cell_path, values, span)?;
    Ok(PipelineData::Value(group_value, None))
}

pub fn group(
    column_name: &Option<CellPath>,
    values: Vec<Value>,
    span: Span,
) -> Result<Value, ShellError> {
    let mut groups: IndexMap<String, Vec<Value>> = IndexMap::new();

    if let Some(column_name) = column_name {
        for value in values.into_iter() {
            let group_key = value
                .clone()
                .follow_cell_path(&column_name.members, false)?;
            if matches!(group_key, Value::Nothing { .. }) {
                continue; // likely the result of a failed optional access, ignore this value
            }

            let group_key = group_key.as_string()?;
            let group = groups.entry(group_key).or_default();
            group.push(value);
        }
    } else {
        for value in values.into_iter() {
            let group_key = value.as_string()?;
            let group = groups.entry(group_key).or_default();
            group.push(value);
        }
    };

    let mut cols = vec![];
    let mut vals = vec![];

    for (k, v) in groups {
        cols.push(k.to_string());
        vals.push(Value::List { vals: v, span });
    }

    Ok(Value::Record { cols, vals, span })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(GroupBy {})
    }
}
