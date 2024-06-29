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
        Signature::new(self.name())
            .required(
                "closure",
                SyntaxShape::Closure(None),
                "the closure to see compiled code for",
            )
            .switch("json", "Dump the raw block data as JSON", Some('j'))
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
        let json = call.has_flag(engine_state, stack, "json")?;

        let block = engine_state.get_block(closure.block_id);
        // Use the pre-compiled block if available, otherwise try to compile it
        // This helps display the actual compilation error
        let ir_block = match &block.ir_block {
            Some(ir_block) => Cow::Borrowed(ir_block),
            None => Cow::Owned(
                compile(&StateWorkingSet::new(engine_state), &block).map_err(|err| {
                    ShellError::IrCompileError {
                        span: block.span,
                        errors: vec![err],
                    }
                })?,
            ),
        };

        let formatted = if json {
            let formatted_instructions = ir_block
                .instructions
                .iter()
                .map(|instruction| {
                    instruction
                        .display(engine_state, &ir_block.data)
                        .to_string()
                })
                .collect::<Vec<_>>();

            serde_json::to_string_pretty(&serde_json::json!({
                "block_id": closure.block_id,
                "span": block.span,
                "ir_block": ir_block,
                "formatted_instructions": formatted_instructions,
            }))
            .map_err(|err| ShellError::GenericError {
                error: "JSON serialization failed".into(),
                msg: err.to_string(),
                span: Some(call.head),
                help: None,
                inner: vec![],
            })?
        } else {
            format!("{}", ir_block.display(engine_state))
        };

        Ok(Value::string(formatted, call.head).into_pipeline_data())
    }
}
