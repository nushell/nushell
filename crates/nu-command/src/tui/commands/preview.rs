use super::WidgetKind;
use super::{empty_tui, placement_flags, push_widget, with_app};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiPreview;

impl Command for TuiPreview {
    fn name(&self) -> &str {
        "tui preview"
    }

    fn description(&self) -> &str {
        "Preview the file at the selected table, list, or tree row. Arrow keys stay on the source."
    }

    fn extra_description(&self) -> &str {
        "An optional closure receives the file contents as `$in`, with pipeline metadata `content_type`. If the closure takes a parameter, that parameter is the selected row. `--from` picks the source widget."
    }

    fn signature(&self) -> Signature {
        placement_flags(
            Signature::build("tui preview")
                .category(Category::Viewers)
                .optional(
                    "transform",
                    SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                    "Closure run on file contents (`$in`). Optional argument is the selected row.",
                )
                .named(
                    "column",
                    SyntaxShape::String,
                    "Column that holds the path. Default: name.",
                    Some('c'),
                )
                .named(
                    "max-bytes",
                    SyntaxShape::Int,
                    "Maximum bytes to read from a file (default 65536).",
                    None,
                )
                .named(
                    "from",
                    SyntaxShape::String,
                    "Table, list, or tree id to follow.",
                    None,
                )
                .named("id", SyntaxShape::String, "Widget id.", None)
                .input_output_types(vec![
                    (Type::Nothing, empty_tui()),
                    (empty_tui(), empty_tui()),
                    (Type::Any, empty_tui()),
                ]),
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "File list on the left, contents on the right",
            example: r#"ls | tui table --id files | tui preview --from files --right-of files | tui run"#,
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
            let transform = call.opt(engine_state, stack, 0)?;
            let column = call
                .get_flag(engine_state, stack, "column")?
                .unwrap_or_else(|| "name".into());
            let max_bytes = call
                .get_flag::<i64>(engine_state, stack, "max-bytes")?
                .unwrap_or(65536)
                .clamp(1, 8 * 1024 * 1024) as usize;
            let from = call.get_flag(engine_state, stack, "from")?;
            push_widget(
                engine_state,
                stack,
                call,
                app,
                "preview",
                WidgetKind::Preview {
                    column,
                    max_bytes,
                    transform,
                    from,
                },
            )
        })
    }
}
