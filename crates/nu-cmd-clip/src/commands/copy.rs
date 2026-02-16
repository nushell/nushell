use crate::clipboard::clipboard::{Clipboard, create_clipboard};
use crate::utils::json;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct ClipCopy;

impl ClipCopy {
    fn format_json(input: &Value, span: Span) -> Result<String, ShellError> {
        let json_value = json::value_to_json_value(input)?;
        nu_json::to_string_with_indent(&json_value, 4).map_err(|err| ShellError::GenericError {
            error: "Failed to serialize value for clipboard.".into(),
            msg: err.to_string(),
            span: Some(span),
            help: None,
            inner: vec![],
        })
    }

    fn copy_text(input: &Value, span: Span) -> Result<(), ShellError> {
        let text = match input {
            Value::String { val, .. } => val.to_owned(),
            _ => Self::format_json(input, span)?,
        };

        create_clipboard().copy_text(&text)
    }
}

impl Command for ClipCopy {
    fn name(&self) -> &str {
        "clip copy"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::Any, Type::Any)])
            .switch("show", "Display copied value in the output.", Some('s'))
            .category(Category::System)
    }

    fn description(&self) -> &str {
        "Copy the input into the clipboard."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let value = input.into_value(call.head)?;
        Self::copy_text(&value, call.head)?;

        if call.has_flag(engine_state, stack, "show")? {
            Ok(value.into_pipeline_data())
        } else {
            Ok(Value::nothing(call.head).into_pipeline_data())
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "'hello' | clip copy",
                description: "Copy a string to the clipboard.",
                result: None,
            },
            Example {
                example: "$env | clip copy --show",
                description: "Copy a structured value and pass it through.",
                result: None,
            },
        ]
    }
}
