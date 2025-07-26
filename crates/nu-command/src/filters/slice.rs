use nu_engine::command_prelude::*;
use nu_protocol::IntRange;
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
        let range: IntRange = call.req(engine_state, stack, 0)?;

        // only collect the input if we have any negative indices
        if range.is_relative() {
            let v: Vec<_> = input.into_iter().collect();

            let (from, to) = range.absolute_bounds(v.len());

            let count = match to {
                Bound::Excluded(to) => to.saturating_sub(from),
                Bound::Included(to) => to.saturating_sub(from) + 1,
                Bound::Unbounded => usize::MAX,
            };

            if count == 0 {
                Ok(PipelineData::value(Value::list(vec![], head), None))
            } else {
                let iter = v.into_iter().skip(from).take(count);
                Ok(iter.into_pipeline_data(head, engine_state.signals().clone()))
            }
        } else {
            let from = range.start() as usize;
            let count = match range.end() {
                Bound::Excluded(to) | Bound::Included(to) if range.start() > to => 0,
                Bound::Excluded(to) => (to as usize).saturating_sub(from),
                Bound::Included(to) => (to as usize).saturating_sub(from) + 1,
                Bound::Unbounded => {
                    if range.step() < 0 {
                        0
                    } else {
                        usize::MAX
                    }
                }
            };

            if count == 0 {
                Ok(PipelineData::value(Value::list(vec![], head), None))
            } else {
                let iter = input.into_iter().skip(from).take(count);
                Ok(iter.into_pipeline_data(head, engine_state.signals().clone()))
            }
        }
        .map(|x| x.set_metadata(metadata))
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
