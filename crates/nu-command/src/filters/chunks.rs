use nu_engine::command_prelude::*;
use nu_protocol::{ListStream, shell_error::io::IoError};
use std::{
    io::{BufRead, Cursor, ErrorKind},
    num::NonZeroUsize,
};

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
                (Type::Binary, Type::list(Type::Binary)),
            ])
            .required("chunk_size", SyntaxShape::Int, "The size of each chunk.")
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Divide a list, table or binary input into chunks of `chunk_size`."
    }

    fn extra_description(&self) -> &str {
        "This command will error if `chunk_size` is negative or zero."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["batch", "group", "split", "bytes"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
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
            Example {
                example: "0x[11 22 33 44 55 66 77 88] | chunks 3",
                description: "Chunk the bytes of a binary into triplets",
                result: Some(Value::test_list(vec![
                    Value::test_binary(vec![0x11, 0x22, 0x33]),
                    Value::test_binary(vec![0x44, 0x55, 0x66]),
                    Value::test_binary(vec![0x77, 0x88]),
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
    let from_io_error = IoError::factory(span, None);
    match input {
        PipelineData::Value(Value::List { vals, .. }, metadata) => {
            let chunks = ChunksIter::new(vals, chunk_size, span);
            let stream = ListStream::new(chunks, span, engine_state.signals().clone());
            Ok(PipelineData::list_stream(stream, metadata))
        }
        PipelineData::ListStream(stream, metadata) => {
            let stream = stream.modify(|iter| ChunksIter::new(iter, chunk_size, span));
            Ok(PipelineData::list_stream(stream, metadata))
        }
        PipelineData::Value(Value::Binary { val, .. }, metadata) => {
            let chunk_read = ChunkRead {
                reader: Cursor::new(val),
                size: chunk_size,
            };
            let value_stream = chunk_read.map(move |chunk| match chunk {
                Ok(chunk) => Value::binary(chunk, span),
                Err(e) => Value::error(from_io_error(e).into(), span),
            });
            let pipeline_data_with_metadata = value_stream.into_pipeline_data_with_metadata(
                span,
                engine_state.signals().clone(),
                metadata,
            );
            Ok(pipeline_data_with_metadata)
        }
        PipelineData::ByteStream(stream, metadata) => {
            let pipeline_data = match stream.reader() {
                None => PipelineData::empty(),
                Some(reader) => {
                    let chunk_read = ChunkRead {
                        reader,
                        size: chunk_size,
                    };
                    let value_stream = chunk_read.map(move |chunk| match chunk {
                        Ok(chunk) => Value::binary(chunk, span),
                        Err(e) => Value::error(from_io_error(e).into(), span),
                    });
                    value_stream.into_pipeline_data_with_metadata(
                        span,
                        engine_state.signals().clone(),
                        metadata,
                    )
                }
            };
            Ok(pipeline_data)
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

struct ChunkRead<R: BufRead> {
    reader: R,
    size: NonZeroUsize,
}

impl<R: BufRead> Iterator for ChunkRead<R> {
    type Item = Result<Vec<u8>, std::io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf = Vec::with_capacity(self.size.get());
        while buf.len() < self.size.get() {
            let available = match self.reader.fill_buf() {
                Ok([]) if buf.is_empty() => return None,
                Ok([]) => return Some(Ok(buf)),
                Ok(n) => n,
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Some(Err(e)),
            };
            let needed = self.size.get() - buf.len();
            let have = available.len().min(needed);
            buf.extend_from_slice(&available[..have]);
            self.reader.consume(have);
        }
        Some(Ok(buf))
    }
}

#[cfg(test)]
mod test {
    use std::io::Read;

    use super::*;

    #[test]
    fn chunk_read() {
        let s = "hello world";
        let data = Cursor::new(s);
        let chunk_read = ChunkRead {
            reader: data,
            size: NonZeroUsize::new(4).unwrap(),
        };
        let chunks = chunk_read.map(|e| e.unwrap()).collect::<Vec<_>>();
        assert_eq!(
            chunks,
            [&s.as_bytes()[..4], &s.as_bytes()[4..8], &s.as_bytes()[8..]]
        );
    }

    #[test]
    fn chunk_read_stream() {
        let s = "hello world";
        let data = Cursor::new(&s[..3])
            .chain(Cursor::new(&s[3..9]))
            .chain(Cursor::new(&s[9..]));
        let chunk_read = ChunkRead {
            reader: data,
            size: NonZeroUsize::new(4).unwrap(),
        };
        let chunks = chunk_read.map(|e| e.unwrap()).collect::<Vec<_>>();
        assert_eq!(
            chunks,
            [&s.as_bytes()[..4], &s.as_bytes()[4..8], &s.as_bytes()[8..]]
        );
    }

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Chunks {})
    }
}
