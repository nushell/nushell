use nu_engine::CallExt;
use nu_protocol::ast::{Call, PathMember};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, Range, Record, ShellError, Signature, Span,
    SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct ProposeSelect;

impl Command for ProposeSelect {
    fn name(&self) -> &str {
        "propose select"
    }

    fn signature(&self) -> Signature {
        Signature::build("propose select")
            .input_output_types(vec![
                (Type::Record(vec![]), Type::Record(vec![])),
                (Type::Table(vec![]), Type::Table(vec![])),
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (Type::Binary, Type::Binary),
            ])
            .rest(
                "rest",
                SyntaxShape::OneOf(vec![
                    SyntaxShape::CellPath,
                    SyntaxShape::Int,
                    SyntaxShape::Range,
                ]),
                "The columns and rows to select from the table.",
            )
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Select only these columns and rows from the input. Opposite of `reject`."
    }

    fn extra_usage(&self) -> &str {
        r#"Unlike `get`, this command always returns the same type it was given.  Hence, using `select` on a table will produce a table, a list will produce a list, and so on."#
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["pick", "choose", "take"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let values: Vec<Value> = call.rest(engine_state, stack, 0)?;
        let call_span = call.span();
        let (members, rows, ranges) = split_args(&values, call_span)?;

        // get column names and rows
        // two functions: for rejecting columns and for rows
        // columns just uses `remove_data_at_cell_path`
        // for rows:
        // zip (0..) with input
        // filter checking i not in ranges or ints
        // if there are negative rows, collect input and also check length - i
        // map and remove the index

        let values = input.into_value(Span::unknown());

        let out: Vec<_> = (0i64..)
            .zip(values.as_list()?)
            .filter(|(i, _)| {
                if rows.contains(i) {
                    return true;
                }
                let curr_i = Value::int(*i, Span::unknown());
                for range in &ranges {
                    if range.contains(&curr_i) {
                        return true;
                    }
                }

                // TODO variable
                false
            })
            .map(|(_, value)| value)
            .map(|value| {
                let value = match value {
                    Value::Record { val, .. } => {
                        let record = select_record(val, members);
			Value::record(record, value.span())
                    }
                    _ => value.clone(),
                };
                let mut value = value.clone();
                for i in 0..members.len() {
                    // Ignore error
                    let _ = value.remove_data_at_cell_path(&members[i..i + 1]);
                }
                value
            })
            .collect();
        Ok(Value::test_list(out).into_pipeline_data())

        // Ok(Value::nothing(Span::unknown()).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }
}

fn split_args(
    values: &[Value],
    call_span: Span,
) -> Result<(Vec<PathMember>, Vec<i64>, Vec<Range>), ShellError> {
    let mut members = vec![];
    let mut rows = vec![];
    let mut ranges = vec![];

    for value in values {
        let span = value.span();
        match value {
            Value::Int { val, .. } => rows.push(*val),
            Value::CellPath { val, .. } => {
                if let [member] = &val.members[..] {
                    match member {
                        // TODO panic
                        PathMember::Int { val, .. } => rows.push(*val as i64),
                        _ => members.push(member.clone()),
                    }
                } else {
                    return Err(ShellError::IncorrectValue {
                        msg: "`select` doesn't support nested cell paths".to_string(),
                        val_span: span,
                        call_span,
                    });
                }
            }
            Value::Range { val, .. } => ranges.push(*val.to_owned()),
            _ => unreachable!(),
        }
    }

    Ok((members, rows, ranges))
}

fn select_record(record: &Record, names: &[String]) -> Record {
    let mut copy = record.clone();
    for column in record.columns() {
        if names.contains(column) {
            copy.remove(column);
        }
    }
    copy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(ProposeSelect)
    }
}
