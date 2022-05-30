use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{CaptureBlock, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, ShellError,
    Signature, Span, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Merge;

impl Command for Merge {
    fn name(&self) -> &str {
        "merge"
    }

    fn usage(&self) -> &str {
        "Merge a table into an input table"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("merge")
            .required(
                "block",
                SyntaxShape::Block(Some(vec![])),
                "the block to run and merge into the table",
            )
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[a b c] | wrap name | merge { [1 2 3] | wrap index }",
                description: "Merge an index column into the input table",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_record(
                            vec!["name", "index"],
                            vec![Value::test_string("a"), Value::test_int(1)],
                        ),
                        Value::test_record(
                            vec!["name", "index"],
                            vec![Value::test_string("b"), Value::test_int(2)],
                        ),
                        Value::test_record(
                            vec!["name", "index"],
                            vec![Value::test_string("c"), Value::test_int(3)],
                        ),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "{a: 1, b: 2} | merge { {c: 3} }",
                description: "Merge two records",
                result: Some(Value::test_record(
                    vec!["a", "b", "c"],
                    vec![Value::test_int(1), Value::test_int(2), Value::test_int(3)],
                )),
            },
            Example {
                example: "{a: 1, b: 3} | merge { {b: 2, c: 4} }",
                description: "Merge two records with overlap key",
                result: Some(Value::test_record(
                    vec!["a", "b", "c"],
                    vec![Value::test_int(1), Value::test_int(2), Value::test_int(4)],
                )),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let block: CaptureBlock = call.req(engine_state, stack, 0)?;
        let mut stack = stack.captures_to_stack(&block.captures);

        let metadata = input.metadata();
        let ctrlc = engine_state.ctrlc.clone();
        let block = engine_state.get_block(block.block_id);
        let call = call.clone();

        let result = eval_block(
            engine_state,
            &mut stack,
            block,
            PipelineData::new(call.head),
            call.redirect_stdout,
            call.redirect_stderr,
        );

        let table = match result {
            Ok(res) => res,
            Err(e) => return Err(e),
        };

        match (&input, &table) {
            // table (list of records)
            (
                PipelineData::Value(Value::List { .. }, ..) | PipelineData::ListStream { .. },
                PipelineData::Value(Value::List { .. }, ..) | PipelineData::ListStream { .. },
            ) => {
                let mut table_iter = table.into_iter();

                let res =
                    input
                        .into_iter()
                        .map(move |inp| match (inp.as_record(), table_iter.next()) {
                            (Ok((inp_cols, inp_vals)), Some(to_merge)) => {
                                match to_merge.as_record() {
                                    Ok((to_merge_cols, to_merge_vals)) => {
                                        let (cols, vals) = do_merge(
                                            (inp_cols.to_vec(), inp_vals.to_vec()),
                                            (to_merge_cols.to_vec(), to_merge_vals.to_vec()),
                                        );
                                        Value::Record {
                                            cols,
                                            vals,
                                            span: call.head,
                                        }
                                    }
                                    Err(error) => Value::Error { error },
                                }
                            }
                            (_, None) => inp,
                            (Err(error), _) => Value::Error { error },
                        });

                if let Some(md) = metadata {
                    Ok(res.into_pipeline_data_with_metadata(md, ctrlc))
                } else {
                    Ok(res.into_pipeline_data(ctrlc))
                }
            }
            // record
            (
                PipelineData::Value(
                    Value::Record {
                        cols: inp_cols,
                        vals: inp_vals,
                        ..
                    },
                    ..,
                ),
                PipelineData::Value(
                    Value::Record {
                        cols: to_merge_cols,
                        vals: to_merge_vals,
                        ..
                    },
                    ..,
                ),
            ) => {
                let (cols, vals) = do_merge(
                    (inp_cols.to_vec(), inp_vals.to_vec()),
                    (to_merge_cols.to_vec(), to_merge_vals.to_vec()),
                );
                Ok(Value::Record {
                    cols,
                    vals,
                    span: call.head,
                }
                .into_pipeline_data())
            }
            (_, PipelineData::Value(val, ..)) | (PipelineData::Value(val, ..), _) => {
                let span = if val.span()? == Span::test_data() {
                    Span::new(call.head.start, call.head.start)
                } else {
                    val.span()?
                };

                Err(ShellError::PipelineMismatch(
                    "record or table in both the input and the argument block".to_string(),
                    call.head,
                    span,
                ))
            }
            _ => Err(ShellError::PipelineMismatch(
                "record or table in both the input and the argument block".to_string(),
                call.head,
                Span::new(call.head.start, call.head.start),
            )),
        }
    }
}

fn do_merge(
    input_record: (Vec<String>, Vec<Value>),
    to_merge_record: (Vec<String>, Vec<Value>),
) -> (Vec<String>, Vec<Value>) {
    let (mut result_cols, mut result_vals) = input_record;
    let (to_merge_cols, to_merge_vals) = to_merge_record;

    for (col, val) in to_merge_cols.into_iter().zip(to_merge_vals) {
        let pos = result_cols.iter().position(|c| c == &col);
        // if find, replace existing data, else, push new data.
        match pos {
            Some(index) => {
                result_vals[index] = val;
            }
            None => {
                result_cols.push(col);
                result_vals.push(val);
            }
        }
    }
    (result_cols, result_vals)
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
