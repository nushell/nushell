use super::{empty_tui, session_cwd, size_flag};
use crate::tui::app::TuiApp;
use crate::tui::runtime::{RunOptions, debug};
use nu_engine::command_prelude::*;
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct TuiDebug;

impl Command for TuiDebug {
    fn name(&self) -> &str {
        "tui debug"
    }

    fn description(&self) -> &str {
        "Render a TUI without a terminal and return the screen plus the resolved layout, focus order, and pages."
    }

    fn extra_description(&self) -> &str {
        "Use this to test a TUI in a script or CI, or to see why it looks the way it does. The result record has the same fields as `tui run` (action, focused, selected, search, page, values, rows, live) plus:\n\
         - `screen`: the painted buffer as a string\n\
         - `widgets`: the widget tree with each widget's layout `rect`, `focusable`, and for tables the `resolved_columns` and filtered `rows`; searches show their `query`; previews show the `source` they follow; scrollable widgets show the `search_scope` that filters them\n\
         - `focus`: the tab order and the default focus\n\
         - `pages`: the tab bar entries\n\
         \n\
         `--keys` replays comma-separated tokens before painting: enter, esc, tab, shift+tab, up, down, left, right, home, end, pageup, pagedown, backspace, delete, space, insert, ctrl+c, alt+a, f1, a single character, `type:hello`, `click:COL,ROW`, `drag:COL,ROW`, `scroll-up`, `scroll-down`. `action` is `render` when the keys finished without Enter or quit.\n\
         \n\
         A finite stream is drained (5 second cap) before painting. A closure runs once and replaces the data list, as `tui run --refresh` would."
    }

    fn signature(&self) -> Signature {
        Signature::build("tui debug")
            .category(Category::Viewers)
            .optional(
                "using",
                SyntaxShape::Closure(None),
                "Closure whose output replaces the TUI data list, run once.",
            )
            .named(
                "keys",
                SyntaxShape::String,
                "Comma-separated key tokens to replay before painting.",
                Some('k'),
            )
            .named(
                "size",
                SyntaxShape::List(Box::new(SyntaxShape::Int)),
                "Canvas [width height]. Default [80 24].",
                Some('s'),
            )
            .switch("dialog", "Paint as a floating popup.", Some('d'))
            .input_output_types(vec![
                (empty_tui(), Type::record()),
                (Type::Any, Type::record()),
            ])
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["headless", "test", "render", "inspect"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Paint a title and status bar",
                example: r#"tui label --title "Demo" | tui label --status "ready" | tui debug | get screen"#,
                result: None,
            },
            Example {
                description: "Replay keys and read the selection",
                example: "[{name: a}, {name: b}] | tui table | tui debug --keys down,enter | get selected.name",
                result: None,
            },
            Example {
                description: "See where each widget landed",
                example: "ls | tui split [(tui table) (tui preview)] | tui debug | get widgets.0.children | select id rect",
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
        let (width, height) = size_flag(engine_state, stack, call)?.unwrap_or((80, 24));
        let using: Option<Closure> = call.opt(engine_state, stack, 0)?;
        let opts = RunOptions {
            keys: call.get_flag(engine_state, stack, "keys")?,
            width: width.clamp(16, 400) as u16,
            height: height.clamp(4, 200) as u16,
            dialog: call.has_flag(engine_state, stack, "dialog")?,
            using,
            span: call.head,
            ..RunOptions::default()
        };
        debug(
            app,
            data,
            engine_state,
            stack,
            opts,
            session_cwd(engine_state, stack),
        )
    }
}
