use nu_engine::eval_block;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};

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
        let cond = &call.positional[0];
        let span = call.head;

        let metadata = input.metadata();

        let block_id = cond
            .as_row_condition_block()
            .ok_or_else(|| ShellError::TypeMismatch("expected row condition".to_owned(), span))?;

        let ctrlc = engine_state.ctrlc.clone();
        let engine_state = engine_state.clone();

        let block = engine_state.get_block(block_id).clone();
        let mut stack = stack.collect_captures(&block.captures);

        input
            .filter(
                move |value| {
                    if let Some(var) = block.signature.get_positional(0) {
                        if let Some(var_id) = &var.var_id {
                            stack.add_var(*var_id, value.clone());
                        }
                    }
                    let result =
                        eval_block(&engine_state, &mut stack, &block, PipelineData::new(span));

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
