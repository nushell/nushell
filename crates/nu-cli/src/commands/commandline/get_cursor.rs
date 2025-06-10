use nu_engine::command_prelude::*;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone)]
pub struct CommandlineGetCursor;

impl Command for CommandlineGetCursor {
    fn name(&self) -> &str {
        "commandline get-cursor"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::Nothing, Type::Int)])
            .allow_variants_without_examples(true)
            .category(Category::Core)
    }

    fn description(&self) -> &str {
        "Get the current cursor position."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["repl", "interactive"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let repl = engine_state.repl_state.lock().expect("repl state mutex");
        let char_pos = repl
            .buffer
            .grapheme_indices(true)
            .chain(std::iter::once((repl.buffer.len(), "")))
            .position(|(i, _c)| i == repl.cursor_pos)
            .expect("Cursor position isn't on a grapheme boundary");
        match i64::try_from(char_pos) {
            Ok(pos) => Ok(Value::int(pos, call.head).into_pipeline_data()),
            Err(e) => Err(ShellError::GenericError {
                error: "Failed to convert cursor position to int".to_string(),
                msg: e.to_string(),
                span: None,
                help: None,
                inner: vec![],
            }),
        }
    }
}
