use super::clipboard::provider::{Clipboard, create_clipboard};
use crate::viewers::render_value_as_plain_table_text;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct ClipCopy;

impl ClipCopy {
    fn copy_text(
        engine_state: &EngineState,
        stack: &mut Stack,
        input: &Value,
        span: Span,
        config: &nu_protocol::Config,
    ) -> Result<(), ShellError> {
        let text = match input {
            Value::String { val, .. } => val.to_owned(),
            _ => render_value_as_plain_table_text(engine_state, stack, input.clone(), span)?,
        };

        create_clipboard(config, engine_state, stack).copy_text(&text)
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
        let config = stack.get_config(engine_state);

        Self::copy_text(engine_state, stack, &value, call.head, &config)?;

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
                example: "ls | clip copy",
                description: "Copy structured values as plain table text without ANSI escape sequences.",
                result: None,
            },
        ]
    }
}
