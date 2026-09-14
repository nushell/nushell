use super::WidgetKind;
use super::{consume_text_arg, empty_tui, push_widget, with_app};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiStatus;

impl Command for TuiStatus {
    fn name(&self) -> &str {
        "tui status"
    }

    fn description(&self) -> &str {
        "Add a status bar. Live focus/filter hints are appended."
    }

    fn signature(&self) -> Signature {
        Signature::build("tui status")
            .category(Category::Viewers)
            .optional("text", SyntaxShape::String, "Status text.")
            .named("id", SyntaxShape::String, "Widget id.", None)
            .input_output_types(vec![
                (Type::Nothing, empty_tui()),
                (Type::String, empty_tui()),
                (empty_tui(), empty_tui()),
                (Type::Any, empty_tui()),
            ])
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Status bar under a title",
            example: r#"tui title "App" | tui status "ready" | tui run --headless"#,
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
            let text = consume_text_arg(engine_state, stack, call, app, false, "text")?;
            push_widget(
                engine_state,
                stack,
                call,
                app,
                "status",
                WidgetKind::Status { text },
            )
        })
    }
}
