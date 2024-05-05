use nu_engine::command_prelude::*;
use nu_protocol::{ast::PathMember, PipelineIterator};
use std::collections::BTreeSet;

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
                (Type::record(), Type::record()),
                (Type::table(), Type::table()),
                (Type::List(Box::new(Type::Any)), Type::Any),
            ])
            .switch(
                "ignore-errors",
                "ignore missing data (make all cell path members optional)",
                Some('i'),
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "The columns to select from the table.",
            )
            .allow_variants_without_examples(true)
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
        let columns: Vec<Value> = call.rest(engine_state, stack, 0)?;
        let mut new_columns: Vec<CellPath> = vec![];
        for col_val in columns {
            let col_span = &col_val.span();
            match col_val {
                Value::CellPath { val, .. } => {
                    new_columns.push(val);
                }
                Value::String { val, .. } => {
                    let cv = CellPath {
                        members: vec![PathMember::String {
                            val: val.clone(),
                            span: *col_span,
                            optional: false,
                        }],
                    };
                    new_columns.push(cv.clone());
                }
                Value::Int { val, internal_span } => {
                    if val < 0 {
                        return Err(ShellError::CantConvert {
                            to_type: "cell path".into(),
                            from_type: "negative number".into(),
                            span: internal_span,
                            help: None,
                        });
                    }
                    let cv = CellPath {
                        members: vec![PathMember::Int {
                            val: val as usize,
                            span: *col_span,
                            optional: false,
                        }],
                    };
                    new_columns.push(cv.clone());
                }
                x => {
                    return Err(ShellError::CantConvert {
                        to_type: "cell path".into(),
                        from_type: x.get_type().to_string(),
                        span: x.span(),
                        help: None,
                    });
                }
            }
        }
        let ignore_errors = call.has_flag(engine_state, stack, "ignore-errors")?;
        let span = call.head;

        if ignore_errors {
            for cell_path in &mut new_columns {
                cell_path.make_optional();
            }
        }

        select(engine_state, span, new_columns, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Select a column in a table",
                example: "[{a: a b: b}] | select a",
                result: Some(Value::test_list(
                    vec![Value::test_record(record! {
                        "a" => Value::test_string("a")
                    })],
                )),
            },
            Example {
                description: "Select a field in a record",
                example: "{a: a b: b} | select a",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_string("a")
                })),
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
            Example {
                description: "Select multiple columns",
                example: "[[name type size]; [Cargo.toml toml 1kb] [Cargo.lock toml 2kb]] | select name type",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "name" => Value::test_string("Cargo.toml"),
                        "type" => Value::test_string("toml"),
                    }),
                    Value::test_record(record! {
                        "name" => Value::test_string("Cargo.lock"),
                        "type" => Value::test_string("toml")
                    }),
                ]))
            },
            Example {
                description: "Select multiple columns by spreading a list",
                example: r#"let cols = [name type]; [[name type size]; [Cargo.toml toml 1kb] [Cargo.lock toml 2kb]] | select ...$cols"#,
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "name" => Value::test_string("Cargo.toml"),
                        "type" => Value::test_string("toml")
                    }),
                    Value::test_record(record! {
                        "name" => Value::test_string("Cargo.lock"),
                        "type" => Value::test_string("toml")
                    }),
                ]))
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
    let mut unique_rows: BTreeSet<usize> = BTreeSet::new();

    let mut new_columns = vec![];

    for column in columns {
        let CellPath { ref members } = column;
        match members.first() {
            Some(PathMember::Int { val, span, .. }) => {
                if members.len() > 1 {
                    return Err(ShellError::GenericError {
                        error: "Select only allows row numbers for rows".into(),
                        msg: "extra after row number".into(),
                        span: Some(*span),
                        help: None,
                        inner: vec![],
                    });
                }
                unique_rows.insert(*val);
            }
            _ => {
                if !new_columns.contains(&column) {
                    new_columns.push(column)
                }
            }
        };
    }
    let columns = new_columns;

    let input = if !unique_rows.is_empty() {
        // let skip = call.has_flag(engine_state, stack, "skip")?;
        let metadata = input.metadata();
        let pipeline_iter: PipelineIterator = input.into_iter();

        NthIterator {
            input: pipeline_iter,
            rows: unique_rows.into_iter().peekable(),
            current: 0,
        }
        .into_pipeline_data_with_metadata(call_span, engine_state.ctrlc.clone(), metadata)
    } else {
        input
    };

    match input {
        PipelineData::Value(v, metadata, ..) => {
            let span = v.span();
            match v {
                Value::List {
                    vals: input_vals, ..
                } => {
                    let mut output = vec![];
                    let mut columns_with_value = Vec::new();
                    for input_val in input_vals {
                        if !columns.is_empty() {
                            let mut record = Record::new();
                            for path in &columns {
                                //FIXME: improve implementation to not clone
                                match input_val.clone().follow_cell_path(&path.members, false) {
                                    Ok(fetcher) => {
                                        record.push(path.to_string().replace('.', "_"), fetcher);
                                        if !columns_with_value.contains(&path) {
                                            columns_with_value.push(path);
                                        }
                                    }
                                    Err(e) => {
                                        return Err(e);
                                    }
                                }
                            }

                            output.push(Value::record(record, span))
                        } else {
                            output.push(input_val)
                        }
                    }

                    Ok(output.into_iter().into_pipeline_data_with_metadata(
                        call_span,
                        engine_state.ctrlc.clone(),
                        metadata,
                    ))
                }
                _ => {
                    if !columns.is_empty() {
                        let mut record = Record::new();

                        for cell_path in columns {
                            // FIXME: remove clone
                            match v.clone().follow_cell_path(&cell_path.members, false) {
                                Ok(result) => {
                                    record.push(cell_path.to_string().replace('.', "_"), result);
                                }
                                Err(e) => return Err(e),
                            }
                        }

                        Ok(Value::record(record, call_span)
                            .into_pipeline_data_with_metadata(metadata))
                    } else {
                        Ok(v.into_pipeline_data_with_metadata(metadata))
                    }
                }
            }
        }
        PipelineData::ListStream(stream, metadata, ..) => {
            let mut values = vec![];

            for x in stream {
                if !columns.is_empty() {
                    let mut record = Record::new();
                    for path in &columns {
                        //FIXME: improve implementation to not clone
                        match x.clone().follow_cell_path(&path.members, false) {
                            Ok(value) => {
                                record.push(path.to_string().replace('.', "_"), value);
                            }
                            Err(e) => return Err(e),
                        }
                    }
                    values.push(Value::record(record, call_span));
                } else {
                    values.push(x);
                }
            }

            Ok(values.into_pipeline_data_with_metadata(
                call_span,
                engine_state.ctrlc.clone(),
                metadata,
            ))
        }
        _ => Ok(PipelineData::empty()),
    }
}

struct NthIterator {
    input: PipelineIterator,
    rows: std::iter::Peekable<std::collections::btree_set::IntoIter<usize>>,
    current: usize,
}

impl Iterator for NthIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(row) = self.rows.peek() {
                if self.current == *row {
                    self.rows.next();
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
