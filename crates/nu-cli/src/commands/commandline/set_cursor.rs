use nu_engine::command_prelude::*;

use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "commandline set-cursor"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .switch(
                "end",
                "set the current cursor position to the end of the buffer",
                Some('e'),
            )
            .optional("pos", SyntaxShape::Int, "Cursor position to be set")
            .category(Category::Core)
    }

    fn usage(&self) -> &str {
        "Set the current cursor position."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["repl", "interactive"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let mut repl = engine_state.repl_state.lock().expect("repl state mutex");
        if let Some(pos) = call.opt::<i64>(engine_state, stack, 0)? {
            repl.cursor_pos = if pos <= 0 {
                0usize
            } else {
                repl.buffer
                    .grapheme_indices(true)
                    .map(|(i, _c)| i)
                    .nth(pos as usize)
                    .unwrap_or(repl.buffer.len())
            };
            Ok(Value::nothing(call.head).into_pipeline_data())
        } else if call.has_flag(engine_state, stack, "end")? {
            repl.cursor_pos = repl.buffer.len();
            Ok(Value::nothing(call.head).into_pipeline_data())
        } else {
            Err(ShellError::GenericError {
                error: "Required a positional argument or a flag".to_string(),
                msg: "".to_string(),
                span: None,
                help: None,
                inner: vec![],
            })
        }
    }
}
