use super::clipboard::provider::{Clipboard, create_clipboard};
use crate::convert_json_string_to_value;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct ClipPaste;

impl Command for ClipPaste {
    fn name(&self) -> &str {
        "clip paste"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .switch("raw", "Disable JSON parsing.", Some('r'))
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .category(Category::System)
    }

    fn description(&self) -> &str {
        "Output the current clipboard content."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let text = create_clipboard(None).get_text()?;
        if text.trim().is_empty() {
            return Err(ShellError::GenericError {
                error: "Clipboard is empty.".into(),
                msg: "No text data is currently available in the clipboard.".into(),
                span: Some(call.head),
                help: None,
                inner: vec![],
            });
        }

        if call.has_flag(engine_state, stack, "raw")? {
            return Ok(Value::string(text, call.head).into_pipeline_data());
        }

        let trimmed = text.trim_start();
        if trimmed.starts_with('{') || trimmed.starts_with('[') || trimmed.starts_with('"') {
            return match convert_json_string_to_value(trimmed, call.head) {
                Ok(value) => Ok(value.into_pipeline_data()),
                Err(_) => Ok(Value::string(text, call.head).into_pipeline_data()),
            };
        }

        Ok(Value::string(text, call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "clip paste",
                description: "Paste from clipboard and try to parse JSON.",
                result: None,
            },
            Example {
                example: "clip paste --raw",
                description: "Paste raw clipboard text without JSON parsing.",
                result: None,
            },
        ]
    }
}
