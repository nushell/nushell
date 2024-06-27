use std::borrow::Cow;

use nu_engine::{command_prelude::*, compile};
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct ViewIr;

impl Command for ViewIr {
    fn name(&self) -> &str {
        "view ir"
    }

    fn signature(&self) -> Signature {
        Signature::new(self.name()).required(
            "closure",
            SyntaxShape::Closure(None),
            "the closure to see compiled code for",
        )
    }

    fn usage(&self) -> &str {
        "View the compiled IR code for a block of code"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let closure: Closure = call.req(engine_state, stack, 0)?;

        let block = engine_state.get_block(closure.block_id);
        // Use the pre-compiled block if available, otherwise try to compile it
        // This helps display the actual compilation error
        let ir_block = match &block.ir_block {
            Some(ir_block) => Cow::Borrowed(ir_block),
            None => Cow::Owned(
                compile(&StateWorkingSet::new(engine_state), &block)
                    .map_err(|err| err.to_shell_error(block.span))?,
            ),
        };

        let formatted = format!("{}", ir_block.display(engine_state));
        Ok(Value::string(formatted, call.head).into_pipeline_data())
    }
}
