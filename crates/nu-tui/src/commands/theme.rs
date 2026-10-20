use super::{builder_io_types, with_app};
use crate::theme::{COLOR_SLOTS, GLYPH_SLOTS, Glyphs, is_slot};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiTheme;

impl Command for TuiTheme {
    fn name(&self) -> &str {
        "tui theme"
    }

    fn description(&self) -> &str {
        "Give this TUI its own colors, over those in `$env.config.tui`."
    }

    fn extra_description(&self) -> &str {
        "The record's keys are the slots of `$env.config.tui`. Colors: `title_bar`, `status_bar`, `border`, `border_focused`, `selected`, `header`, `muted`, `highlight`, `tab_active`, `tab_inactive`, `progress`, `button`, `surface`, `backdrop`, and the file kinds path cells are colored by (`file_dir`, `file_orphan`, `file_exec`, `file_image`, `file_media`, `file_archive`, `file_document`); each takes a color name, `#rrggbb`, or `{fg, bg, attr}`. `border_type` takes a `table --theme` name.\n\
         \n\
         Glyphs: `title_caps`, `status_caps`, `tab_caps`, and `selected_caps` take `{open, close}`, a one-cell glyph each (or empty), drawn in the text's fill color at the ends of the title bar's text, each status segment, each tab's title, and the highlighted row of a table, select, or tree: powerline's half circles, `{open: \"\\u{e0b6}\", close: \"\\u{e0b4}\"}` in a Nerd Font, make pills as yazi's. With caps the title and status bars no longer fill their line. `check` (`--multi` marks) and `radio` (`tui select`) take `{on, off}` of one width; `tree_open` and `tree_closed` an arrow each; `progress_fill` the gauge's one-cell fill.\n\
         \n\
         Slots left out keep the config's value, so a script can expose its look as `$env.<app>.theme` and lay that over the user's config."
    }

    fn signature(&self) -> Signature {
        Signature::build("tui theme")
            .category(Category::Viewers)
            .required(
                "slots",
                SyntaxShape::Record(vec![].into()),
                "Slot names to colors, as in `$env.config.tui`.",
            )
            .input_output_types(builder_io_types())
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["color", "style", "palette"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "A solid highlight bar and red directories for one TUI",
                example: r##"ls | tui table | tui theme {selected: {bg: "#3b4252"}, file_dir: red} | tui run"##,
                result: None,
            },
            Example {
                description: "Powerline pills on the bars and tabs, and round the highlighted row (needs a Nerd Font)",
                example: r#"ls | tui label --titlebar files | tui tab a [(tui table)] | tui theme {title_caps: {open: "\u{e0b6}", close: "\u{e0b4}"}, tab_caps: {open: "\u{e0b6}", close: "\u{e0b4}"}, selected_caps: {open: "\u{e0b6}", close: "\u{e0b4}"}} | tui run"#,
                result: None,
            },
            Example {
                description: "Let users restyle a script through an env record",
                example: "ls | tui table | tui theme ($env.browser?.theme? | default {}) | tui run",
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
        let slots: Record = call.req(engine_state, stack, 0)?;
        for (name, value) in &slots {
            if !is_slot(name) {
                return Err(ShellError::InvalidValue {
                    valid: format!(
                        "one of border_type, {}, {}",
                        COLOR_SLOTS.join(", "),
                        GLYPH_SLOTS.join(", ")
                    ),
                    actual: name.clone(),
                    span: value.span(),
                });
            }
            if name == "border_type" {
                crate::theme::parse_border(value)?;
            }
            Glyphs::default().apply(name, value)?;
        }
        with_app(call, input, |app| {
            app.theme.extend(slots);
            Ok(())
        })
    }
}
