use super::{
    WidgetKind, border_flag, builder_io_types, common_flags, image_flags, image_options,
    push_widget, title_flag, with_app,
};
use crate::widgets::preview::PreviewWidget;
use nu_engine::command_prelude::*;
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct TuiPreview;

impl Command for TuiPreview {
    fn name(&self) -> &str {
        "tui preview"
    }

    fn description(&self) -> &str {
        "Show text or an image for the selected table, tree, or select row. Tab to it to scroll the text or zoom and pan the image."
    }

    fn extra_description(&self) -> &str {
        "Without a closure, the row's `name` (or the row itself when it is a string) is read as a file path, up to `--max-bytes`. Directories and binary files show a short note.\n\
         \n\
         An image file (PNG, JPEG, GIF, WebP, BMP, ICO, TIFF) is drawn with the terminal's graphics protocol: Kitty, Sixel, or iTerm2, else as colored half blocks. `--image-fit` sizes it in the pane and `--image-width` / `--image-height` cap that size; every mode keeps the aspect ratio. Images decode on a thread, so the list keeps moving while a large one loads.\n\
         \n\
         With `--threads`, the file read or the closure runs on a thread and the pane says `(loading...)` until it is done, so holding an arrow key over slow sources keeps the list moving. Moving on before it finishes interrupts the closure and kills any external it started. Images always decode on a thread.\n\
         \n\
         The pane takes focus with Tab or a click. Text then scrolls with the arrows, `j`/`k`, PageUp/PageDown, Home/End. An image zooms with `+`/`-`, pans with the arrows or `hjkl`, and `0` shows it as first drawn; the zoom and pan are in the widget's `image` value and reset when the previewed file changes. `$env.config.tui.image_protocol` forces a protocol, or `none` to describe images as text. Under Kitty each widget transmits its images under one id, freed when the TUI exits; `--image-id` picks the id and leaves the image in the terminal for `tui image delete`. `tui image` shows a fixed image the same way.\n\
         \n\
         A closure with no parameters transforms that file text: it receives the contents as `$in`, with `content_type` in pipeline metadata, so `{ nu-highlight }` colors source files. Images skip it.\n\
         \n\
         A closure with one parameter is the source: it receives the selected row and whatever it returns is shown. Nothing is read from disk, so any column or computed value can be previewed: `{|row| $row.event | to nuon }` or `{|row| open --raw $row.path }`. Bytes it returns are drawn as an image when they are one.\n\
         \n\
         A second parameter receives the context: the app's `tui context` record with this pane's own part laid over, `views.<id>` of it. When the file on show changes, that part starts over as what the `--on-change` hook returns (`{|s| {page: 1} }`, or nothing without one), until a hook returns `{action: context, id: <this id>, value: {...}}`, which merges into it; a change to any key the closure sees runs it again. That is how a key turns a PDF page: `{|row, ctx| ... -f $ctx.page ...}`. The context and the text scroll are in the widget's value as `context` and `scroll`; a `hint` string in the context replaces the status bar hint while the pane is focused, and `{action: scroll, id, line}` moves the text.\n\
         \n\
         The preview follows the focused list, else the nearest one in the same container. `--from` names one explicitly.\n\
         \n\
         The border shows the previewed file's name. With `--title`, the title comes first and the file name follows in parentheses: `--title src` shows `src (Cargo.toml)`."
    }

    fn signature(&self) -> Signature {
        common_flags(
            image_flags(border_flag(title_flag(Signature::build("tui preview"))))
                .category(Category::Viewers)
                .optional(
                    "transform",
                    SyntaxShape::Closure(Some(vec![SyntaxShape::Any, SyntaxShape::Any])),
                    "No parameters: transform file text (`$in`). One parameter: produce text from the row. Two: the row and the context.",
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
                )
                .named(
                    "on-change",
                    SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                    "A hook run when the file on show changes, with the state record (`values.<id>.row` is the new row); what it returns is this pane's part of the app context (`views.<id>`) until `{action: context, id}` hooks change it.",
                    None,
                )
                .switch(
                    "threads",
                    "Read the file or run the closure on a thread, so a slow source (a PDF rasteriser) does not hold up the list.",
                    Some('t'),
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
            Example {
                description: "Images at most half the pane wide, never scaled up",
                example: "ls | tui split [(tui table) (tui preview --image-width \"50%\")] | tui run",
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
        let image = image_options(engine_state, stack, call)?;
        with_app(call, input, |app| {
            let transform = call.opt(engine_state, stack, 0)?;
            let max_bytes = call
                .get_flag::<i64>(engine_state, stack, "max-bytes")?
                .unwrap_or(65536)
                .clamp(1, 8 * 1024 * 1024) as usize;
            let threads = call.has_flag(engine_state, stack, "threads")?;
            let on_change: Option<Closure> = call.get_flag(engine_state, stack, "on-change")?;
            push_widget(
                engine_state,
                stack,
                call,
                app,
                WidgetKind::Preview(PreviewWidget {
                    max_bytes,
                    transform,
                    fixed: None,
                    image,
                    threads,
                    on_change,
                }),
                Vec::new(),
                None,
                None,
            )
        })
    }
}
