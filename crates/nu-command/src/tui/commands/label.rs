use super::{WidgetKind, builder_io_types, consume_text_arg, push_widget, with_app};
use crate::tui::widget::Slot;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiLabel;

impl Command for TuiLabel {
    fn name(&self) -> &str {
        "tui label"
    }

    fn description(&self) -> &str {
        "Add static text: inline, or as the title bar (--title) or status bar (--status)."
    }

    fn extra_description(&self) -> &str {
        "Without a flag the text is drawn where it appears in the layout. `--title` puts it on the one-line bar at the top (also used as the dialog title). `--status` puts it on the bottom bar, where live focus/filter/row hints are appended."
    }

    fn signature(&self) -> Signature {
        Signature::build("tui label")
            .category(Category::Viewers)
            .optional("text", SyntaxShape::String, "Label text.")
            .switch("title", "Show as the title bar at the top.", None)
            .switch("status", "Show as the status bar at the bottom.", None)
            .named("id", SyntaxShape::String, "Widget id.", None)
            .input_output_types(builder_io_types())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Title and status around a table",
                example: r#"ls | tui label --title "files" | tui table | tui label --status "enter: pick  q: quit" | tui run"#,
                result: None,
            },
            Example {
                description: "Inline text above a text box",
                example: r#"tui label "new name" | tui textbox | tui debug | get screen"#,
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
        let title = call.has_flag(engine_state, stack, "title")?;
        let status = call.has_flag(engine_state, stack, "status")?;
        let slot = match (title, status) {
            (true, true) => {
                return Err(ShellError::IncompatibleParametersSingle {
                    msg: "use only one of --title, --status".into(),
                    span: call.head,
                });
            }
            (true, false) => Slot::Title,
            (false, true) => Slot::Status,
            (false, false) => Slot::Content,
        };
        with_app(call, input, |app| {
            let text = consume_text_arg(engine_state, stack, call, app, false, "text")?;
            push_widget(
                engine_state,
                stack,
                call,
                app,
                "label",
                WidgetKind::Label { text, slot },
                Vec::new(),
            )
        })
    }
}
