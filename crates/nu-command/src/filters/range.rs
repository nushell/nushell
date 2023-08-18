use nu_engine::CallExt;

use nu_protocol::ast::{Call, RangeInclusion};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Range;

impl Command for Range {
    fn name(&self) -> &str {
        "range"
    }

    fn signature(&self) -> Signature {
        Signature::build("range")
            .input_output_types(vec![(
                Type::List(Box::new(Type::Any)),
                Type::List(Box::new(Type::Any)),
            )])
            .required(
                "rows",
                SyntaxShape::Range,
                "range of rows to return: Eg) 4..7 (=> from 4 to 7)",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Return only the selected rows."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[0,1,2,3,4,5] | range 4..5",
                description: "Get the last 2 items",
                result: Some(Value::List {
                    vals: vec![Value::test_int(4), Value::test_int(5)],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[0,1,2,3,4,5] | range (-2)..",
                description: "Get the last 2 items",
                result: Some(Value::List {
                    vals: vec![Value::test_int(4), Value::test_int(5)],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[0,1,2,3,4,5] | range (-3)..-2",
                description: "Get the next to last 2 items",
                result: Some(Value::List {
                    vals: vec![Value::test_int(3), Value::test_int(4)],
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
        let metadata = input.metadata();
        let rows: nu_protocol::Range = call.req(engine_state, stack, 0)?;

        let rows_from = get_range_val(rows.from);
        let rows_to = if rows.inclusion == RangeInclusion::RightExclusive {
            get_range_val(rows.to) - 1
        } else {
            get_range_val(rows.to)
        };

        // only collect the input if we have any negative indices
        if rows_from < 0 || rows_to < 0 {
            let v: Vec<_> = input.into_iter().collect();
            let vlen: i64 = v.len() as i64;

            let from = if rows_from < 0 {
                (vlen + rows_from) as usize
            } else {
                rows_from as usize
            };

            let to = if rows_to < 0 {
                (vlen + rows_to) as usize
            } else if rows_to > v.len() as i64 {
                v.len()
            } else {
                rows_to as usize
            };

            if from > to {
                Ok(PipelineData::Value(
                    Value::Nothing { span: call.head },
                    None,
                ))
            } else {
                let iter = v.into_iter().skip(from).take(to - from + 1);
                Ok(iter.into_pipeline_data(engine_state.ctrlc.clone()))
            }
        } else {
            let from = rows_from as usize;
            let to = rows_to as usize;

            if from > to {
                Ok(PipelineData::Value(
                    Value::Nothing { span: call.head },
                    None,
                ))
            } else {
                let iter = input.into_iter().skip(from).take(to - from + 1);
                Ok(iter.into_pipeline_data(engine_state.ctrlc.clone()))
            }
        }
        .map(|x| x.set_metadata(metadata))
    }
}

fn get_range_val(rows_val: Value) -> i64 {
    match rows_val {
        Value::Int { val: x, .. } => x,
        _ => 0,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Range {})
    }
}
