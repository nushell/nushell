use nu_engine::CallExt;
use nu_protocol::ast::{Call, PathMember};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, Range,
    ShellError, Signature, Span, SyntaxShape, Type, Value,
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
		// TODO: investigate replacing CellPath with strings
		// The reason for this is because this forbids nested cell paths
		// anyways, since we'll have `get` for it.  Currently
		// expressions like `select 1..` error out, since `1..` gets
		// interpretted as a cell path.  And typing `select (1)..` every
		// time is quite cluncky.
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
        let (keys, rows) = split_args(&values, call_span)?;

	// TODO: handle negatives
	// TODO: do error checking on columns via `get_columns` (and figure out
	// flags and default error checking states)
	// TODO: move this to a separate function
        match input {
            PipelineData::Value(mut value, ..) => {
                let value_span = value.span();
                match value {
                    Value::Record { .. } => {
                        select_in_record(&mut value, &keys);
                        Ok(value.into_pipeline_data())
                    }
                    Value::List { vals, .. } => {
                        let mut values = select_rows(vals, &rows);
                        for value in &mut values {
                            select_in_record(value, &keys);
                        }

                        Ok(Value::list(values, value_span).into_pipeline_data())
                    }
                    Value::Binary { val, .. } => {
                        let bytes = select_rows(val, &rows);
                        Ok(Value::binary(bytes, value_span).into_pipeline_data())
                    }
                    _ => unreachable!("Unexpected type {}", value.get_type()),
                }
            }
            PipelineData::ListStream(stream, metadata, ..) => {
                if let Some(max_idex) = rows.max_index() {
                    // TODO panic.  Needs a proper integer management to avoid
		    // stuff like #11756
                    let values: Vec<Value> = stream.take(max_idex as usize).collect();

                    // XXX copypasta from Value::List handling
                    let mut values = select_rows(values, &rows);
                    for value in &mut values {
                        select_in_record(value, &keys);
                    }

                    Ok(values
                        .into_pipeline_data_with_metadata(metadata, engine_state.ctrlc.clone()))
                } else {
                    let iter = stream
                        // enumerate
                        .zip(0i64..)
                        .filter(move |(_, i)| {
                            // TODO try avoiding the entire filter here if rows
			    // are empty
                            if rows.is_empty() {
                                return true;
                            }

                            rows.contains(*i)
                        })
                        // remove enumeration
                        .map(|(value, _)| value)
                        .map(move |mut value| {
                            select_in_record(&mut value, &keys);
                            value
                        })
                        .into_pipeline_data_with_metadata(metadata, engine_state.ctrlc.clone());
                    Ok(iter)
                }
            }
            _ => todo!("add error"),
        }
    }

    fn examples(&self) -> Vec<Example> {
        // TODO add examples back when the interface gets locked in
        vec![]
    }
}

/// Split arguments into rows and columns.
fn split_args(values: &[Value], call_span: Span) -> Result<(Vec<String>, Rows), ShellError> {
    let mut members = vec![];
    let mut integers = vec![];
    let mut ranges = vec![];

    for value in values {
        let span = value.span();
        match value {
            Value::Int { val, .. } => integers.push(*val),
            Value::CellPath { val, .. } => {
                if let [member] = &val.members[..] {
                    match member {
                        // TODO panic, conversion
                        PathMember::Int { val, .. } => integers.push(*val as i64),
                        PathMember::String { val, .. } => members.push(val.clone()),
                    }
                } else {
                    return Err(ShellError::IncorrectValue {
                        msg: "`select` doesn't support nested cell paths".to_string(),
                        val_span: span,
                        call_span,
                    });
                }
            }
            Value::Range { val, .. } => ranges.push(val.to_owned()),
            _ => unreachable!("Unexpected type: {}", value.get_type()),
        }
    }

    Ok((members, Rows::new(integers, ranges)))
}

/// Pick key-value pairs from `select_keys` inplace.
fn select_in_record(value: &mut Value, select_keys: &[String]) {
    if select_keys.is_empty() {
        return;
    }

    match value {
        Value::Record { val, .. } => {
            // An ugly hack because columns is some weird iterator, not a vector
            // TODO do it the right way.  I'm ready to help change the record
            // inferface if that'll get rid of this monstrosity
            let columns = val.columns().map(|c| c.clone()).collect::<Vec<String>>();
            for key in columns {
                if !select_keys.contains(&key) {
                    val.remove(key);
                }
            }
        }
        _ => {}
    }
}

/// Select `rows` from `values`, without cloning the values themselves.
fn select_rows<T>(values: Vec<T>, rows: &Rows) -> Vec<T> {
    if rows.is_empty() {
        return values;
    }

    let mut out = vec![];

    for (value, i) in values.into_iter().zip(0i64..) {
        if rows.contains(i) {
            out.push(value);
        }
    }

    out
}

struct Rows {
    integers: Vec<i64>,
    ranges: Vec<Range>,
}

impl Rows {
    fn new(integers: Vec<i64>, ranges: Vec<Range>) -> Self {
        Rows { integers, ranges }
    }

    fn is_empty(&self) -> bool {
        self.integers.is_empty() && self.ranges.is_empty()
    }

    fn contains(&self, index: i64) -> bool {
        if self.integers.contains(&index) {
            return true;
        }

        let value_index = Value::int(index, Span::unknown());
        for range in &self.ranges {
            if range.contains(&value_index) {
                return true;
            }
        }

        false
    }

    fn max_index(&self) -> Option<i64> {
        // TODO
	None
    }
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
