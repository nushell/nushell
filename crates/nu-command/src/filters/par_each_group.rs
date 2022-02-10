use nu_engine::{eval_block_with_redirect, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{CaptureBlock, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, Signature, Spanned,
    SyntaxShape, Value,
};
use rayon::prelude::*;

#[derive(Clone)]
pub struct ParEachGroup;

impl Command for ParEachGroup {
    fn name(&self) -> &str {
        "par_each group"
    }

    fn signature(&self) -> Signature {
        Signature::build("par_each group")
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
        vec![Example {
            example: "echo [1 2 3 4] | par_each group 2 { $it.0 + $it.1 }",
            description: "Multiplies elements in list",
            result: None,
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
        let span = call.head;

        let stack = stack.captures_to_stack(&capture_block.captures);

        //FIXME: add in support for external redirection when engine-q supports it generally

        let each_group_iterator = EachGroupIterator {
            group_size: group_size.item,
            input: Box::new(input.into_iter()),
        };

        Ok(each_group_iterator
            .par_bridge()
            .map(move |x| {
                let block = engine_state.get_block(capture_block.block_id);

                let mut stack = stack.clone();

                if let Some(var) = block.signature.get_positional(0) {
                    if let Some(var_id) = &var.var_id {
                        stack.add_var(*var_id, Value::List { vals: x, span });
                    }
                }

                match eval_block_with_redirect(
                    engine_state,
                    &mut stack,
                    block,
                    PipelineData::new(span),
                ) {
                    Ok(v) => v.into_value(span),
                    Err(error) => Value::Error { error },
                }
            })
            .collect::<Vec<_>>()
            .into_iter()
            .into_pipeline_data(ctrlc))
    }
}

struct EachGroupIterator {
    group_size: usize,
    input: Box<dyn Iterator<Item = Value> + Send>,
}

impl Iterator for EachGroupIterator {
    type Item = Vec<Value>;

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

        Some(group)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(ParEachGroup {})
    }
}
