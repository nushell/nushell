use nu_engine::{eval_block_with_redirect, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{CaptureBlock, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, Signature, Span, Spanned,
    SyntaxShape, Value,
};

#[derive(Clone)]
pub struct EachWindow;

impl Command for EachWindow {
    fn name(&self) -> &str {
        "each window"
    }

    fn signature(&self) -> Signature {
        Signature::build("each window")
            .required("window_size", SyntaxShape::Int, "the size of each window")
            .named(
                "stride",
                SyntaxShape::Int,
                "the number of rows to slide over between windows",
                Some('s'),
            )
            .required(
                "block",
                SyntaxShape::Block(Some(vec![SyntaxShape::Any])),
                "the block to run on each window",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Runs a block on window groups of `window_size` that slide by n rows."
    }

    fn examples(&self) -> Vec<Example> {
        let stream_test_1 = vec![
            Value::Int {
                val: 3,
                span: Span::test_data(),
            },
            Value::Int {
                val: 5,
                span: Span::test_data(),
            },
            Value::Int {
                val: 7,
                span: Span::test_data(),
            },
        ];

        let stream_test_2 = vec![
            Value::Int {
                val: 3,
                span: Span::test_data(),
            },
            Value::Int {
                val: 9,
                span: Span::test_data(),
            },
            Value::Int {
                val: 15,
                span: Span::test_data(),
            },
        ];

        vec![
            Example {
                example: "echo [1 2 3 4] | each window 2 { |it| $it.0 + $it.1 }",
                description: "A sliding window of two elements",
                result: Some(Value::List {
                    vals: stream_test_1,
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[1, 2, 3, 4, 5, 6, 7, 8] | each window 2 --stride 3 { |x| $x.0 + $x.1 }",
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
        let capture_block: CaptureBlock = call.req(engine_state, stack, 1)?;
        let ctrlc = engine_state.ctrlc.clone();
        let stride: Option<usize> = call.get_flag(engine_state, stack, "stride")?;

        let stride = stride.unwrap_or(1);

        //FIXME: add in support for external redirection when engine-q supports it generally

        let each_group_iterator = EachWindowIterator {
            block: capture_block,
            engine_state: engine_state.clone(),
            stack: stack.clone(),
            group_size: group_size.item,
            input: Box::new(input.into_iter()),
            span: call.head,
            previous: vec![],
            stride,
        };

        Ok(each_group_iterator.into_pipeline_data(ctrlc))
    }
}

struct EachWindowIterator {
    block: CaptureBlock,
    engine_state: EngineState,
    stack: Stack,
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

        Some(run_block_on_vec(
            group,
            self.block.clone(),
            self.engine_state.clone(),
            self.stack.clone(),
            self.span,
        ))
    }
}

pub(crate) fn run_block_on_vec(
    input: Vec<Value>,
    capture_block: CaptureBlock,
    engine_state: EngineState,
    stack: Stack,
    span: Span,
) -> Value {
    let value = Value::List { vals: input, span };

    let mut stack = stack.captures_to_stack(&capture_block.captures);

    let block = engine_state.get_block(capture_block.block_id);

    if let Some(var) = block.signature.get_positional(0) {
        if let Some(var_id) = &var.var_id {
            stack.add_var(*var_id, value);
        }
    }

    match eval_block_with_redirect(&engine_state, &mut stack, block, PipelineData::new(span)) {
        Ok(pipeline) => pipeline.into_value(span),
        Err(error) => Value::Error { error },
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(EachWindow {})
    }
}
