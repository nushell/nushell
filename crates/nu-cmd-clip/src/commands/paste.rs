use crate::clipboard::clipboard::{Clipboard, create_clipboard};
use crate::utils::json::json_to_value;
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
        let text = create_clipboard().get_text()?;
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
            return match nu_json::from_str::<nu_json::Value>(&text) {
                Ok(value) => json_to_value(value, call.head).map(|v| v.into_pipeline_data()),
                Err(nu_json::Error::Syntax(_, _, _)) => {
                    Ok(Value::string(trimmed, call.head).into_pipeline_data())
                }
                Err(err) => Err(ShellError::GenericError {
                    error: "Failed to deserialize clipboard JSON.".into(),
                    msg: err.to_string(),
                    span: Some(call.head),
                    help: None,
                    inner: vec![],
                }),
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
