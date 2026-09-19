use super::{WidgetKind, builder_io_types, common_flags, push_widget, with_app};
use crate::widgets::button::ButtonWidget;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiButton;

impl Command for TuiButton {
    fn name(&self) -> &str {
        "tui button"
    }

    fn description(&self) -> &str {
        "Add a button that runs a hook, or submits its label, when activated."
    }

    fn extra_description(&self) -> &str {
        "Tab to the button and press Enter or Space, or click it. With a hook closure the button behaves like `tui bind`: the closure receives the state record and may return a new data list or `{action: submit|quit, ...}`. Without one, activating submits the label, so `tui button Yes | tui button No` is a confirm dialog."
    }

    fn signature(&self) -> Signature {
        common_flags(
            Signature::build("tui button")
                .category(Category::Viewers)
                .required("label", SyntaxShape::String, "Button text.")
                .optional(
                    "hook",
                    SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                    "Closure run with the state record when activated.",
                ),
        )
        .input_output_types(builder_io_types())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "A yes/no confirm",
                example: r#"tui label "Delete everything?" | tui button Yes | tui button No | tui debug --keys [tab enter] | get selected"#,
                result: None,
            },
            Example {
                description: "A button that saves the text box and quits",
                example: "tui textbox --id name | tui button Save {|s| $s.values.name | save name.txt; {action: quit} } | tui run",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let label: String = call.req(engine_state, stack, 0)?;
        let action = call.opt(engine_state, stack, 1)?;
        with_app(call, input, |app| {
            push_widget(
                engine_state,
                stack,
                call,
                app,
                WidgetKind::Button(ButtonWidget { label, action }),
                Vec::new(),
                None,
            )
        })
    }
}
