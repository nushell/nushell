use super::WidgetKind;
use super::{empty_tui, placement_flags, push_widget, with_app};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiTextBox;

impl Command for TuiTextBox {
    fn name(&self) -> &str {
        "tui textbox"
    }

    fn description(&self) -> &str {
        "Add an editable text field. Tab to focus it, then type."
    }

    fn extra_description(&self) -> &str {
        "Editable by default. Use --readonly for display-only. While focused, q is a character, Esc leaves the field, Enter submits the TUI with the text as `selected`."
    }

    fn signature(&self) -> Signature {
        placement_flags(
            Signature::build("tui textbox")
                .category(Category::Viewers)
                .named(
                    "placeholder",
                    SyntaxShape::String,
                    "Shown when empty.",
                    None,
                )
                .named("value", SyntaxShape::String, "Initial text.", None)
                .switch("readonly", "Do not accept typing.", None)
                .named("id", SyntaxShape::String, "Widget id.", None)
                .input_output_types(vec![
                    (Type::Nothing, empty_tui()),
                    (empty_tui(), empty_tui()),
                    (Type::Any, empty_tui()),
                ]),
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Type into a text box and submit (headless)",
            example: r#"tui textbox --placeholder "name" | tui run --keys "type:Ada,enter""#,
            result: None,
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        with_app(call, input, |app| {
            let placeholder = call
                .get_flag(engine_state, stack, "placeholder")?
                .unwrap_or_default();
            let value = call
                .get_flag(engine_state, stack, "value")?
                .unwrap_or_default();
            let editable = !call.has_flag(engine_state, stack, "readonly")?;
            push_widget(
                engine_state,
                stack,
                call,
                app,
                "textbox",
                WidgetKind::TextBox {
                    placeholder,
                    editable,
                    value,
                },
            )
        })
    }
}
