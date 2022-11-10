use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, FromValue, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData,
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
        r#"You may provide a column structure to merge, or a block
that generates a column structure.

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
                "block",
                // Both this and `update` should have a shape more like <record> | <table> | <block> than just <any>. -Leon 2022-10-27
                SyntaxShape::Any,
                "the new value to merge with, or a block that produces it",
            )
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[a b c] | wrap name | merge { [1 2 3] | wrap index }",
                description: "Add an 'index' column to the input table",
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
                example: "{a: 1, b: 2} | merge {c: 3}",
                description: "Merge two records",
                result: Some(Value::Record {
                    cols: vec!["a".to_string(), "b".to_string(), "c".to_string()],
                    vals: vec![Value::test_int(1), Value::test_int(2), Value::test_int(3)],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "{a: 1, b: 3} | merge { { b: 2 } | merge { c: 4 } }",
                description: "Merge records, overwriting overlapping values",
                result: Some(Value::test_record(
                    vec!["a", "b", "c"],
                    vec![Value::test_int(1), Value::test_int(2), Value::test_int(4)],
                )),
            },
            Example {
                example: "[{columnA: A0 columnB: B0}] | merge [{columnA: 'A0*'}]",
                description: "Merge two tables, overwriting overlapping columns",
                result: Some(Value::List {
                    vals: vec![Value::test_record(
                        vec!["columnA", "columnB"],
                        vec![Value::test_string("A0*"), Value::test_string("B0")],
                    )],
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
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let replacement: Value = call.req(engine_state, stack, 0)?;

        let metadata = input.metadata();
        let ctrlc = engine_state.ctrlc.clone();
        let call = call.clone();

        let argument_was_block = replacement.as_block().is_ok();

        let merge_value: Value = if argument_was_block {
            // When given a block, run it to obtain the matching value.
            let capture_block: Closure = FromValue::from_value(&replacement)?;

            let mut stack = stack.captures_to_stack(&capture_block.captures);
            stack.with_env(&stack.env_vars.clone(), &stack.env_hidden.clone());

            let block = engine_state.get_block(capture_block.block_id);
            let result = eval_block(
                engine_state,
                &mut stack,
                block,
                PipelineData::new(call.head),
                call.redirect_stdout,
                call.redirect_stderr,
            );

            match result {
                Ok(res) => res.into_value(call.head),
                Err(e) => return Err(e),
            }
        } else {
            // Otherwise, just use the passed-in value as-is.
            replacement
        };

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
                Value::Record {
                    cols: to_merge_cols,
                    vals: to_merge_vals,
                    ..
                },
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
            (PipelineData::Value(val, ..), merge_value) => {
                // Only point the "value originates here" arrow at the merge value
                // if it was generated from a block. Otherwise, point at the pipeline value. -Leon 2022-10-27
                let span = if argument_was_block {
                    if merge_value.span()? == Span::test_data() {
                        Span::new(call.head.start, call.head.start)
                    } else {
                        merge_value.span()?
                    }
                } else if val.span()? == Span::test_data() {
                    Span::new(call.head.start, call.head.start)
                } else {
                    val.span()?
                };

                Err(ShellError::PipelineMismatch(
                    "input, and argument, to be both record or both table".to_string(),
                    call.head,
                    span,
                ))
            }
            _ => Err(ShellError::PipelineMismatch(
                "input, and argument, to be both record or both table".to_string(),
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
