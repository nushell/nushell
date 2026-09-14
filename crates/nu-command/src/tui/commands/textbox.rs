use super::{WidgetKind, builder_io_types, push_widget, with_app};
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
        "While focused, q is a character, Esc leaves the field, Enter submits the TUI with the text as `selected`. The text is also returned under `values` keyed by widget id."
    }

    fn signature(&self) -> Signature {
        Signature::build("tui textbox")
            .category(Category::Viewers)
            .named(
                "placeholder",
                SyntaxShape::String,
                "Shown when empty.",
                None,
            )
            .named("value", SyntaxShape::String, "Initial text.", None)
            .named("id", SyntaxShape::String, "Widget id.", None)
            .input_output_types(builder_io_types())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Type into a text box and submit (headless)",
            example: r#"tui textbox --placeholder "name" | tui debug --keys "type:Ada,enter" | get selected"#,
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
            push_widget(
                engine_state,
                stack,
                call,
                app,
                "textbox",
                WidgetKind::TextBox { placeholder, value },
                Vec::new(),
            )
        })
    }
}
