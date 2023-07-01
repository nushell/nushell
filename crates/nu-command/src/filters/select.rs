use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath, PathMember};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData,
    PipelineIterator, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use std::collections::HashSet;

#[derive(Clone)]
pub struct Select;

impl Command for Select {
    fn name(&self) -> &str {
        "select"
    }

    // FIXME: also add support for --skip
    fn signature(&self) -> Signature {
        Signature::build("select")
            .input_output_types(vec![
                (Type::Record(vec![]), Type::Record(vec![])),
                (Type::Table(vec![]), Type::Table(vec![])),
            ])
            .switch(
                "ignore-errors",
                "ignore missing data (make all cell path members optional)",
                Some('i'),
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "the columns to select from the table",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Select only these columns or rows from the input. Opposite of `reject`."
    }

    fn extra_usage(&self) -> &str {
        r#"This differs from `get` in that, rather than accessing the given value in the data structure,
it removes all non-selected values from the structure. Hence, using `select` on a table will
produce a table, a list will produce a list, and a record will produce a record."#
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["pick", "choose", "get"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let mut columns: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let ignore_errors = call.has_flag("ignore-errors");
        let span = call.head;

        if ignore_errors {
            for cell_path in &mut columns {
                cell_path.make_optional();
            }
        }

        select(engine_state, span, columns, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Select a column in a table",
                example: "[{a: a b: b}] | select a",
                result: Some(Value::List {
                    vals: vec![Value::test_record(vec!["a"], vec![Value::test_string("a")])],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Select a field in a record",
                example: "{a: a b: b} | select a",
                result: Some(Value::test_record(vec!["a"], vec![Value::test_string("a")])),
            },
            Example {
                description: "Select just the `name` column",
                example: "ls | select name",
                result: None,
            },
            Example {
                description: "Select the first four rows (this is the same as `first 4`)",
                example: "ls | select 0 1 2 3",
                result: None,
            },
        ]
    }
}

fn select(
    engine_state: &EngineState,
    call_span: Span,
    columns: Vec<CellPath>,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let mut unique_rows: HashSet<usize> = HashSet::new();

    let mut new_columns = vec![];

    for column in columns {
        let CellPath { ref members } = column;
        match members.get(0) {
            Some(PathMember::Int { val, span, .. }) => {
                if members.len() > 1 {
                    return Err(ShellError::GenericError(
                        "Select only allows row numbers for rows".into(),
                        "extra after row number".into(),
                        Some(*span),
                        None,
                        Vec::new(),
                    ));
                }
                if unique_rows.contains(val) {
                    return Err(ShellError::GenericError(
                        "Select can't get the same row twice".into(),
                        "duplicated row index".into(),
                        Some(*span),
                        None,
                        Vec::new(),
                    ));
                }
                unique_rows.insert(*val);
            }
            _ => new_columns.push(column),
        };
    }
    let columns = new_columns;
    let mut unique_rows: Vec<usize> = unique_rows.into_iter().collect();

    let input = if !unique_rows.is_empty() {
        unique_rows.sort_unstable();
        // let skip = call.has_flag("skip");
        let metadata = input.metadata();
        let pipeline_iter: PipelineIterator = input.into_iter();

        NthIterator {
            input: pipeline_iter,
            rows: unique_rows,
            skip: false,
            current: 0,
        }
        .into_pipeline_data(engine_state.ctrlc.clone())
        .set_metadata(metadata)
    } else {
        input
    };

    match input {
        PipelineData::Value(
            Value::List {
                vals: input_vals,
                span,
            },
            metadata,
            ..,
        ) => {
            let mut output = vec![];
            let mut columns_with_value = Vec::new();
            for input_val in input_vals {
                if !columns.is_empty() {
                    let mut cols = vec![];
                    let mut vals = vec![];
                    for path in &columns {
                        //FIXME: improve implementation to not clone
                        match input_val.clone().follow_cell_path(&path.members, false) {
                            Ok(fetcher) => {
                                cols.push(path.into_string().replace('.', "_"));
                                vals.push(fetcher);
                                if !columns_with_value.contains(&path) {
                                    columns_with_value.push(path);
                                }
                            }
                            Err(e) => {
                                return Err(e);
                            }
                        }
                    }

                    output.push(Value::Record { cols, vals, span })
                } else {
                    output.push(input_val)
                }
            }

            Ok(output
                .into_iter()
                .into_pipeline_data(engine_state.ctrlc.clone())
                .set_metadata(metadata))
        }
        PipelineData::ListStream(stream, metadata, ..) => {
            let mut values = vec![];

            for x in stream {
                if !columns.is_empty() {
                    let mut cols = vec![];
                    let mut vals = vec![];
                    for path in &columns {
                        //FIXME: improve implementation to not clone
                        match x.clone().follow_cell_path(&path.members, false) {
                            Ok(value) => {
                                cols.push(path.into_string().replace('.', "_"));
                                vals.push(value);
                            }
                            Err(e) => return Err(e),
                        }
                    }
                    values.push(Value::Record {
                        cols,
                        vals,
                        span: call_span,
                    });
                } else {
                    values.push(x);
                }
            }

            Ok(values
                .into_pipeline_data(engine_state.ctrlc.clone())
                .set_metadata(metadata))
        }
        PipelineData::Value(v, metadata, ..) => {
            if !columns.is_empty() {
                let mut cols = vec![];
                let mut vals = vec![];

                for cell_path in columns {
                    // FIXME: remove clone
                    match v.clone().follow_cell_path(&cell_path.members, false) {
                        Ok(result) => {
                            cols.push(cell_path.into_string().replace('.', "_"));
                            vals.push(result);
                        }
                        Err(e) => return Err(e),
                    }
                }

                Ok(Value::Record {
                    cols,
                    vals,
                    span: call_span,
                }
                .into_pipeline_data()
                .set_metadata(metadata))
            } else {
                Ok(v.into_pipeline_data().set_metadata(metadata))
            }
        }
        _ => Ok(PipelineData::empty()),
    }
}

struct NthIterator {
    input: PipelineIterator,
    rows: Vec<usize>,
    skip: bool,
    current: usize,
}

impl Iterator for NthIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if !self.skip {
                if let Some(row) = self.rows.first() {
                    if self.current == *row {
                        self.rows.remove(0);
                        self.current += 1;
                        return self.input.next();
                    } else {
                        self.current += 1;
                        let _ = self.input.next();
                        continue;
                    }
                } else {
                    return None;
                }
            } else if let Some(row) = self.rows.first() {
                if self.current == *row {
                    self.rows.remove(0);
                    self.current += 1;
                    let _ = self.input.next();
                    continue;
                } else {
                    self.current += 1;
                    return self.input.next();
                }
            } else {
                return self.input.next();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Select)
    }
}
