use super::{WidgetKind, builder_io_types, flag_strings, push_widget, with_app};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiTable;

impl Command for TuiTable {
    fn name(&self) -> &str {
        "tui table"
    }

    fn description(&self) -> &str {
        "Add a navigable table. Pipeline rows become the rows; scalars show as one `item` column."
    }

    fn extra_description(&self) -> &str {
        "Up/Down, j/k, PageUp/PageDown, Home/End, and mouse wheel move the selection. Enter submits the current row. A `tui search` widget filters rows as you type. Streams append while the TUI runs (oldest rows drop after 10,000).\n\
         \n\
         `--capture-keys` turns a chord pressed on the focused table (for example Ctrl+R) into the filter query, so a keybinding table filters to bindings that match that chord. Rows with `modifier`/`keycode` fields match reedline-style chords."
    }

    fn signature(&self) -> Signature {
        Signature::build("tui table")
            .category(Category::Viewers)
            .named(
                "columns",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "Columns to show. Defaults to the columns of the input table.",
                None,
            )
            .switch(
                "capture-keys",
                "A chord pressed on the focused table becomes the filter query.",
                None,
            )
            .named("id", SyntaxShape::String, "Widget id.", None)
            .input_output_types(builder_io_types())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Show a small table and pick a row",
                example: "[{name: foo, size: 1}, {name: bar, size: 2}] | tui table --columns [name size] | tui debug --keys down,enter | get selected.name",
                result: None,
            },
            Example {
                description: "Keybinding explorer: press a chord to filter",
                example: "$env.config.keybindings | tui table --capture-keys --columns [name modifier keycode] | tui run",
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
        with_app(call, input, |app| {
            let columns = flag_strings(engine_state, stack, call, "columns")?;
            let capture_keys = call.has_flag(engine_state, stack, "capture-keys")?;
            push_widget(
                engine_state,
                stack,
                call,
                app,
                "table",
                WidgetKind::Table {
                    columns,
                    capture_keys,
                },
                Vec::new(),
            )
        })
    }
}
