use super::{
    WidgetKind, border_flag, builder_io_types, common_flags, image_flags, image_options,
    push_widget, title_flag, with_app,
};
use crate::image::{kitty_delete, kitty_delete_all};
use crate::widgets::preview::PreviewWidget;
use nu_engine::command_prelude::*;
use std::io::Write;

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
        "The same widget as `tui preview`, showing one fixed image instead of the highlighted row's file, so several can sit side by side: `tui split [(tui image a.png) (tui image b.png)]`. It takes the same `--image-fit`, `--image-width`, `--image-height`, and `--image-id` flags, and `$env.config.tui.image_protocol` applies.\n\
         \n\
         Under Kitty the image is transmitted under an id that is freed when the TUI exits. With `--image-id` the id is yours: the image stays in the terminal's memory until `tui image delete` frees it."
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
                description: "Keep the image in the terminal under Kitty id 7, and free it later",
                example: "tui image logo.png --image-id 7 | tui run; tui image delete 7",
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
                    threads: false,
                    on_change: None,
                }),
                Vec::new(),
                None,
                None,
            )
        })
    }
}

#[derive(Clone)]
pub struct TuiImageDelete;

impl Command for TuiImageDelete {
    fn name(&self) -> &str {
        "tui image delete"
    }

    fn description(&self) -> &str {
        "Free Kitty images from the terminal's memory by id."
    }

    fn extra_description(&self) -> &str {
        "A Kitty terminal keeps every transmitted image until it is deleted or evicted. `tui preview` and `tui image` free the ids they allocate when the TUI exits, but an image shown with `--image-id` is left for its owner. This command sends the delete for each id given, placements and data both, and nothing else: an id the terminal does not know is ignored.\n\
         \n\
         It works from a hook while the TUI runs, and from the shell after it returns. `--all` deletes every image in the terminal, including ones other programs put there.\n\
         \n\
         Inside tmux the sequence is wrapped for passthrough, which needs `allow-passthrough on` in tmux."
    }

    fn signature(&self) -> Signature {
        Signature::build("tui image delete")
            .category(Category::Viewers)
            .rest("ids", SyntaxShape::Int, "Kitty image ids to free.")
            .switch(
                "all",
                "Delete every image in the terminal, whoever transmitted it.",
                Some('a'),
            )
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Free the image shown under id 7",
                example: "tui image delete 7",
                result: None,
            },
            Example {
                description: "Free an image from a key binding while the TUI runs",
                example: "tui image logo.png --image-id 7 | tui bind ctrl+d {|| tui image delete 7 } | tui run",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let ids: Vec<Spanned<i64>> = call.rest(engine_state, stack, 0)?;
        let all = call.has_flag(engine_state, stack, "all")?;
        if ids.is_empty() && !all {
            return Err(ShellError::MissingParameter {
                param_name: "ids (or --all)".into(),
                span: call.head,
            });
        }
        let ids = ids
            .into_iter()
            .map(|id| {
                u32::try_from(id.item)
                    .ok()
                    .filter(|id| *id > 0)
                    .ok_or_else(|| ShellError::InvalidValue {
                        valid: format!("a Kitty image id from 1 to {}", u32::MAX),
                        actual: id.item.to_string(),
                        span: id.span,
                    })
            })
            .collect::<Result<Vec<u32>, _>>()?;
        let tmux = stack.get_env_var(engine_state, "TMUX").is_some();
        let mut sequence = kitty_delete(&ids, tmux);
        if all {
            sequence.push_str(&kitty_delete_all(tmux));
        }
        let mut out = crate::runtime::Tty::open();
        out.write_all(sequence.as_bytes())
            .and_then(|_| out.flush())
            .map_err(|err| ShellError::Io(IoError::new(err, call.head, None)))?;
        Ok(PipelineData::empty())
    }
}
