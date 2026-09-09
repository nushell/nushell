use nu_engine::command_prelude::*;
use nu_protocol::shell_error::generic::GenericError;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone)]
pub struct CommandlineGetSelection;

impl Command for CommandlineGetSelection {
    fn name(&self) -> &str {
        "commandline get-selection"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (Type::Nothing, Type::record()),
                (Type::Nothing, Type::Nothing),
            ])
            .category(Category::Core)
    }

    fn description(&self) -> &str {
        "Get the current selection positions."
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

        match repl.selection {
            None => Ok(Value::nothing(call.head).into_pipeline_data()),
            Some((from, to)) => {
                let char_pos_start = repl
                    .buffer
                    .grapheme_indices(true)
                    .chain(std::iter::once((repl.buffer.len(), "")))
                    .position(|(i, _c)| i == from)
                    .expect("Selection start isn't on a grapheme boundary");
                let char_pos_end = repl
                    .buffer
                    .grapheme_indices(true)
                    .chain(std::iter::once((repl.buffer.len(), "")))
                    .position(|(i, _c)| i == to)
                    .expect("Selection end isn't on a grapheme boundary");
                match (i64::try_from(char_pos_start), i64::try_from(char_pos_end)) {
                    (Ok(pos_start), Ok(pos_end)) => Ok(Value::record(
                        record! {
                            "start" => Value::int(pos_start, call.head),
                            "end" => Value::int(pos_end, call.head),
                        },
                        call.head,
                    )
                    .into_pipeline_data()),
                    (Err(e), _) => Err(ShellError::Generic(GenericError::new_internal(
                        "Failed to convert selection start to int",
                        e.to_string(),
                    ))),
                    (_, Err(e)) => Err(ShellError::Generic(GenericError::new_internal(
                        "Failed to convert selection end to int",
                        e.to_string(),
                    ))),
                }
            }
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: r#"let s = commandline get-selection; commandline | str substring ($s.start?)..<($s.end?)"#,
            description: "Get the current selection content, or the full command line if none.",
            result: None,
        }]
    }
}
