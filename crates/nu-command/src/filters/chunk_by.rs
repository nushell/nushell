use super::utils::chain_error_with_input;
use nu_engine::{ClosureEval, command_prelude::*};
use nu_protocol::Signals;
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct ChunkBy;

impl Command for ChunkBy {
    fn name(&self) -> &str {
        "chunk-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("chunk-by")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::list(Type::list(Type::Any)),
                ),
                (Type::Range, Type::list(Type::list(Type::Any))),
            ])
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "The closure to run.",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        r#"Divides a sequence into sub-sequences based on a closure."#
    }

    fn extra_description(&self) -> &str {
        r#"chunk-by applies the given closure to each value of the input list, and groups
consecutive elements that share the same closure result value into lists."#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        chunk_by(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Chunk data into runs of larger than zero or not.",
                example: "[1, 3, -2, -2, 0, 1, 2] | chunk-by {|it| $it >= 0 }",
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
                example: r#"[a b b c c c] | chunk-by { |it| $it }"#,
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
            Example {
                description: "Chunk values of range by predicate",
                example: r#"(0..8) | chunk-by { |it| $it // 3 }"#,
                result: Some(Value::test_list(vec![
                    Value::test_list(vec![
                        Value::test_int(0),
                        Value::test_int(1),
                        Value::test_int(2),
                    ]),
                    Value::test_list(vec![
                        Value::test_int(3),
                        Value::test_int(4),
                        Value::test_int(5),
                    ]),
                    Value::test_list(vec![
                        Value::test_int(6),
                        Value::test_int(7),
                        Value::test_int(8),
                    ]),
                ])),
            },
        ]
    }
}

struct Chunk<I, T, F, K> {
    iterator: I,
    last_value: Option<(T, K)>,
    closure: F,
    done: bool,
    signals: Signals,
}

impl<I, T, F, K> Chunk<I, T, F, K>
where
    I: Iterator<Item = T>,
    F: FnMut(&T) -> K,
    K: PartialEq,
{
    fn inner_iterator_next(&mut self) -> Option<I::Item> {
        if self.signals.interrupted() {
            self.done = true;
            return None;
        }
        self.iterator.next()
    }
}

impl<I, T, F, K> Iterator for Chunk<I, T, F, K>
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
                let head = self.inner_iterator_next()?;

                let key = (self.closure)(&head);

                (head, key)
            }

            Some((value, key)) => (value, key),
        };

        let mut result = vec![head];

        loop {
            match self.inner_iterator_next() {
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

/// An iterator with the semantics of the chunk_by operation.
fn chunk_iter_by<I, T, F, K>(iterator: I, signals: Signals, closure: F) -> Chunk<I, T, F, K>
where
    I: Iterator<Item = T>,
    F: FnMut(&T) -> K,
    K: PartialEq,
{
    Chunk {
        closure,
        iterator,
        last_value: None,
        done: false,
        signals,
    }
}

pub fn chunk_by(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let closure: Closure = call.req(engine_state, stack, 0)?;

    let metadata = input.metadata();

    match input {
        PipelineData::Empty => Ok(PipelineData::empty()),
        PipelineData::Value(Value::Range { .. }, ..)
        | PipelineData::Value(Value::List { .. }, ..)
        | PipelineData::ListStream(..) => {
            let closure = ClosureEval::new(engine_state, stack, closure);

            let result = chunk_value_stream(
                input.into_iter(),
                closure,
                head,
                engine_state.signals().clone(),
            );

            Ok(result.into_pipeline_data(head, engine_state.signals().clone()))
        }

        PipelineData::ByteStream(..) | PipelineData::Value(..) => {
            Err(input.unsupported_input_error("list", head))
        }
    }
    .map(|data| data.set_metadata(metadata))
}

fn chunk_value_stream<I>(
    iterator: I,
    mut closure: ClosureEval,
    head: Span,
    signals: Signals,
) -> impl Iterator<Item = Value> + 'static + Send
where
    I: Iterator<Item = Value> + 'static + Send,
{
    chunk_iter_by(iterator, signals, move |value| {
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

        test_examples(ChunkBy {})
    }
}
