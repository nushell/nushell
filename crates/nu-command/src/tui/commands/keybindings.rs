use super::WidgetKind;
use super::{empty_tui, placement_flags, push_widget, with_app};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiKeybindings;

impl Command for TuiKeybindings {
    fn name(&self) -> &str {
        "tui keybindings"
    }

    fn description(&self) -> &str {
        "Show keybindings in a navigable, filterable table."
    }

    fn extra_description(&self) -> &str {
        "Pipe `$env.config.keybindings` or `keybindings list`. When this widget is focused, pressing a chord filters the list to bindings that mention that chord."
    }

    fn signature(&self) -> Signature {
        placement_flags(
            Signature::build("tui keybindings")
                .category(Category::Viewers)
                .named(
                    "bindings",
                    SyntaxShape::Any,
                    "Keybinding table. Defaults to pipeline data stored on the TUI.",
                    None,
                )
                .named("id", SyntaxShape::String, "Widget id.", None)
                .input_output_types(vec![
                    (Type::table(), empty_tui()),
                    (Type::list(Type::Any), empty_tui()),
                    (empty_tui(), empty_tui()),
                    (Type::Any, empty_tui()),
                ]),
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Headless keybindings popup",
            example: r#"[{name: "history", modifier: "control", keycode: "char_r", mode: "emacs", event: {send: "OpenHistory"}}] | tui keybindings | tui run --headless"#,
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
            let data = call.get_flag::<Value>(engine_state, stack, "bindings")?;
            push_widget(
                engine_state,
                stack,
                call,
                app,
                "keybindings",
                WidgetKind::Keybindings { data },
            )
        })
    }
}
