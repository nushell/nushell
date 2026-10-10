use super::{WidgetKind, builder_io_types, data_flags, push_widget, with_app};
use crate::widgets::progress::ProgressWidget;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiProgress;

impl Command for TuiProgress {
    fn name(&self) -> &str {
        "tui progress"
    }

    fn description(&self) -> &str {
        "Add a progress bar."
    }

    fn extra_description(&self) -> &str {
        "The bar shows `--value`, or otherwise reads its data: a number, a `{value, total}` record's `value`, or the last number in a list, so a streamed counter drives it live. Values above 1 are read out of `--total` (default 100).\n\
         \n\
         With `--from` and a closure the bar follows another widget's highlighted row: `tui progress --from table-0 {|row| $row.pct }`."
    }

    fn signature(&self) -> Signature {
        data_flags(
            Signature::build("tui progress")
                .category(Category::Viewers)
                .optional(
                    "source",
                    SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                    "Closure over the --from row that yields the value.",
                )
                .named(
                    "value",
                    SyntaxShape::Number,
                    "Fixed value to show.",
                    Some('v'),
                )
                .named(
                    "total",
                    SyntaxShape::Number,
                    "Value that fills the bar (default 1, or 100 for values above 1).",
                    Some('t'),
                )
                .named(
                    "label",
                    SyntaxShape::String,
                    "Text before the percentage.",
                    Some('l'),
                ),
        )
        .input_output_types(builder_io_types())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "A static bar at 40%",
                example: "tui progress --value 0.4 --label copying | tui debug | get screen",
                result: None,
            },
            Example {
                description: "A bar driven by a stream",
                example: "1..100 | each {|n| sleep 50ms; $n } | tui progress --total 100 | tui run",
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
        let source = call.opt(engine_state, stack, 0)?;
        let value = call.get_flag::<f64>(engine_state, stack, "value")?;
        let total = call.get_flag::<f64>(engine_state, stack, "total")?;
        let label = call.get_flag(engine_state, stack, "label")?;
        with_app(call, input, |app| {
            push_widget(
                engine_state,
                stack,
                call,
                app,
                WidgetKind::Progress(ProgressWidget {
                    value,
                    total,
                    label,
                }),
                Vec::new(),
                source,
            )
        })
    }
}
