use super::{
    WidgetKind, border_flag, builder_io_types, common_flags, image_flags, image_options,
    push_widget, title_flag, with_app,
};
use crate::widgets::preview::PreviewWidget;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiImage;

impl Command for TuiImage {
    fn name(&self) -> &str {
        "tui image"
    }

    fn description(&self) -> &str {
        "Add an image: a file by path, or its bytes."
    }

    fn extra_description(&self) -> &str {
        "The same widget as `tui preview`, showing one fixed image instead of the highlighted row's file, so it needs no list to follow and several can sit side by side: `tui split [(tui image a.png) (tui image b.png)]`. It takes the same `--image-fit`, `--image-width`, and `--image-height` flags, and `$env.config.tui.image_protocol` applies.\n\
         \n\
         The file is read and the image decoded on a thread, so a large one does not hold up the rest of the TUI; the pane says `(loading...)` until it is ready."
    }

    fn signature(&self) -> Signature {
        common_flags(
            image_flags(border_flag(title_flag(Signature::build("tui image"))))
                .category(Category::Viewers)
                .required(
                    "source",
                    SyntaxShape::OneOf(vec![SyntaxShape::Filepath, SyntaxShape::Binary]),
                    "Path of an image file, or its bytes.",
                ),
        )
        .input_output_types(builder_io_types())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Show an image file",
                example: "tui image logo.png | tui run",
                result: None,
            },
            Example {
                description: "Two images side by side, each at most 40 cells wide",
                example: "tui split [(tui image a.png --image-width 40) (tui image b.png --image-width 40)] | tui run",
                result: None,
            },
            Example {
                description: "Read an image's size and format without a terminal",
                example: "tui image logo.png | tui debug | get values.image-0.image",
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
        let source: Value = call.req(engine_state, stack, 0)?;
        let fixed = match source {
            Value::String { .. } | Value::Glob { .. } | Value::Binary { .. } => source,
            other => {
                return Err(ShellError::TypeMismatch {
                    err_message: format!(
                        "expected a file path or binary, found {}",
                        other.get_type()
                    ),
                    span: other.span(),
                });
            }
        };
        let image = image_options(engine_state, stack, call)?;
        with_app(call, input, |app| {
            push_widget(
                engine_state,
                stack,
                call,
                app,
                WidgetKind::Preview(PreviewWidget {
                    max_bytes: 65536,
                    transform: None,
                    fixed: Some(fixed),
                    image,
                    // Read and decode off the UI thread, so a large file
                    // does not hold up the rest of the TUI.
                    threads: true,
                    on_change: None,
                }),
                Vec::new(),
                None,
                None,
            )
        })
    }
}
