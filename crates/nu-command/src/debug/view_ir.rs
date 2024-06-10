use nu_engine::{command_prelude::*, compile};

#[derive(Clone)]
pub struct ViewIr;

impl Command for ViewIr {
    fn name(&self) -> &str {
        "view ir"
    }

    fn signature(&self) -> Signature {
        Signature::new(self.name()).required(
            "block",
            SyntaxShape::Block,
            "the block to see compiled code for",
        )
    }

    fn usage(&self) -> &str {
        "View the compiled IR code for a block"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let expr = call
            .positional_nth(0)
            .ok_or_else(|| ShellError::AccessEmptyContent { span: call.head })?;

        let block_id = expr.as_block().ok_or_else(|| ShellError::TypeMismatch {
            err_message: "expected block".into(),
            span: expr.span,
        })?;

        let block = engine_state.get_block(block_id);
        let ir_block = compile(engine_state, &block)?;

        let formatted = format!("{}", ir_block.display(engine_state));
        Ok(Value::string(formatted, call.head).into_pipeline_data())
    }
}
