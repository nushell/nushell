use nu_engine::command_prelude::*;
use nu_protocol::Range;
use std::ops::Bound;

#[derive(Clone)]
pub struct Slice;

impl Command for Slice {
    fn name(&self) -> &str {
        "slice"
    }

    fn signature(&self) -> Signature {
        Signature::build("slice")
            .input_output_types(vec![(
                Type::List(Box::new(Type::Any)),
                Type::List(Box::new(Type::Any)),
            )])
            .required("rows", SyntaxShape::Range, "Range of rows to return.")
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Return only the selected rows."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["filter", "head", "tail", "range"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[0,1,2,3,4,5] | slice 4..5",
                description: "Get the last 2 items",
                result: Some(Value::list(
                    vec![Value::test_int(4), Value::test_int(5)],
                    Span::test_data(),
                )),
            },
            Example {
                example: "[0,1,2,3,4,5] | slice (-2)..",
                description: "Get the last 2 items",
                result: Some(Value::list(
                    vec![Value::test_int(4), Value::test_int(5)],
                    Span::test_data(),
                )),
            },
            Example {
                example: "[0,1,2,3,4,5] | slice (-3)..-2",
                description: "Get the next to last 2 items",
                result: Some(Value::list(
                    vec![Value::test_int(3), Value::test_int(4)],
                    Span::test_data(),
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
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let metadata = input.metadata();
        let rows: Spanned<Range> = call.req(engine_state, stack, 0)?;

        match rows.item {
            Range::IntRange(range) => {
                let start = range.start();
                let end = match range.end() {
                    Bound::Included(end) => end,
                    Bound::Excluded(end) => end - 1,
                    Bound::Unbounded => {
                        if range.step() < 0 {
                            i64::MIN
                        } else {
                            i64::MAX
                        }
                    }
                };

                // only collect the input if we have any negative indices
                if start < 0 || end < 0 {
                    let v: Vec<_> = input.into_iter().collect();
                    let vlen: i64 = v.len() as i64;

                    let from = if start < 0 {
                        (vlen + start) as usize
                    } else {
                        start as usize
                    };

                    let to = if end < 0 {
                        (vlen + end) as usize
                    } else if end > v.len() as i64 {
                        v.len()
                    } else {
                        end as usize
                    };

                    if from > to {
                        Ok(PipelineData::Value(Value::nothing(head), None))
                    } else {
                        let iter = v.into_iter().skip(from).take(to - from + 1);
                        Ok(iter.into_pipeline_data(head, engine_state.signals().clone()))
                    }
                } else {
                    let from = start as usize;
                    let to = end as usize;

                    if from > to {
                        Ok(PipelineData::Value(Value::nothing(head), None))
                    } else {
                        let iter = input.into_iter().skip(from).take(to - from + 1);
                        Ok(iter.into_pipeline_data(head, engine_state.signals().clone()))
                    }
                }
                .map(|x| x.set_metadata(metadata))
            }
            Range::FloatRange(_) => Err(ShellError::UnsupportedInput {
                msg: "float range".into(),
                input: "value originates from here".into(),
                msg_span: call.head,
                input_span: rows.span,
            }),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Slice {})
    }
}
