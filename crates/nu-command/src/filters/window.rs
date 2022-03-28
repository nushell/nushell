use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, Signature, Span, Spanned,
    SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Window;

impl Command for Window {
    fn name(&self) -> &str {
        "window"
    }

    fn signature(&self) -> Signature {
        Signature::build("window")
            .required("window_size", SyntaxShape::Int, "the size of each window")
            .named(
                "stride",
                SyntaxShape::Int,
                "the number of rows to slide over between windows",
                Some('s'),
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

        vec![
            Example {
                example: "echo [1 2 3 4] | window 2",
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

        let stride = stride.unwrap_or(1);

        //FIXME: add in support for external redirection when engine-q supports it generally

        let each_group_iterator = EachWindowIterator {
            group_size: group_size.item,
            input: Box::new(input.into_iter()),
            span: call.head,
            previous: vec![],
            stride,
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
    previous: Vec<Value>,
    stride: usize,
}

impl Iterator for EachWindowIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        let mut group = self.previous.clone();
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
                    None => return None,
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
                    None => return None,
                }
            }

            for _ in 0..current_count {
                let _ = group.remove(0);
            }
        }

        if group.is_empty() || current_count == 0 {
            return None;
        }

        self.previous = group.clone();

        Some(Value::List {
            vals: group,
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
