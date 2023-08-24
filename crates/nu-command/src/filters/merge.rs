use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, Record,
    ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Merge;

impl Command for Merge {
    fn name(&self) -> &str {
        "merge"
    }

    fn usage(&self) -> &str {
        "Merge the input with a record or table, overwriting values in matching columns."
    }

    fn extra_usage(&self) -> &str {
        r#"You may provide a column structure to merge

When merging tables, row 0 of the input table is overwritten
with values from row 0 of the provided table, then
repeating this process with row 1, and so on."#
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("merge")
            .input_output_types(vec![
                (Type::Record(vec![]), Type::Record(vec![])),
                (Type::Table(vec![]), Type::Table(vec![])),
            ])
            .required(
                "value",
                // Both this and `update` should have a shape more like <record> | <table> than just <any>. -Leon 2022-10-27
                SyntaxShape::Any,
                "the new value to merge with",
            )
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[a b c] | wrap name | merge ( [1 2 3] | wrap index )",
                description: "Add an 'index' column to the input table",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_record(Record {
                            cols: vec!["name".to_string(), "index".to_string()],
                            vals: vec![Value::test_string("a"), Value::test_int(1)],
                        }),
                        Value::test_record(Record {
                            cols: vec!["name".to_string(), "index".to_string()],
                            vals: vec![Value::test_string("b"), Value::test_int(2)],
                        }),
                        Value::test_record(Record {
                            cols: vec!["name".to_string(), "index".to_string()],
                            vals: vec![Value::test_string("c"), Value::test_int(3)],
                        }),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "{a: 1, b: 2} | merge {c: 3}",
                description: "Merge two records",
                result: Some(Value::test_record(Record {
                    cols: vec!["a".to_string(), "b".to_string(), "c".to_string()],
                    vals: vec![Value::test_int(1), Value::test_int(2), Value::test_int(3)],
                })),
            },
            Example {
                example: "[{columnA: A0 columnB: B0}] | merge [{columnA: 'A0*'}]",
                description: "Merge two tables, overwriting overlapping columns",
                result: Some(Value::List {
                    vals: vec![Value::test_record(Record {
                        cols: vec!["columnA".to_string(), "columnB".to_string()],
                        vals: vec![Value::test_string("A0*"), Value::test_string("B0")],
                    })],
                    span: Span::test_data(),
                }),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let merge_value: Value = call.req(engine_state, stack, 0)?;

        let metadata = input.metadata();
        let ctrlc = engine_state.ctrlc.clone();
        let call = call.clone();

        match (&input, merge_value) {
            // table (list of records)
            (
                PipelineData::Value(Value::List { .. }, ..) | PipelineData::ListStream { .. },
                Value::List { vals, .. },
            ) => {
                let mut table_iter = vals.into_iter();

                let res =
                    input
                        .into_iter()
                        .map(move |inp| match (inp.as_record(), table_iter.next()) {
                            (Ok(inp), Some(to_merge)) => match to_merge.as_record() {
                                Ok(to_merge) => Value::record(do_merge(inp, to_merge), call.head),
                                Err(error) => Value::Error {
                                    error: Box::new(error),
                                },
                            },
                            (_, None) => inp,
                            (Err(error), _) => Value::Error {
                                error: Box::new(error),
                            },
                        });

                if let Some(md) = metadata {
                    Ok(res.into_pipeline_data_with_metadata(md, ctrlc))
                } else {
                    Ok(res.into_pipeline_data(ctrlc))
                }
            }
            // record
            (
                PipelineData::Value(Value::Record { val: inp, .. }, ..),
                Value::Record { val: to_merge, .. },
            ) => Ok(Value::record(do_merge(inp, &to_merge), call.head).into_pipeline_data()),
            (PipelineData::Value(val, ..), ..) => {
                // Only point the "value originates here" arrow at the merge value
                // if it was generated from a block. Otherwise, point at the pipeline value. -Leon 2022-10-27
                let span = if val.span()? == Span::test_data() {
                    Span::new(call.head.start, call.head.start)
                } else {
                    val.span()?
                };

                Err(ShellError::PipelineMismatch {
                    exp_input_type: "input, and argument, to be both record or both table"
                        .to_string(),
                    dst_span: call.head,
                    src_span: span,
                })
            }
            _ => Err(ShellError::PipelineMismatch {
                exp_input_type: "input, and argument, to be both record or both table".to_string(),
                dst_span: call.head,
                src_span: Span::new(call.head.start, call.head.start),
            }),
        }
    }
}

fn do_merge(input_record: &Record, to_merge_record: &Record) -> Record {
    let mut result = input_record.clone();

    for (col, val) in to_merge_record {
        let pos = result.cols.iter().position(|c| c == col);
        // if find, replace existing data, else, push new data.
        match pos {
            Some(index) => {
                result.vals[index] = val.clone();
            }
            None => {
                result.push(col, val.clone());
            }
        }
    }
    result
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Merge {})
    }
}
