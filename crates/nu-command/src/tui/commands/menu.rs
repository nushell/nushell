use super::WidgetKind;
use super::{empty_tui, flag_strings, push_widget, with_app};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiMenu;

impl Command for TuiMenu {
    fn name(&self) -> &str {
        "tui menu"
    }

    fn description(&self) -> &str {
        "Add a horizontal menu bar. Left/Right (or h/l) move the selection."
    }

    fn signature(&self) -> Signature {
        Signature::build("tui menu")
            .category(Category::Viewers)
            .named(
                "items",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "Menu item labels.",
                None,
            )
            .named("id", SyntaxShape::String, "Widget id.", None)
            .input_output_types(vec![
                (Type::Nothing, empty_tui()),
                (empty_tui(), empty_tui()),
                (Type::Any, empty_tui()),
            ])
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Menu bar above a table",
            example: r#"tui title "Editor" | tui menu --items [File Edit View] | tui table | tui run --headless"#,
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
            let items = flag_strings(engine_state, stack, call, "items")?;
            push_widget(
                engine_state,
                stack,
                call,
                app,
                "menu",
                WidgetKind::Menu { items },
            )
        })
    }
}
