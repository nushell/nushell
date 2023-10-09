use nu_engine::column::get_columns;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, Record, ShellError, Signature,
    Span, Spanned, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Transpose;

pub struct TransposeArgs {
    rest: Vec<Spanned<String>>,
    header_row: bool,
    ignore_titles: bool,
    as_record: bool,
    keep_last: bool,
    keep_all: bool,
}

impl Command for Transpose {
    fn name(&self) -> &str {
        "transpose"
    }

    fn signature(&self) -> Signature {
        Signature::build("transpose")
            .input_output_types(vec![
                (Type::Table(vec![]), Type::Any),
                (Type::Record(vec![]), Type::Table(vec![])),
            ])
            .switch(
                "header-row",
                "treat the first row as column names",
                Some('r'),
            )
            .switch(
                "ignore-titles",
                "don't transpose the column names into values",
                Some('i'),
            )
            .switch(
                "as-record",
                "transfer to record if the result is a table and contains only one row",
                Some('d'),
            )
            .switch(
                "keep-last",
                "on repetition of record fields due to `header-row`, keep the last value obtained",
                Some('l'),
            )
            .switch(
                "keep-all",
                "on repetition of record fields due to `header-row`, keep all the values obtained",
                Some('a'),
            )
            .allow_variants_without_examples(true)
            .rest(
                "rest",
                SyntaxShape::String,
                "the names to give columns once transposed",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Transposes the table contents so rows become columns and columns become rows."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["pivot"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        transpose(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        let span = Span::test_data();
        vec![
            Example {
                description: "Transposes the table contents with default column names",
                example: "[[c1 c2]; [1 2]] | transpose",
                result: Some(Value::list(
                    vec![
                        Value::test_record(Record {
                            cols: vec!["column0".to_string(), "column1".to_string()],
                            vals: vec![Value::test_string("c1"), Value::test_int(1)],
                        }),
                        Value::test_record(Record {
                            cols: vec!["column0".to_string(), "column1".to_string()],
                            vals: vec![Value::test_string("c2"), Value::test_int(2)],
                        }),
                    ],
                    span,
                )),
            },
            Example {
                description: "Transposes the table contents with specified column names",
                example: "[[c1 c2]; [1 2]] | transpose key val",
                result: Some(Value::list(
                    vec![
                        Value::test_record(Record {
                            cols: vec!["key".to_string(), "val".to_string()],
                            vals: vec![Value::test_string("c1"), Value::test_int(1)],
                        }),
                        Value::test_record(Record {
                            cols: vec!["key".to_string(), "val".to_string()],
                            vals: vec![Value::test_string("c2"), Value::test_int(2)],
                        }),
                    ],
                    span,
                )),
            },
            Example {
                description:
                    "Transposes the table without column names and specify a new column name",
                example: "[[c1 c2]; [1 2]] | transpose --ignore-titles val",
                result: Some(Value::list(
                    vec![
                        Value::test_record(Record {
                            cols: vec!["val".to_string()],
                            vals: vec![Value::test_int(1)],
                        }),
                        Value::test_record(Record {
                            cols: vec!["val".to_string()],
                            vals: vec![Value::test_int(2)],
                        }),
                    ],
                    span,
                )),
            },
            Example {
                description: "Transfer back to record with -d flag",
                example: "{c1: 1, c2: 2} | transpose | transpose --ignore-titles -r -d",
                result: Some(Value::test_record(Record {
                    cols: vec!["c1".to_string(), "c2".to_string()],
                    vals: vec![Value::test_int(1), Value::test_int(2)],
                })),
            },
        ]
    }
}

pub fn transpose(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let name = call.head;
    let transpose_args = TransposeArgs {
        header_row: call.has_flag("header-row"),
        ignore_titles: call.has_flag("ignore-titles"),
        as_record: call.has_flag("as-record"),
        keep_last: call.has_flag("keep-last"),
        keep_all: call.has_flag("keep-all"),
        rest: call.rest(engine_state, stack, 0)?,
    };

    let ctrlc = engine_state.ctrlc.clone();
    let metadata = input.metadata();
    let input: Vec<_> = input.into_iter().collect();
    let args = transpose_args;

    let descs = get_columns(&input);

    let mut headers: Vec<String> = vec![];

    if !args.rest.is_empty() && args.header_row {
        return Err(ShellError::GenericError(
            "Can not provide header names and use header row".into(),
            "using header row".into(),
            Some(name),
            None,
            Vec::new(),
        ));
    }

    if args.header_row {
        for i in input.clone() {
            if let Some(desc) = descs.get(0) {
                match &i.get_data_by_key(desc) {
                    Some(x) => {
                        if let Ok(s) = x.as_string() {
                            headers.push(s.to_string());
                        } else {
                            return Err(ShellError::GenericError(
                                "Header row needs string headers".into(),
                                "used non-string headers".into(),
                                Some(name),
                                None,
                                Vec::new(),
                            ));
                        }
                    }
                    _ => {
                        return Err(ShellError::GenericError(
                            "Header row is incomplete and can't be used".into(),
                            "using incomplete header row".into(),
                            Some(name),
                            None,
                            Vec::new(),
                        ));
                    }
                }
            } else {
                return Err(ShellError::GenericError(
                    "Header row is incomplete and can't be used".into(),
                    "using incomplete header row".into(),
                    Some(name),
                    None,
                    Vec::new(),
                ));
            }
        }
    } else {
        for i in 0..=input.len() {
            if let Some(name) = args.rest.get(i) {
                headers.push(name.item.clone())
            } else {
                headers.push(format!("column{i}"));
            }
        }
    }

    let descs: Vec<_> = if args.header_row {
        descs.into_iter().skip(1).collect()
    } else {
        descs
    };

    let mut result_data = descs
        .into_iter()
        .map(move |desc| {
            let mut column_num: usize = 0;
            let mut record = Record::new();

            if !args.ignore_titles && !args.header_row {
                record.push(
                    headers[column_num].clone(),
                    Value::string(desc.clone(), name),
                );
                column_num += 1
            }

            for i in input.clone() {
                match &i.get_data_by_key(&desc) {
                    Some(x) => {
                        if args.keep_all && record.cols.contains(&headers[column_num]) {
                            let index = record
                                .cols
                                .iter()
                                .position(|y| y == &headers[column_num])
                                .expect("value is contained.");
                            let current_span = record.vals[index].span();
                            let new_val = match &record.vals[index] {
                                Value::List { vals, .. } => {
                                    let mut vals = vals.clone();
                                    vals.push(x.clone());
                                    Value::list(vals.to_vec(), current_span)
                                }
                                v => Value::list(vec![v.clone(), x.clone()], v.span()),
                            };
                            record.cols.remove(index);
                            record.vals.remove(index);

                            record.push(headers[column_num].clone(), new_val);
                        } else if args.keep_last && record.cols.contains(&headers[column_num]) {
                            let index = record
                                .cols
                                .iter()
                                .position(|y| y == &headers[column_num])
                                .expect("value is contained.");
                            record.cols.remove(index);
                            record.vals.remove(index);
                            record.push(headers[column_num].clone(), x.clone());
                        } else if !record.cols.contains(&headers[column_num]) {
                            record.push(headers[column_num].clone(), x.clone());
                        }
                    }
                    _ => {
                        if args.keep_all && record.cols.contains(&headers[column_num]) {
                            let index = record
                                .cols
                                .iter()
                                .position(|y| y == &headers[column_num])
                                .expect("value is contained.");
                            let current_span = record.vals[index].span();
                            let new_val = match &record.vals[index] {
                                Value::List { vals, .. } => {
                                    let mut vals = vals.clone();
                                    vals.push(Value::nothing(name));
                                    Value::list(vals.to_vec(), current_span)
                                }
                                v => Value::list(vec![v.clone(), Value::nothing(name)], v.span()),
                            };
                            record.cols.remove(index);
                            record.vals.remove(index);

                            record.push(headers[column_num].clone(), new_val);
                        } else if args.keep_last && record.cols.contains(&headers[column_num]) {
                            let index = record
                                .cols
                                .iter()
                                .position(|y| y == &headers[column_num])
                                .expect("value is contained.");
                            record.cols.remove(index);
                            record.vals.remove(index);
                            record.push(headers[column_num].clone(), Value::nothing(name));
                        } else if !record.cols.contains(&headers[column_num]) {
                            record.push(headers[column_num].clone(), Value::nothing(name));
                        }
                    }
                }
                column_num += 1;
            }

            Value::record(record, name)
        })
        .collect::<Vec<Value>>();
    if result_data.len() == 1 && args.as_record {
        Ok(PipelineData::Value(
            result_data
                .pop()
                .expect("already check result only contains one item"),
            metadata,
        ))
    } else {
        Ok(result_data.into_pipeline_data(ctrlc).set_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Transpose {})
    }
}
