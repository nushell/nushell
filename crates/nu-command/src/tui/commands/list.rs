use super::WidgetKind;
use super::{empty_tui, placement_flags, push_widget, with_app};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiList;

impl Command for TuiList {
    fn name(&self) -> &str {
        "tui list"
    }

    fn description(&self) -> &str {
        "Add a selectable list. Pipeline values become items; streams append while the TUI runs (oldest rows drop after 10,000)."
    }

    fn signature(&self) -> Signature {
        placement_flags(
            Signature::build("tui list")
                .category(Category::Viewers)
                .named("id", SyntaxShape::String, "Widget id.", None)
                .input_output_types(vec![
                    (Type::list(Type::Any), empty_tui()),
                    (empty_tui(), empty_tui()),
                    (Type::Any, empty_tui()),
                ]),
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Stream numbers into a live list",
            example: "1..5 | each {|n| sleep 10ms; $n} | tui list | tui run --headless",
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
            push_widget(
                engine_state,
                stack,
                call,
                app,
                "list",
                WidgetKind::List { data: None },
            )
        })
    }
}
