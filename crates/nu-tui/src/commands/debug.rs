use super::{empty_tui, session_cwd, size_flag};
use crate::app::TuiApp;
use crate::keys::key_script_from_value;
use crate::runtime::{RunOptions, debug};
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
        "Use this to test a TUI in a script or CI, or to see why it looks the way it does. The result record has the same fields as `tui run` (action, focused, selected, page, values, rows, live) plus:\n\
         - `screen`: the painted buffer as a string\n\
         - `widgets`: the widget tree with each widget's layout `rect`, `focusable`, and per-kind fields: tables show `resolved_columns` and filtered `rows`; searches show their `query`; previews and `--from` widgets show the `source` they follow; lists show the `search_scope` that filters them\n\
         - `focus`: the tab order and the default focus\n\
         - `pages`: the tab bar entries\n\
         \n\
         `--keys` replays tokens before painting, as a comma-separated string or a list: enter, esc, tab, shift+tab, up, down, left, right, home, end, pageup, pagedown, backspace, delete, space, insert, ctrl+c, alt+a, f1, a single character, `type:hello`, `click:COL,ROW`, `drag:COL,ROW`, `scroll-up`, `scroll-down`. `action` is `render` when the keys finished without Enter or quit. `--until {|state| ...}` stops the replay early once the closure returns true.\n\
         \n\
         A live stream is read for up to 5 seconds (and no more rows than the widgets keep) before painting. A hook closure runs once with the state record, as `tui run` would."
    }

    fn signature(&self) -> Signature {
        Signature::build("tui debug")
            .category(Category::Viewers)
            .optional(
                "hook",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "Hook whose output replaces the data list, run once.",
            )
            .named(
                "keys",
                SyntaxShape::OneOf(vec![
                    SyntaxShape::List(Box::new(SyntaxShape::String)),
                    SyntaxShape::String,
                ]),
                "Key tokens to replay before painting.",
                Some('k'),
            )
            .named(
                "until",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "Stop replaying keys once this returns true for the state record.",
                Some('u'),
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
                example: "[{name: a}, {name: b}] | tui table | tui debug --keys [down enter] | get selected.name",
                result: None,
            },
            Example {
                description: "See where each widget landed",
                example: "ls | tui split [(tui table) (tui preview)] | tui debug | get widgets.0.children | select id rect",
                result: None,
            },
            Example {
                description: "Stop replaying once a condition holds",
                example: "[a b c] | tui table | tui debug --keys [down down down] --until {|s| $s.values.table-0.index == 1 } | get values.table-0.index",
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
        let (app, data) = TuiApp::split_input(input)?;
        let (width, height) = size_flag(engine_state, stack, call)?.unwrap_or((80, 24));
        let using: Option<Closure> = call.opt(engine_state, stack, 0)?;
        let keys = match call.get_flag::<Value>(engine_state, stack, "keys")? {
            Some(value) => key_script_from_value(&value, call.head)?,
            None => Vec::new(),
        };
        let opts = RunOptions {
            keys,
            until: call.get_flag(engine_state, stack, "until")?,
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
