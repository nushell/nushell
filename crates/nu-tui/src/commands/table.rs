use super::{WidgetKind, builder_io_types, flag_strings, push_widget, selectable_flags, with_app};
use crate::widgets::table::TableWidget;
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
        "Up/Down, j/k, PageUp/PageDown, Home/End, and mouse wheel move the selection. Enter submits the current row (`--index` submits its index). With `--multi`, Space checks rows and the selection is the checked rows. A `tui search` widget filters rows as you type. Streams append while the TUI runs (oldest rows drop after 10,000).\n\
         \n\
         `--data` gives this table its own rows; `--from <id>` with a closure makes it a detail view of another widget's highlighted row: `tui table --from tree-0 {|node| ls $node.name }`. `--on-select` runs a hook whenever the highlight moves.\n\
         \n\
         `--capture-keys` turns a chord pressed on the focused table (for example Ctrl+R) into the filter query, so a keybinding table filters to bindings that match that chord. Rows with `modifier`/`keycode` fields match reedline-style chords."
    }

    fn signature(&self) -> Signature {
        selectable_flags(
            Signature::build("tui table")
                .category(Category::Viewers)
                .optional(
                    "source",
                    SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                    "Closure over the --from row whose output is this table's rows.",
                )
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
                .switch("index", "Select row indexes instead of rows.", Some('i')),
        )
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
                description: "Check several rows and submit them",
                example: "[a b c] | tui table --multi | tui debug --keys [space down space enter] | get selected",
                result: None,
            },
            Example {
                description: "A detail table driven by a tree",
                example: "ls | tui split [(tui tree --walk) (tui table --from tree-0 {|node| ls $node.name })] | tui run",
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
        let source = call.opt(engine_state, stack, 0)?;
        with_app(call, input, |app| {
            let widget = TableWidget {
                columns: flag_strings(engine_state, stack, call, "columns")?,
                capture_keys: call.has_flag(engine_state, stack, "capture-keys")?,
                multi: call.has_flag(engine_state, stack, "multi")?,
                index: call.has_flag(engine_state, stack, "index")?,
            };
            push_widget(
                engine_state,
                stack,
                call,
                app,
                WidgetKind::Table(widget),
                Vec::new(),
                source,
            )
        })
    }
}
