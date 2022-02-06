use nu_engine::{eval_block_with_redirect, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{CaptureBlock, Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, Signature, SyntaxShape};

#[derive(Clone)]
pub struct Where;

impl Command for Where {
    fn name(&self) -> &str {
        "where"
    }

    fn usage(&self) -> &str {
        "Filter values based on a condition."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("where")
            .required("cond", SyntaxShape::RowCondition, "condition")
            .category(Category::Filters)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let span = call.head;

        let metadata = input.metadata();

        let block: CaptureBlock = call.req(engine_state, stack, 0)?;
        let mut stack = stack.captures_to_stack(&block.captures);
        let block = engine_state.get_block(block.block_id).clone();

        let ctrlc = engine_state.ctrlc.clone();
        let engine_state = engine_state.clone();

        input
            .filter(
                move |value| {
                    if let Some(var) = block.signature.get_positional(0) {
                        if let Some(var_id) = &var.var_id {
                            stack.add_var(*var_id, value.clone());
                        }
                    }
                    let result = eval_block_with_redirect(
                        &engine_state,
                        &mut stack,
                        &block,
                        PipelineData::new(span),
                    );

                    match result {
                        Ok(result) => result.into_value(span).is_true(),
                        _ => false,
                    }
                },
                ctrlc,
            )
            .map(|x| x.set_metadata(metadata))
    }
}
