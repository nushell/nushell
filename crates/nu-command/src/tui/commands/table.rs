use super::WidgetKind;
use super::{empty_tui, flag_strings, placement_flags, push_widget, with_app};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiTable;

impl Command for TuiTable {
    fn name(&self) -> &str {
        "tui table"
    }

    fn description(&self) -> &str {
        "Add a navigable table. Pipeline records become the rows."
    }

    fn extra_description(&self) -> &str {
        "Up/Down, j/k, PageUp/PageDown, Home/End, and mouse wheel move the selection. Enter submits the current row. A `tui search` widget filters rows as you type. Streams append while the TUI runs (oldest rows drop after 10,000). Use --right-of / --left-of / --above / --below to place this table next to another widget."
    }

    fn signature(&self) -> Signature {
        placement_flags(
            Signature::build("tui table")
                .category(Category::Viewers)
                .named(
                    "columns",
                    SyntaxShape::List(Box::new(SyntaxShape::String)),
                    "Columns to show. Defaults to the columns of the input table.",
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
            description: "Show a small table and pick a row",
            example: r#"[{name: foo, size: 1}, {name: bar, size: 2}] | tui table --columns [name size] | tui run --keys down,enter"#,
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
            let columns = flag_strings(engine_state, stack, call, "columns")?;
            push_widget(
                engine_state,
                stack,
                call,
                app,
                "table",
                WidgetKind::Table {
                    columns,
                    data: None,
                },
            )
        })
    }
}
