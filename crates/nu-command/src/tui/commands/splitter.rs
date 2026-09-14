use super::WidgetKind;
use super::{empty_tui, push_widget, with_app};
use crate::tui::widget::SplitDir;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiSplitter;

impl Command for TuiSplitter {
    fn name(&self) -> &str {
        "tui splitter"
    }

    fn description(&self) -> &str {
        "Split the current page into moveable panes, one per following content widget."
    }

    fn extra_description(&self) -> &str {
        "Prefer --right-of / --left-of / --above / --below on the widgets themselves when you know the ids. `tui splitter` still splits leftover widgets on a page. Drag the handle; ratio is stored in tenths of a percent so the bar tracks the mouse."
    }

    fn signature(&self) -> Signature {
        Signature::build("tui splitter")
            .category(Category::Viewers)
            .named(
                "direction",
                SyntaxShape::String,
                "horizontal (side by side) or vertical (stacked). Default: horizontal.",
                Some('d'),
            )
            .named(
                "ratio",
                SyntaxShape::Int,
                "Percent of space for the first pane (default 50).",
                Some('r'),
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
            description: "Table on the left, preview on the right via placement flags",
            example: "ls | tui table --id files | tui preview --from files --right-of files --ratio 55 | tui run",
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
            let direction = match call
                .get_flag::<String>(engine_state, stack, "direction")?
                .as_deref()
            {
                None => SplitDir::Horizontal,
                Some(s) => SplitDir::parse(s).ok_or_else(|| ShellError::InvalidValue {
                    valid: "horizontal or vertical".into(),
                    actual: s.to_string(),
                    span: call.head,
                })?,
            };
            let ratio = call
                .get_flag::<i64>(engine_state, stack, "ratio")?
                .unwrap_or(50)
                .clamp(10, 90) as u16;
            push_widget(
                engine_state,
                stack,
                call,
                app,
                "splitter",
                WidgetKind::Splitter { direction, ratio },
            )
        })
    }
}
