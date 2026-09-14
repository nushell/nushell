use super::WidgetKind;
use super::{empty_tui, placement_flags, push_widget, with_app};
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
        "Up/Down move. Right or l expands, Left or h collapses. Enter submits the selected node. `--walk` treats `type == dir` rows as folders and lists them on expand. `--from` on `tui preview` can follow this tree."
    }

    fn signature(&self) -> Signature {
        placement_flags(
            Signature::build("tui tree")
                .category(Category::Viewers)
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
                )
                .named("id", SyntaxShape::String, "Widget id.", None)
                .input_output_types(vec![
                    (Type::table(), empty_tui()),
                    (Type::record(), empty_tui()),
                    (Type::list(Type::Any), empty_tui()),
                    (empty_tui(), empty_tui()),
                    (Type::Any, empty_tui()),
                ]),
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Nested record as a tree",
                example: "{a: {b: 1, c: 2}, d: [3, 4]} | tui tree | tui run --headless",
                result: None,
            },
            Example {
                description: "Directory walk next to a preview",
                example: "ls | tui tree --id files --walk | tui preview --from files --right-of files | tui run",
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
            let walk = call.has_flag(engine_state, stack, "walk")?;
            let column = call
                .get_flag(engine_state, stack, "column")?
                .unwrap_or_else(|| "name".into());
            push_widget(
                engine_state,
                stack,
                call,
                app,
                "tree",
                WidgetKind::Tree {
                    data: None,
                    walk,
                    column,
                },
            )
        })
    }
}
