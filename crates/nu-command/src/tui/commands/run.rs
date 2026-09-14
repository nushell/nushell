use super::{empty_tui, session_cwd, size_flag};
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
         - Arrows, hjkl, PageUp/PageDown, Home/End: move in tables and trees; scroll logs\n\
         - Type in a focused search box or text box; q is a character there\n\
         - Mouse: click to focus/select, scroll, drag split handles and dialog chrome\n\
         - Enter: submit the current selection and return a record\n\
         - q / Esc (when not typing) or Ctrl+C: quit\n\
         \n\
         `--refresh 1sec { ls }` re-runs the closure on that interval and replaces the shared data list. A closure without `--refresh` runs once at start.\n\
         `--dialog` opens a floating popup on the alternate screen; `--size [70 20]` sets its size.\n\
         \n\
         To render without a terminal, or to replay keys in a test, use `tui debug`."
    }

    fn signature(&self) -> Signature {
        Signature::build("tui run")
            .category(Category::Viewers)
            .optional(
                "using",
                SyntaxShape::Closure(None),
                "Closure whose output replaces the TUI data list. Used with --refresh.",
            )
            .switch(
                "dialog",
                "Show a floating, draggable, resizable popup on the alternate screen.",
                Some('d'),
            )
            .named(
                "size",
                SyntaxShape::List(Box::new(SyntaxShape::Int)),
                "Popup [width height] with --dialog. Default: 3/4 of the terminal.",
                Some('s'),
            )
            .named(
                "refresh",
                SyntaxShape::Duration,
                "How often to re-run the using closure (e.g. 1sec).",
                Some('r'),
            )
            .switch("no-mouse", "Disable mouse capture.", None)
            .input_output_types(vec![(empty_tui(), Type::Any), (Type::Any, Type::Any)])
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["interactive", "display", "popup"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Pick a file",
                example: r#"ls | tui label --title "files" | tui table | tui run | get selected.name"#,
                result: None,
            },
            Example {
                description: "Refresh a file list every second, in a popup",
                example: "ls | tui table | tui run --dialog --refresh 1sec { ls }",
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
        let size = size_flag(engine_state, stack, call)?;
        let refresh = call
            .get_flag::<i64>(engine_state, stack, "refresh")?
            .map(|nanos| Duration::from_nanos(nanos.unsigned_abs()));
        let using: Option<Closure> = call.opt(engine_state, stack, 0)?;
        let opts = RunOptions {
            mouse: !call.has_flag(engine_state, stack, "no-mouse")?,
            dialog,
            popup_width: size.map(|(w, _)| w.clamp(20, 400) as u16),
            popup_height: size.map(|(_, h)| h.clamp(8, 200) as u16),
            refresh,
            using,
            span: call.head,
            ..RunOptions::default()
        };
        run(
            app,
            data,
            engine_state,
            stack,
            opts,
            session_cwd(engine_state, stack),
        )
    }
}
