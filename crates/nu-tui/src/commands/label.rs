use super::{WidgetKind, builder_io_types, data_flags, push_widget, with_app};
use crate::widgets::label::{LabelWidget, Slot};
use nu_engine::command_prelude::*;
use nu_protocol::engine::Closure;
use nu_protocol::{DeprecationEntry, DeprecationType, ReportMode};

#[derive(Clone)]
pub struct TuiLabel;

impl Command for TuiLabel {
    fn name(&self) -> &str {
        "tui label"
    }

    fn description(&self) -> &str {
        "Add static text: inline, or as the title bar (--titlebar) or status bar (--status)."
    }

    fn extra_description(&self) -> &str {
        "Without a flag the text is drawn where it appears in the layout. `--titlebar` puts it on the one-line bar at the top (also used as the dialog title). `--status` puts it on the bottom bar, where live focus/filter/row hints are appended.\n\
         \n\
         A closure instead of text makes the label follow a list: it receives the highlighted row of `--from` (or the nearest table/tree/select) and its output is shown."
    }

    fn signature(&self) -> Signature {
        data_flags(
            Signature::build("tui label")
                .category(Category::Viewers)
                .optional(
                    "text",
                    SyntaxShape::OneOf(vec![
                        SyntaxShape::String,
                        SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                    ]),
                    "Label text, or a closure over the source row.",
                )
                .switch("titlebar", "Show as the title bar at the top.", None)
                .switch(
                    "title",
                    "Show as the title bar at the top (deprecated, use --titlebar).",
                    None,
                )
                .switch("status", "Show as the status bar at the bottom.", None),
        )
        .input_output_types(builder_io_types())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Title and status around a table",
                example: r#"ls | tui label --titlebar "files" | tui table | tui label --status "enter: pick  q: quit" | tui run"#,
                result: None,
            },
            Example {
                description: "Inline text above a text box",
                example: r#"tui label "new name" | tui textbox | tui debug | get screen"#,
                result: None,
            },
            Example {
                description: "A label that follows the highlighted row",
                example: r#"ls | tui table | tui label {|row| $"size: ($row.size)" } | tui run"#,
                result: None,
            },
        ]
    }

    fn deprecation_info(&self) -> Vec<DeprecationEntry> {
        vec![DeprecationEntry {
            ty: DeprecationType::Flag("title".into()),
            report_mode: ReportMode::FirstUse,
            since: Some("0.117.0".into()),
            expected_removal: Some("0.118.0".into()),
            help: Some(
                "This switch has been renamed to `--titlebar`. Bordered widgets use `--title <text>` to name their border."
                    .into(),
            ),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // `--title` is the 0.116.0 name of `--titlebar`, kept for one
        // release (see `deprecation_info`).
        let titlebar = call.has_flag(engine_state, stack, "titlebar")?
            || call.has_flag(engine_state, stack, "title")?;
        let status = call.has_flag(engine_state, stack, "status")?;
        let slot = match (titlebar, status) {
            (true, true) => {
                return Err(ShellError::IncompatibleParametersSingle {
                    msg: "use only one of --titlebar, --status".into(),
                    span: call.head,
                });
            }
            (true, false) => Slot::Title,
            (false, true) => Slot::Status,
            (false, false) => Slot::Content,
        };
        let arg: Option<Value> = call.opt(engine_state, stack, 0)?;
        let (text, closure): (Option<String>, Option<Closure>) = match arg {
            Some(Value::Closure { val, .. }) => (None, Some(*val)),
            Some(Value::String { val, .. }) => (Some(val), None),
            Some(other) => {
                return Err(ShellError::TypeMismatch {
                    err_message: format!("expected text or a closure, found {}", other.get_type()),
                    span: other.span(),
                });
            }
            None => (None, None),
        };
        with_app(call, input, |app| {
            let text = match text {
                Some(text) => text,
                None if closure.is_some() => String::new(),
                // A scalar string piped in is the label text.
                None => super::consume_text_arg(engine_state, stack, call, app, false, "text")?,
            };
            push_widget(
                engine_state,
                stack,
                call,
                app,
                WidgetKind::Label(LabelWidget { text, slot }),
                Vec::new(),
                closure,
            )
        })
    }
}
