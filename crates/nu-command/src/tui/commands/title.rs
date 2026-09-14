use super::WidgetKind;
use super::{consume_text_arg, empty_tui, push_widget, with_app};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiTitle;

impl Command for TuiTitle {
    fn name(&self) -> &str {
        "tui title"
    }

    fn description(&self) -> &str {
        "Add a one-line title bar to the TUI."
    }

    fn signature(&self) -> Signature {
        Signature::build("tui title")
            .category(Category::Viewers)
            .optional("text", SyntaxShape::String, "Title text.")
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
            description: "Title a headless TUI",
            example: r#"tui title "My App" | tui run --headless"#,
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
            let text = consume_text_arg(engine_state, stack, call, app, true, "text")?;
            push_widget(
                engine_state,
                stack,
                call,
                app,
                "title",
                WidgetKind::Title { text },
            )
        })
    }
}
