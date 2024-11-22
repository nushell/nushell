use super::utils::chain_error_with_input;
use nu_engine::{command_prelude::*, ClosureEval};
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct PartitionBy;

impl Command for PartitionBy {
    fn name(&self) -> &str {
        "partition-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("partition-by")
            .input_output_types(vec![(
                Type::List(Box::new(Type::Any)),
                Type::List(Box::new(Type::Any)),
            )])
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "The closure to run.",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        r#"Divides a sequence into sub-sequences based on function."#
    }

    fn extra_description(&self) -> &str {
        r#"partition-by applies the given closure to each value of the input list, and groups
consecutive elements that share the same closure result value into lists."#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        partition_by(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Partition data into runs of larger than zero or not.",
                example: "[1, 3, -2, -2, 0, 1, 2] | partition-by {|it| $it >= 0 }",
                result: Some(Value::test_list(vec![
                    Value::test_list(vec![Value::test_int(1), Value::test_int(3)]),
                    Value::test_list(vec![Value::test_int(-2), Value::test_int(-2)]),
                    Value::test_list(vec![
                        Value::test_int(0),
                        Value::test_int(1),
                        Value::test_int(2),
                    ]),
                ])),
            },
            Example {
                description: "Identify repetitions in a string",
                example: r#"[a b b c c c] | partition-by { |it| $it }"#,
                result: Some(Value::test_list(vec![
                    Value::test_list(vec![Value::test_string("a")]),
                    Value::test_list(vec![Value::test_string("b"), Value::test_string("b")]),
                    Value::test_list(vec![
                        Value::test_string("c"),
                        Value::test_string("c"),
                        Value::test_string("c"),
                    ]),
                ])),
            },
        ]
    }
}

struct Partition<I, T, F, K> {
    iterator: I,
    last_value: Option<(T, K)>,
    closure: F,
    done: bool,
}

impl<I, T, F, K> Iterator for Partition<I, T, F, K>
where
    I: Iterator<Item = T>,
    F: FnMut(&T) -> K,
    K: PartialEq,
{
    type Item = Vec<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let (head, head_key) = match self.last_value.take() {
            None => {
                let head = self.iterator.next()?;

                let key = (self.closure)(&head);

                (head, key)
            }

            Some((value, key)) => (value, key),
        };

        let mut result = vec![head];

        loop {
            match self.iterator.next() {
                None => {
                    self.done = true;
                    return Some(result);
                }
                Some(value) => {
                    let value_key = (self.closure)(&value);

                    if value_key == head_key {
                        result.push(value);
                    } else {
                        self.last_value = Some((value, value_key));
                        return Some(result);
                    }
                }
            }
        }
    }
}

/// An iterator with the semantics of the partition_by operation.
fn partition_iter_by<I, T, F, K>(iterator: I, closure: F) -> Partition<I, T, F, K>
where
    I: Iterator<Item = T>,
    F: FnMut(&T) -> K,
    K: PartialEq,
{
    Partition {
        closure,
        iterator,
        last_value: None,
        done: false,
    }
}

pub fn partition_by(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let closure: Closure = call.req(engine_state, stack, 0)?;

    let metadata = input.metadata();

    match input {
        PipelineData::Empty => Ok(PipelineData::Empty),
        PipelineData::Value(Value::Range { .. }, ..)
        | PipelineData::Value(Value::List { .. }, ..)
        | PipelineData::ListStream(..) => {
            let closure = ClosureEval::new(engine_state, stack, closure);

            let result = partition_value_stream(input.into_iter(), closure, head);

            Ok(result.into_pipeline_data(head, engine_state.signals().clone()))
        }
        PipelineData::ByteStream(stream, ..) => {
            if let Some(chunks) = stream.chunks() {
                let closure = ClosureEval::new(engine_state, stack, closure);

                let mapped_chunks =
                    chunks.map(move |value| value.unwrap_or_else(|err| Value::error(err, head)));

                let result = partition_value_stream(mapped_chunks, closure, head);

                Ok(result.into_pipeline_data(head, engine_state.signals().clone()))
            } else {
                Ok(PipelineData::Empty)
            }
        }

        PipelineData::Value(..) => Err(input.unsupported_input_error("list", head)),
    }
    .map(|data| data.set_metadata(metadata))
}

fn partition_value_stream<I>(
    iterator: I,
    mut closure: ClosureEval,
    head: Span,
) -> impl Iterator<Item = Value> + 'static + Send
where
    I: Iterator<Item = Value> + 'static + Send,
{
    partition_iter_by(iterator, move |value| {
        match closure.run_with_value(value.clone()) {
            Ok(data) => data.into_value(head).unwrap_or_else(|error| {
                Value::error(chain_error_with_input(error, value.is_error(), head), head)
            }),

            Err(error) => Value::error(chain_error_with_input(error, value.is_error(), head), head),
        }
    })
    .map(move |it| Value::list(it, head))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(PartitionBy {})
    }
}
