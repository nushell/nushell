use super::empty_tui;
use crate::tui::app::TuiApp;
use crate::tui::runtime::{RunOptions, run};
use nu_engine::command_prelude::*;
use nu_protocol::engine::Closure;
use std::time::Duration;

#[derive(Clone)]
pub struct TuiRun;

impl Command for TuiRun {
    fn name(&self) -> &str {
        "tui run"
    }

    fn description(&self) -> &str {
        "Run a composed TUI until the user quits or submits a selection."
    }

    fn extra_description(&self) -> &str {
        "Interactive keys:\n\
         - Tab / Shift+Tab: move focus between widgets\n\
         - [ / ] or Ctrl+Tab: switch tabs\n\
         - 1-9: jump to a tab\n\
         - Arrows, hjkl, PageUp/PageDown, Home/End: move in tables, lists, and trees\n\
         - Type in a focused search box or text box; q is a character there\n\
         - Mouse: click to focus/select, scroll, drag splitter handles and dialog chrome\n\
         - Enter: submit the current selection and return a record\n\
         - q / Esc (when not typing) or Ctrl+C: quit\n\
         \n\
         `--refresh 1sec { ls }` re-runs the closure on that interval and replaces the shared data list.\n\
         `--dialog` opens a floating popup on the alternate screen."
    }

    fn signature(&self) -> Signature {
        Signature::build("tui run")
            .category(Category::Viewers)
            .switch(
                "headless",
                "Render without a TTY. Without --keys, returns the screen as a string.",
                Some('H'),
            )
            .named(
                "keys",
                SyntaxShape::String,
                "Replay events then return a result record. Implies headless.",
                Some('k'),
            )
            .named(
                "width",
                SyntaxShape::Int,
                "Headless canvas width, or popup width with --dialog.",
                Some('w'),
            )
            .named(
                "height",
                SyntaxShape::Int,
                "Headless canvas height, or popup height with --dialog.",
                None,
            )
            .switch(
                "dialog",
                "Show a floating, draggable, resizable popup on the alternate screen.",
                Some('d'),
            )
            .named(
                "refresh",
                SyntaxShape::Duration,
                "How often to re-run the using closure (e.g. 1sec).",
                Some('r'),
            )
            .optional(
                "using",
                SyntaxShape::Closure(None),
                "Closure whose output replaces the TUI data list. Used with --refresh.",
            )
            .switch(
                "no-mouse",
                "Disable mouse capture in interactive mode.",
                None,
            )
            .input_output_types(vec![(empty_tui(), Type::Any), (Type::Any, Type::Any)])
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["interactive", "display", "popup"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Render a title and status bar in headless mode",
                example: r#"tui title "Demo" | tui status "ready" | tui run --headless"#,
                result: None,
            },
            Example {
                description: "Refresh a file list every second",
                example: r#"ls | tui table | tui run --refresh 1sec { ls }"#,
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
        let (app, data) = TuiApp::split_input(input, call.head)?;
        let dialog = call.has_flag(engine_state, stack, "dialog")?;
        let width_flag = call.get_flag::<i64>(engine_state, stack, "width")?;
        let height_flag = call.get_flag::<i64>(engine_state, stack, "height")?;
        let width = width_flag.unwrap_or(80).clamp(16, 400) as u16;
        let height = height_flag.unwrap_or(24).clamp(4, 200) as u16;
        let refresh = call
            .get_flag::<i64>(engine_state, stack, "refresh")?
            .map(|nanos| Duration::from_nanos(nanos.unsigned_abs()));
        let using: Option<Closure> = call.opt(engine_state, stack, 0)?;
        let opts = RunOptions {
            headless: call.has_flag(engine_state, stack, "headless")?,
            keys: call.get_flag(engine_state, stack, "keys")?,
            width,
            height,
            mouse: !call.has_flag(engine_state, stack, "no-mouse")?,
            dialog,
            popup_width: if dialog {
                width_flag.map(|w| w.clamp(20, 400) as u16)
            } else {
                None
            },
            popup_height: if dialog {
                height_flag.map(|h| h.clamp(8, 200) as u16)
            } else {
                None
            },
            refresh,
            using,
            span: call.head,
        };
        let cwd = engine_state
            .cwd_as_string(Some(stack))
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("."));
        run(app, data, engine_state, stack, opts, cwd)
    }
}
