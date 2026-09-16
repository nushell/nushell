use super::{WidgetKind, builder_io_types, children_from_values, push_widget, with_app};
use crate::tui::widget::SplitDir;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiSplit;

impl Command for TuiSplit {
    fn name(&self) -> &str {
        "tui split"
    }

    fn description(&self) -> &str {
        "Lay out child widgets side by side (or stacked with --vertical) with a draggable divider."
    }

    fn extra_description(&self) -> &str {
        "Children are `tui` values built with no pipeline input, in parentheses: `tui split [(tui table) (tui preview)]`. Splits nest: a child can itself be a `tui split`.\n\
         \n\
         `--ratio` is the percent given to the first child (10-90, default 50); any further children share the rest equally. Drag the 1-cell handle with the mouse, or Tab to it and use arrows/hjkl for 1% steps."
    }

    fn signature(&self) -> Signature {
        Signature::build("tui split")
            .category(Category::Viewers)
            .required(
                "children",
                SyntaxShape::List(Box::new(SyntaxShape::Any)),
                "Widgets to arrange, e.g. [(tui table) (tui preview)].",
            )
            .switch("vertical", "Stack children top to bottom.", Some('v'))
            .named(
                "ratio",
                SyntaxShape::Int,
                "Percent of space for the first child (default 50).",
                Some('r'),
            )
            .named("id", SyntaxShape::String, "Widget id.", None)
            .input_output_types(builder_io_types())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Table on the left, file preview on the right",
                example: "ls | tui split --ratio 60 [(tui table --columns [name size]) (tui preview)] | tui run",
                result: None,
            },
            Example {
                description: "Nested: a searchable table beside a log",
                example: "ls | tui split [(tui split --vertical [(tui search --bind /) (tui table)]) (tui log)] | tui run",
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
        let children = children_from_values(call.req(engine_state, stack, 0)?)?;
        let direction = if call.has_flag(engine_state, stack, "vertical")? {
            SplitDir::Vertical
        } else {
            SplitDir::Horizontal
        };
        let ratio = call
            .get_flag::<i64>(engine_state, stack, "ratio")?
            .unwrap_or(50)
            .clamp(10, 90) as u16;
        with_app(call, input, |app| {
            push_widget(
                engine_state,
                stack,
                call,
                app,
                "split",
                WidgetKind::Split { direction, ratio },
                children,
            )
        })
    }
}
