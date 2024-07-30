use nu_engine::command_prelude::*;
use nu_protocol::ListStream;
use std::num::NonZeroUsize;

#[derive(Clone)]
pub struct Chunks;

impl Command for Chunks {
    fn name(&self) -> &str {
        "chunks"
    }

    fn signature(&self) -> Signature {
        Signature::build("chunks")
            .input_output_types(vec![
                (Type::table(), Type::list(Type::table())),
                (Type::list(Type::Any), Type::list(Type::list(Type::Any))),
            ])
            .required("chunk_size", SyntaxShape::Int, "The size of each chunk.")
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Divide a list or table into chunks of `chunk_size`."
    }

    fn extra_usage(&self) -> &str {
        "This command will error if `chunk_size` is negative or zero."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["batch", "group"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[1 2 3 4] | chunks 2",
                description: "Chunk a list into pairs",
                result: Some(Value::test_list(vec![
                    Value::test_list(vec![Value::test_int(1), Value::test_int(2)]),
                    Value::test_list(vec![Value::test_int(3), Value::test_int(4)]),
                ])),
            },
            Example {
                example: "[[foo bar]; [0 1] [2 3] [4 5] [6 7] [8 9]] | chunks 3",
                description: "Chunk the rows of a table into triplets",
                result: Some(Value::test_list(vec![
                    Value::test_list(vec![
                        Value::test_record(record! {
                            "foo" => Value::test_int(0),
                            "bar" => Value::test_int(1),
                        }),
                        Value::test_record(record! {
                            "foo" => Value::test_int(2),
                            "bar" => Value::test_int(3),
                        }),
                        Value::test_record(record! {
                            "foo" => Value::test_int(4),
                            "bar" => Value::test_int(5),
                        }),
                    ]),
                    Value::test_list(vec![
                        Value::test_record(record! {
                            "foo" => Value::test_int(6),
                            "bar" => Value::test_int(7),
                        }),
                        Value::test_record(record! {
                            "foo" => Value::test_int(8),
                            "bar" => Value::test_int(9),
                        }),
                    ]),
                ])),
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
        let chunk_size: Value = call.req(engine_state, stack, 0)?;

        let size =
            usize::try_from(chunk_size.as_int()?).map_err(|_| ShellError::NeedsPositiveValue {
                span: chunk_size.span(),
            })?;

        let size = NonZeroUsize::try_from(size).map_err(|_| ShellError::IncorrectValue {
            msg: "`chunk_size` cannot be zero".into(),
            val_span: chunk_size.span(),
            call_span: head,
        })?;

        chunks(engine_state, input, size, head)
    }
}

pub fn chunks(
    engine_state: &EngineState,
    input: PipelineData,
    chunk_size: NonZeroUsize,
    span: Span,
) -> Result<PipelineData, ShellError> {
    match input {
        PipelineData::Value(Value::List { vals, .. }, metadata) => {
            let chunks = ChunksIter::new(vals, chunk_size, span);
            let stream = ListStream::new(chunks, span, engine_state.signals().clone());
            Ok(PipelineData::ListStream(stream, metadata))
        }
        PipelineData::ListStream(stream, metadata) => {
            let stream = stream.modify(|iter| ChunksIter::new(iter, chunk_size, span));
            Ok(PipelineData::ListStream(stream, metadata))
        }
        input => Err(input.unsupported_input_error("list", span)),
    }
}

struct ChunksIter<I: Iterator<Item = Value>> {
    iter: I,
    size: usize,
    span: Span,
}

impl<I: Iterator<Item = Value>> ChunksIter<I> {
    fn new(iter: impl IntoIterator<IntoIter = I>, size: NonZeroUsize, span: Span) -> Self {
        Self {
            iter: iter.into_iter(),
            size: size.into(),
            span,
        }
    }
}

impl<I: Iterator<Item = Value>> Iterator for ChunksIter<I> {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        let first = self.iter.next()?;
        let mut chunk = Vec::with_capacity(self.size); // delay allocation to optimize for empty iter
        chunk.push(first);
        chunk.extend((&mut self.iter).take(self.size - 1));
        Some(Value::list(chunk, self.span))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Chunks {})
    }
}
