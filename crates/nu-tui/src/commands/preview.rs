use super::{WidgetKind, builder_io_types, common_flags, push_widget, with_app};
use crate::widgets::preview::PreviewWidget;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiPreview;

impl Command for TuiPreview {
    fn name(&self) -> &str {
        "tui preview"
    }

    fn description(&self) -> &str {
        "Show text for the selected table, tree, or select row. Arrow keys stay on the source."
    }

    fn extra_description(&self) -> &str {
        "Without a closure, the row's `name` (or the row itself when it is a string) is read as a file path, up to `--max-bytes`. Directories and binary files show a short note.\n\
         \n\
         A closure with no parameters transforms that file text: it receives the contents as `$in`, with `content_type` in pipeline metadata, so `{ nu-highlight }` colors source files.\n\
         \n\
         A closure with one parameter is the source: it receives the selected row and whatever it returns is shown. Nothing is read from disk, so any column or computed value can be previewed: `{|row| $row.event | to nuon }` or `{|row| open --raw $row.path }`.\n\
         \n\
         The preview follows the focused list, else the nearest one in the same container. `--from` names one explicitly."
    }

    fn signature(&self) -> Signature {
        common_flags(
            Signature::build("tui preview")
                .category(Category::Viewers)
                .optional(
                    "transform",
                    SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                    "No parameters: transform file text (`$in`). One parameter: produce text from the row.",
                )
                .named(
                    "from",
                    SyntaxShape::String,
                    "Table, tree, or select id to follow.",
                    None,
                )
                .named(
                    "max-bytes",
                    SyntaxShape::Int,
                    "Maximum bytes to read from a file (default 65536).",
                    None,
                ),
        )
        .input_output_types(builder_io_types())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "File list on the left, highlighted contents on the right",
                example: "ls | tui split [(tui table) (tui preview { nu-highlight })] | tui run",
                result: None,
            },
            Example {
                description: "Preview a value from the row instead of a file",
                example: "$env.config.keybindings | tui split [(tui table) (tui preview {|row| $row.event | to nuon })] | tui run",
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
            let transform = call.opt(engine_state, stack, 0)?;
            let max_bytes = call
                .get_flag::<i64>(engine_state, stack, "max-bytes")?
                .unwrap_or(65536)
                .clamp(1, 8 * 1024 * 1024) as usize;
            push_widget(
                engine_state,
                stack,
                call,
                app,
                WidgetKind::Preview(PreviewWidget {
                    max_bytes,
                    transform,
                }),
                Vec::new(),
                None,
            )
        })
    }
}
