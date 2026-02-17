use super::clipboard::provider::{Clipboard, create_clipboard};
use crate::formats::value_to_json_value;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct ClipCopy;

impl ClipCopy {
    fn format_json(
        engine_state: &EngineState,
        input: &Value,
        span: Span,
    ) -> Result<String, ShellError> {
        let json_value = value_to_json_value(engine_state, input, span, true)?;
        nu_json::to_string_with_indent(&json_value, 4).map_err(|err| ShellError::GenericError {
            error: "Failed to serialize value for clipboard.".into(),
            msg: err.to_string(),
            span: Some(span),
            help: None,
            inner: vec![],
        })
    }

    fn format_raw(input: &Value, config: &nu_protocol::Config) -> String {
        match input {
            Value::String { val, .. } => val.to_owned(),
            _ => input.to_expanded_string("", config),
        }
    }

    fn copy_text(
        engine_state: &EngineState,
        input: &Value,
        span: Span,
        plugin_config: Option<&Value>,
        raw: bool,
        config: &nu_protocol::Config,
    ) -> Result<(), ShellError> {
        let text = if raw {
            Self::format_raw(input, config)
        } else {
            match input {
                Value::String { val, .. } => val.to_owned(),
                _ => Self::format_json(engine_state, input, span)?,
            }
        };

        create_clipboard(plugin_config).copy_text(&text)
    }
}

impl Command for ClipCopy {
    fn name(&self) -> &str {
        "clip copy"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::Any, Type::Any)])
            .switch("raw", "Disable JSON serialization.", Some('r'))
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
        let config = stack.get_config(engine_state);

        #[cfg(target_os = "linux")]
        let plugin_config = {
            config
                .plugins
                .get("clip")
                .or_else(|| config.plugins.get("clipboard"))
                .or_else(|| config.plugins.get("nu_plugin_clipboard"))
                .cloned()
        };
        #[cfg(not(target_os = "linux"))]
        let plugin_config: Option<Value> = None;

        let raw = call.has_flag(engine_state, stack, "raw")?;
        Self::copy_text(
            engine_state,
            &value,
            call.head,
            plugin_config.as_ref(),
            raw,
            &config,
        )?;

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
            Example {
                example: "$env | clip copy --raw",
                description: "Copy a structured value as plain text without JSON serialization.",
                result: None,
            },
        ]
    }
}
