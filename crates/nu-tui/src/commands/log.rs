use super::{WidgetKind, builder_io_types, data_flags, push_widget, with_app};
use crate::widgets::log::LogWidget;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiLog;

impl Command for TuiLog {
    fn name(&self) -> &str {
        "tui log"
    }

    fn description(&self) -> &str {
        "Add an append-only log view. Streamed rows appear at the bottom."
    }

    fn extra_description(&self) -> &str {
        "Follows the tail as new items arrive. Mouse wheel and Up/Down scroll; scrolling up pauses follow until you hit the bottom again. `--max-lines` keeps that many newest lines in the view. Streamed rows share a global store capped at max(10000, --max-lines).\n\
         \n\
         With `--from` and a closure the log shows the closure's output for the highlighted row of another widget, e.g. `tui log --from table-0 {|row| open $row.name | lines }`."
    }

    fn signature(&self) -> Signature {
        data_flags(
            Signature::build("tui log")
                .category(Category::Viewers)
                .optional(
                    "source",
                    SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                    "Closure over the source row whose output is shown.",
                )
                .named(
                    "max-lines",
                    SyntaxShape::Int,
                    "Keep only the last N lines (default 10000).",
                    None,
                ),
        )
        .input_output_types(builder_io_types())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Live log of a slow stream",
            example: r#"1.. | each {|n| sleep 100ms; $"tick ($n)"} | tui label --title "ticks" | tui log | tui run"#,
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
        let source = call.opt(engine_state, stack, 0)?;
        with_app(call, input, |app| {
            let max_lines = call
                .get_flag::<i64>(engine_state, stack, "max-lines")?
                .unwrap_or(10_000)
                .clamp(10, 1_000_000) as usize;
            push_widget(
                engine_state,
                stack,
                call,
                app,
                WidgetKind::Log(LogWidget { max_lines }),
                Vec::new(),
                source,
            )
        })
    }
}
