use super::WidgetKind;
use super::{consume_text_arg, empty_tui, push_widget, with_app};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiTab;

impl Command for TuiTab {
    fn name(&self) -> &str {
        "tui tab"
    }

    fn description(&self) -> &str {
        "Start a named tab. Later content widgets belong to this tab until the next `tui tab` or `tui body`."
    }

    fn extra_description(&self) -> &str {
        "Click a tab, press 1-9, or use `[` `]` / Ctrl+Tab to switch. Same page model as `tui body`, with a title meant for the tab bar."
    }

    fn signature(&self) -> Signature {
        Signature::build("tui tab")
            .category(Category::Viewers)
            .optional("title", SyntaxShape::String, "Tab title.")
            .named("id", SyntaxShape::String, "Widget id.", None)
            .input_output_types(vec![
                (Type::Nothing, empty_tui()),
                (empty_tui(), empty_tui()),
                (Type::Any, empty_tui()),
            ])
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Files tab and a keys tab",
            example: r#"ls | tui tab "files" | tui table | tui tab "keys" | tui keybindings --bindings $env.config.keybindings | tui run"#,
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
            let title = consume_text_arg(engine_state, stack, call, app, false, "title")?;
            let title = if title.is_empty() {
                "tab".into()
            } else {
                title
            };
            push_widget(
                engine_state,
                stack,
                call,
                app,
                "tab",
                WidgetKind::Tab { title },
            )
        })
    }
}
