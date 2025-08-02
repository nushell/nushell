use nu_engine::command_prelude::*;
use nu_protocol::ListStream;
use std::num::NonZeroUsize;

#[derive(Clone)]
pub struct Window;

impl Command for Window {
    fn name(&self) -> &str {
        "window"
    }

    fn signature(&self) -> Signature {
        Signature::build("window")
            .input_output_types(vec![(
                Type::list(Type::Any),
                Type::list(Type::list(Type::Any)),
            )])
            .required("window_size", SyntaxShape::Int, "The size of each window.")
            .named(
                "stride",
                SyntaxShape::Int,
                "the number of rows to slide over between windows",
                Some('s'),
            )
            .switch(
                "remainder",
                "yield last chunks even if they have fewer elements than size",
                Some('r'),
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Creates a sliding window of `window_size` that slide by n rows/elements across input."
    }

    fn extra_description(&self) -> &str {
        "This command will error if `window_size` or `stride` are negative or zero."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[1 2 3 4] | window 2",
                description: "A sliding window of two elements",
                result: Some(Value::test_list(vec![
                    Value::test_list(vec![Value::test_int(1), Value::test_int(2)]),
                    Value::test_list(vec![Value::test_int(2), Value::test_int(3)]),
                    Value::test_list(vec![Value::test_int(3), Value::test_int(4)]),
                ])),
            },
            Example {
                example: "[1, 2, 3, 4, 5, 6, 7, 8] | window 2 --stride 3",
                description: "A sliding window of two elements, with a stride of 3",
                result: Some(Value::test_list(vec![
                    Value::test_list(vec![Value::test_int(1), Value::test_int(2)]),
                    Value::test_list(vec![Value::test_int(4), Value::test_int(5)]),
                    Value::test_list(vec![Value::test_int(7), Value::test_int(8)]),
                ])),
            },
            Example {
                example: "[1, 2, 3, 4, 5] | window 3 --stride 3 --remainder",
                description: "A sliding window of equal stride that includes remainder. Equivalent to chunking",
                result: Some(Value::test_list(vec![
                    Value::test_list(vec![
                        Value::test_int(1),
                        Value::test_int(2),
                        Value::test_int(3),
                    ]),
                    Value::test_list(vec![Value::test_int(4), Value::test_int(5)]),
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
        let window_size: Value = call.req(engine_state, stack, 0)?;
        let stride: Option<Value> = call.get_flag(engine_state, stack, "stride")?;
        let remainder = call.has_flag(engine_state, stack, "remainder")?;

        let size =
            usize::try_from(window_size.as_int()?).map_err(|_| ShellError::NeedsPositiveValue {
                span: window_size.span(),
            })?;

        let size = NonZeroUsize::try_from(size).map_err(|_| ShellError::IncorrectValue {
            msg: "`window_size` cannot be zero".into(),
            val_span: window_size.span(),
            call_span: head,
        })?;

        let stride = if let Some(stride_val) = stride {
            let stride = usize::try_from(stride_val.as_int()?).map_err(|_| {
                ShellError::NeedsPositiveValue {
                    span: stride_val.span(),
                }
            })?;

            NonZeroUsize::try_from(stride).map_err(|_| ShellError::IncorrectValue {
                msg: "`stride` cannot be zero".into(),
                val_span: stride_val.span(),
                call_span: head,
            })?
        } else {
            NonZeroUsize::MIN
        };

        if remainder && size == stride {
            super::chunks::chunks(engine_state, input, size, head)
        } else if stride >= size {
            match input {
                PipelineData::Value(Value::List { vals, .. }, metadata) => {
                    let chunks = WindowGapIter::new(vals, size, stride, remainder, head);
                    let stream = ListStream::new(chunks, head, engine_state.signals().clone());
                    Ok(PipelineData::list_stream(stream, metadata))
                }
                PipelineData::ListStream(stream, metadata) => {
                    let stream = stream
                        .modify(|iter| WindowGapIter::new(iter, size, stride, remainder, head));
                    Ok(PipelineData::list_stream(stream, metadata))
                }
                input => Err(input.unsupported_input_error("list", head)),
            }
        } else {
            match input {
                PipelineData::Value(Value::List { vals, .. }, metadata) => {
                    let chunks = WindowOverlapIter::new(vals, size, stride, remainder, head);
                    let stream = ListStream::new(chunks, head, engine_state.signals().clone());
                    Ok(PipelineData::list_stream(stream, metadata))
                }
                PipelineData::ListStream(stream, metadata) => {
                    let stream = stream
                        .modify(|iter| WindowOverlapIter::new(iter, size, stride, remainder, head));
                    Ok(PipelineData::list_stream(stream, metadata))
                }
                input => Err(input.unsupported_input_error("list", head)),
            }
        }
    }
}

struct WindowOverlapIter<I: Iterator<Item = Value>> {
    iter: I,
    window: Vec<Value>,
    stride: usize,
    remainder: bool,
    span: Span,
}

impl<I: Iterator<Item = Value>> WindowOverlapIter<I> {
    fn new(
        iter: impl IntoIterator<IntoIter = I>,
        size: NonZeroUsize,
        stride: NonZeroUsize,
        remainder: bool,
        span: Span,
    ) -> Self {
        Self {
            iter: iter.into_iter(),
            window: Vec::with_capacity(size.into()),
            stride: stride.into(),
            remainder,
            span,
        }
    }
}

impl<I: Iterator<Item = Value>> Iterator for WindowOverlapIter<I> {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        let len = if self.window.is_empty() {
            self.window.capacity()
        } else {
            self.stride
        };

        self.window.extend((&mut self.iter).take(len));

        if self.window.len() == self.window.capacity()
            || (self.remainder && !self.window.is_empty())
        {
            let mut next = Vec::with_capacity(self.window.len());
            next.extend(self.window.iter().skip(self.stride).cloned());
            let window = std::mem::replace(&mut self.window, next);
            Some(Value::list(window, self.span))
        } else {
            None
        }
    }
}

struct WindowGapIter<I: Iterator<Item = Value>> {
    iter: I,
    size: usize,
    skip: usize,
    first: bool,
    remainder: bool,
    span: Span,
}

impl<I: Iterator<Item = Value>> WindowGapIter<I> {
    fn new(
        iter: impl IntoIterator<IntoIter = I>,
        size: NonZeroUsize,
        stride: NonZeroUsize,
        remainder: bool,
        span: Span,
    ) -> Self {
        let size = size.into();
        Self {
            iter: iter.into_iter(),
            size,
            skip: stride.get() - size,
            first: true,
            remainder,
            span,
        }
    }
}

impl<I: Iterator<Item = Value>> Iterator for WindowGapIter<I> {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        let mut window = Vec::with_capacity(self.size);
        window.extend(
            (&mut self.iter)
                .skip(if self.first { 0 } else { self.skip })
                .take(self.size),
        );

        self.first = false;

        if window.len() == self.size || (self.remainder && !window.is_empty()) {
            Some(Value::list(window, self.span))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Window {})
    }
}
