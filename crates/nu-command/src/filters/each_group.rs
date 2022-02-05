use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{CaptureBlock, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, Signature,
    Span, Spanned, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct EachGroup;

impl Command for EachGroup {
    fn name(&self) -> &str {
        "each group"
    }

    fn signature(&self) -> Signature {
        Signature::build("each group")
            .required("group_size", SyntaxShape::Int, "the size of each group")
            .required(
                "block",
                SyntaxShape::Block(Some(vec![SyntaxShape::Any])),
                "the block to run on each group",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Runs a block on groups of `group_size` rows of a table at a time."
    }

    fn examples(&self) -> Vec<Example> {
        let stream_test_1 = vec![
            Value::Int {
                val: 3,
                span: Span::test_data(),
            },
            Value::Int {
                val: 7,
                span: Span::test_data(),
            },
        ];

        vec![Example {
            example: "echo [1 2 3 4] | each group 2 { $it.0 + $it.1 }",
            description: "Multiplies elements in list",
            result: Some(Value::List {
                vals: stream_test_1,
                span: Span::test_data(),
            }),
        }]
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

        //FIXME: add in support for external redirection when engine-q supports it generally

        let each_group_iterator = EachGroupIterator {
            block: capture_block,
            engine_state: engine_state.clone(),
            stack: stack.clone(),
            group_size: group_size.item,
            input: Box::new(input.into_iter()),
            span: call.head,
        };

        Ok(each_group_iterator.flatten().into_pipeline_data(ctrlc))
    }
}

struct EachGroupIterator {
    block: CaptureBlock,
    engine_state: EngineState,
    stack: Stack,
    group_size: usize,
    input: Box<dyn Iterator<Item = Value> + Send>,
    span: Span,
}

impl Iterator for EachGroupIterator {
    type Item = PipelineData;

    fn next(&mut self) -> Option<Self::Item> {
        let mut group = vec![];
        let mut current_count = 0;

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
                None => break,
            }
        }

        if group.is_empty() {
            return None;
        }

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
) -> PipelineData {
    let value = Value::List { vals: input, span };

    let mut stack = stack.captures_to_stack(&capture_block.captures);

    let block = engine_state.get_block(capture_block.block_id);

    if let Some(var) = block.signature.get_positional(0) {
        if let Some(var_id) = &var.var_id {
            stack.add_var(*var_id, value);
        }
    }

    match eval_block(&engine_state, &mut stack, block, PipelineData::new(span)) {
        Ok(pipeline) => pipeline,
        Err(error) => Value::Error { error }.into_pipeline_data(),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(EachGroup {})
    }
}
