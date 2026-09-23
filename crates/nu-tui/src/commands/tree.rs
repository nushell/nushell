use super::{WidgetKind, builder_io_types, push_widget, selectable_flags, with_app};
use crate::widgets::tree::TreeWidget;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiTree;

impl Command for TuiTree {
    fn name(&self) -> &str {
        "tui tree"
    }

    fn description(&self) -> &str {
        "Add a tree for nested records or a directory walk."
    }

    fn extra_description(&self) -> &str {
        "Up/Down move. Right or l expands, Left or h collapses. Enter submits the selected node; with `--multi`, Space checks nodes. `--walk` treats `type == dir` rows as folders and lists them on expand; `--column` names the path column for that walk. A `tui preview` or a `--from` widget next to this tree follows the selected node."
    }

    fn signature(&self) -> Signature {
        selectable_flags(
            Signature::build("tui tree")
                .category(Category::Viewers)
                .optional(
                    "source",
                    SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                    "Closure over the --from row whose output is this tree's data.",
                )
                .switch(
                    "walk",
                    "Expand directories on disk when a node is opened.",
                    None,
                )
                .named(
                    "column",
                    SyntaxShape::String,
                    "Path/name column (default name).",
                    None,
                ),
        )
        .input_output_types(builder_io_types())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Nested record as a tree",
                example: "{a: {b: 1, c: 2}, d: [3, 4]} | tui tree | tui debug | get screen",
                result: None,
            },
            Example {
                description: "Directory walk next to a preview",
                example: "ls | tui split [(tui tree --walk) (tui preview)] | tui run",
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
            let widget = TreeWidget {
                walk: call.has_flag(engine_state, stack, "walk")?,
                column: call
                    .get_flag(engine_state, stack, "column")?
                    .unwrap_or_else(|| "name".into()),
                multi: call.has_flag(engine_state, stack, "multi")?,
            };
            push_widget(
                engine_state,
                stack,
                call,
                app,
                WidgetKind::Tree(widget),
                Vec::new(),
                source,
            )
        })
    }
}
