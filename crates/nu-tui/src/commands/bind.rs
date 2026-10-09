use super::{builder_io_types, with_app};
use crate::app::Bind;
use crate::keys::chord_from_value;
use nu_engine::command_prelude::*;
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct TuiBind;

impl Command for TuiBind {
    fn name(&self) -> &str {
        "tui bind"
    }

    fn description(&self) -> &str {
        "Run a hook when a key is pressed anywhere in the TUI."
    }

    fn extra_description(&self) -> &str {
        "The key is a chord string like `ctrl+s`, `alt+r`, `f5`, or `x`, or a reedline-style record `{modifier: control, keycode: char_s}` as in `$env.config.keybindings`.\n\
         \n\
         The hook receives the state record (`{action, focused, selected, page, values, rows, live}`) as `$in` and as its first parameter. Return nothing to change nothing, a value to replace the data list, or `{action: submit, selected: ...}` / `{action: quit}` to end the TUI.\n\
         \n\
         Binds take precedence over the built-in keys, except Ctrl+C. A bind without a modifier does not fire while typing in a search or text box."
    }

    fn signature(&self) -> Signature {
        Signature::build("tui bind")
            .category(Category::Viewers)
            .required(
                "key",
                SyntaxShape::OneOf(vec![
                    SyntaxShape::String,
                    SyntaxShape::Record(vec![].into()),
                ]),
                "Chord like 'ctrl+s', or a {modifier, keycode} record.",
            )
            .required(
                "hook",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "Closure run with the state record.",
            )
            .input_output_types(builder_io_types())
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["keybinding", "shortcut", "hotkey"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Reload the rows with Ctrl+R",
                example: "ls | tui table | tui bind ctrl+r {|| ls } | tui run",
                result: None,
            },
            Example {
                description: "Submit the highlighted row's name with s",
                example: "ls | tui table | tui bind s {|state| {action: submit, selected: $state.selected.name} } | tui run",
                result: None,
            },
            Example {
                description: "Use a reedline-style key record",
                example: "[a b] | tui table | tui bind {modifier: control, keycode: char_q} {|| {action: quit} } | tui debug --keys ctrl+q | get action",
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
        let key: Value = call.req(engine_state, stack, 0)?;
        let chord = chord_from_value(&key)?;
        let closure: Closure = call.req(engine_state, stack, 1)?;
        with_app(call, input, |app| {
            app.binds.retain(|b| b.chord != chord);
            app.binds.push(Bind { chord, closure });
            Ok(())
        })
    }
}
