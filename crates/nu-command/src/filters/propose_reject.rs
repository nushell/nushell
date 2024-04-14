use nu_engine::command_prelude::*;
use nu_protocol::{IntRange, Range};

#[derive(Clone)]
pub struct ProposeReject;

impl Command for ProposeReject {
    fn name(&self) -> &str {
        "propose reject"
    }

    fn signature(&self) -> Signature {
        Signature::build("propose reject")
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
                    SyntaxShape::Int,
                    SyntaxShape::Range,
                    SyntaxShape::CellPath,
                ]),
                "The columns and rows to select from the table.",
            )
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Remove the given columns or rows from the table. Opposite of `select`."
    }

    fn extra_usage(&self) -> &str {
        "To remove a quantity of rows or columns, use `skip`, `drop`, or `drop column`."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["drop", "key"]
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
        let (paths, rows) = split_args(&values, call_span)?;
        let ctrlc = engine_state.ctrlc.clone();
        let metadata = input.metadata();

        // TODO: handle negatives
        // TODO: do error checking on columns via `get_columns` (and figure out
        // flags and default error checking states)
        // TODO: move this to a separate function
        match input {
            PipelineData::Value(mut value, ..) => {
                let value_span = value.span();
                match value {
                    Value::Record { .. } => {
                        reject_in_record(&mut value, &paths);
                        Ok(value.into_pipeline_data())
                    }
                    Value::List { vals, .. } => {
                        let mut values = select_rows(vals, &rows);
                        for value in &mut values {
                            reject_in_record(value, &paths);
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
            PipelineData::ListStream(..) => {
                if let Some(span) = rows.negatives_span() {
                    return Err(ShellError::IncorrectValue {
                        msg: "Can't use negative indexes with streams".into(),
                        val_span: span,
                        call_span,
                    });
                }

                Ok(input
                    .into_iter_strict(call.head)?
                    .zip(0..)
                    .filter(move |(_, i)| !rows.contains(*i))
                    .map(|(value, _)| value)
		    // XXX: the reason this monstrosity with clones is here is
		    // because the plain filter doesn't work, as it breaks some
		    // trait bound.
		    // .map(move |value| reject_in_record(&mut value, &paths))
                    .map(move |value| {
                        let mut clone = value.clone();
                        reject_in_record(&mut clone, &paths);
                        clone
                    })
                    .into_pipeline_data_with_metadata(metadata, ctrlc))
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
fn split_args(values: &[Value], call_span: Span) -> Result<(Vec<CellPath>, Rows), ShellError> {
    let mut paths = vec![];
    let mut indexes = vec![];
    let mut ranges = vec![];

    for value in values {
        let span = value.span();
        match value {
            Value::Int { val, .. } => indexes.push((*val, span)),
            Value::CellPath { val, .. } => paths.push(val.clone()),
            val @ Value::Range { val: range, .. } => match range {
                Range::IntRange(int_range) => ranges.push((*int_range, span)),
                Range::FloatRange { .. } => {
                    return Err(ShellError::IncorrectValue {
                        msg: "`reject` doesn't support float ranges".into(),
                        val_span: val.span(),
                        call_span,
                    })
                }
            },
            _ => unreachable!("Unexpected type: {}", value.get_type()),
        }
    }

    Ok((paths, Rows::new(indexes, ranges)))
}

/// Select `rows` from `values`, without cloning the values themselves.
fn select_rows<T>(values: Vec<T>, rows: &Rows) -> Vec<T> {
    let mut out = vec![];

    for (value, i) in values.into_iter().zip(0i64..) {
        if !rows.contains(i) {
            out.push(value);
        }
    }

    out
}

fn reject_in_record(value: &mut Value, paths: &[CellPath]) {
    for path in paths {
        value
            .remove_data_at_cell_path(&path.members)
            .expect("TODO flag?");
    }
}

struct Rows {
    // We have to preserve spans here for error messages.
    indexes: Vec<(i64, Span)>,
    ranges: Vec<(IntRange, Span)>,
}

impl Rows {
    fn new(integers: Vec<(i64, Span)>, ranges: Vec<(IntRange, Span)>) -> Self {
        Rows {
            indexes: integers,
            ranges,
        }
    }

    fn contains(&self, index: i64) -> bool {
        for (range, _) in &self.ranges {
            if range.contains(index) {
                return true;
            }
        }
        for (i, _) in &self.indexes {
            if *i == index {
                return true;
            }
        }

        false
    }

    /// Checks wherever the rows contains negative indexes
    fn negatives_span(&self) -> Option<Span> {
        for (index, span) in &self.indexes {
            if *index < 0 {
                return Some(*span);
            }
        }
        for (range, span) in &self.ranges {
            if range.start() < 0 {
                return Some(*span);
            }

            use std::ops::Bound;
            match range.end() {
                Bound::Included(e) | Bound::Excluded(e) => {
                    if e < 0 {
                        return Some(*span);
                    }
                }
                Bound::Unbounded => {}
            }
        }

        None
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::ProposeReject;
        use crate::test_examples;
        test_examples(ProposeReject {})
    }
}
