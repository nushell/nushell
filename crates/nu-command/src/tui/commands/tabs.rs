use super::WidgetKind;
use super::{empty_tui, push_widget, with_app};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiTabs;

impl Command for TuiTabs {
    fn name(&self) -> &str {
        "tui tabs"
    }

    fn description(&self) -> &str {
        "Show an explicit tab bar. Multiple `tui tab` / `tui body` pages already draw a bar; this forces one even for a single page."
    }

    fn signature(&self) -> Signature {
        Signature::build("tui tabs")
            .category(Category::Viewers)
            .named("id", SyntaxShape::String, "Widget id.", None)
            .input_output_types(vec![
                (Type::Nothing, empty_tui()),
                (empty_tui(), empty_tui()),
                (Type::Any, empty_tui()),
            ])
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Force a tab bar",
            example: r#"tui tabs | tui tab "one" | tui label "first" | tui run --headless"#,
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
            push_widget(engine_state, stack, call, app, "tabs", WidgetKind::Tabs)
        })
    }
}
