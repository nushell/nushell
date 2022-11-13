use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, Signature, Span, Spanned,
    SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Window;

impl Command for Window {
    fn name(&self) -> &str {
        "window"
    }

    fn signature(&self) -> Signature {
        Signature::build("window")
            .input_output_types(vec![(
                Type::List(Box::new(Type::Any)),
                Type::List(Box::new(Type::List(Box::new(Type::Any)))),
            )])
            .required("window_size", SyntaxShape::Int, "the size of each window")
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

    fn usage(&self) -> &str {
        "Creates a sliding window of `window_size` that slide by n rows/elements across input."
    }

    fn examples(&self) -> Vec<Example> {
        let stream_test_1 = vec![
            Value::List {
                vals: vec![
                    Value::Int {
                        val: 1,
                        span: Span::test_data(),
                    },
                    Value::Int {
                        val: 2,
                        span: Span::test_data(),
                    },
                ],
                span: Span::test_data(),
            },
            Value::List {
                vals: vec![
                    Value::Int {
                        val: 2,
                        span: Span::test_data(),
                    },
                    Value::Int {
                        val: 3,
                        span: Span::test_data(),
                    },
                ],
                span: Span::test_data(),
            },
            Value::List {
                vals: vec![
                    Value::Int {
                        val: 3,
                        span: Span::test_data(),
                    },
                    Value::Int {
                        val: 4,
                        span: Span::test_data(),
                    },
                ],
                span: Span::test_data(),
            },
        ];

        let stream_test_2 = vec![
            Value::List {
                vals: vec![
                    Value::Int {
                        val: 1,
                        span: Span::test_data(),
                    },
                    Value::Int {
                        val: 2,
                        span: Span::test_data(),
                    },
                ],
                span: Span::test_data(),
            },
            Value::List {
                vals: vec![
                    Value::Int {
                        val: 4,
                        span: Span::test_data(),
                    },
                    Value::Int {
                        val: 5,
                        span: Span::test_data(),
                    },
                ],
                span: Span::test_data(),
            },
            Value::List {
                vals: vec![
                    Value::Int {
                        val: 7,
                        span: Span::test_data(),
                    },
                    Value::Int {
                        val: 8,
                        span: Span::test_data(),
                    },
                ],
                span: Span::test_data(),
            },
        ];

        let stream_test_3 = vec![
            Value::List {
                vals: vec![
                    Value::Int {
                        val: 1,
                        span: Span::test_data(),
                    },
                    Value::Int {
                        val: 2,
                        span: Span::test_data(),
                    },
                    Value::Int {
                        val: 3,
                        span: Span::test_data(),
                    },
                ],
                span: Span::test_data(),
            },
            Value::List {
                vals: vec![
                    Value::Int {
                        val: 4,
                        span: Span::test_data(),
                    },
                    Value::Int {
                        val: 5,
                        span: Span::test_data(),
                    },
                ],
                span: Span::test_data(),
            },
        ];

        vec![
            Example {
                example: "[1 2 3 4] | window 2",
                description: "A sliding window of two elements",
                result: Some(Value::List {
                    vals: stream_test_1,
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[1, 2, 3, 4, 5, 6, 7, 8] | window 2 --stride 3",
                description: "A sliding window of two elements, with a stride of 3",
                result: Some(Value::List {
                    vals: stream_test_2,
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[1, 2, 3, 4, 5] | window 3 --stride 3 --remainder",
                description: "A sliding window of equal stride that includes remainder. Equivalent to chunking",
                result: Some(Value::List {
                    vals: stream_test_3,
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
        let group_size: Spanned<usize> = call.req(engine_state, stack, 0)?;
        let ctrlc = engine_state.ctrlc.clone();
        let metadata = input.metadata();
        let stride: Option<usize> = call.get_flag(engine_state, stack, "stride")?;
        let remainder = call.has_flag("remainder");

        let stride = stride.unwrap_or(1);

        //FIXME: add in support for external redirection when engine-q supports it generally

        let each_group_iterator = EachWindowIterator {
            group_size: group_size.item,
            input: Box::new(input.into_iter()),
            span: call.head,
            previous: None,
            stride,
            remainder,
        };

        Ok(each_group_iterator
            .into_pipeline_data(ctrlc)
            .set_metadata(metadata))
    }
}

struct EachWindowIterator {
    group_size: usize,
    input: Box<dyn Iterator<Item = Value> + Send>,
    span: Span,
    previous: Option<Vec<Value>>,
    stride: usize,
    remainder: bool,
}

impl Iterator for EachWindowIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        let mut group = self.previous.take().unwrap_or_else(|| {
            let mut vec = Vec::new();

            // We default to a Vec of capacity size + stride as striding pushes n extra elements to the end
            vec.try_reserve(self.group_size + self.stride).ok();

            vec
        });
        let mut current_count = 0;

        if group.is_empty() {
            loop {
                let item = self.input.next();

                match item {
                    Some(v) => {
                        group.push(v);

                        current_count += 1;
                        if current_count >= self.group_size {
                            break;
                        }
                    }
                    None => {
                        if self.remainder {
                            break;
                        } else {
                            return None;
                        }
                    }
                }
            }
        } else {
            // our historic buffer is already full, so stride instead

            loop {
                let item = self.input.next();

                match item {
                    Some(v) => {
                        group.push(v);

                        current_count += 1;
                        if current_count >= self.stride {
                            break;
                        }
                    }
                    None => {
                        if self.remainder {
                            break;
                        } else {
                            return None;
                        }
                    }
                }
            }

            // We now have elements + stride in our group, and need to
            // drop the skipped elements. Drain to preserve allocation and capacity
            // Dropping this iterator consumes it.
            group.drain(..self.stride.min(group.len()));
        }

        if group.is_empty() {
            return None;
        }

        let return_group = group.clone();
        self.previous = Some(group);

        Some(Value::List {
            vals: return_group,
            span: self.span,
        })
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
