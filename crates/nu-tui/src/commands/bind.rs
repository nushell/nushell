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
        "The key is a chord string like `ctrl+s`, `alt+r`, `f5`, or `x`, or a reedline-style record `{modifier: control, keycode: char_s}` as in `$env.config.keybindings`. Chords separated by spaces make a sequence: `m s` fires on `m` then `s`. After `m`, a panel lists the keys that can follow with their `--label`s; a key that continues no sequence cancels it.\n\
         \n\
         The hook receives the state record (`{action, focused, selected, page, values, rows, live}`) as `$in` and as its first parameter. Return nothing to change nothing, a value to replace the data list, or `{action: submit, selected: ...}` / `{action: quit}` to end the TUI.\n\
         \n\
         A hook can also change one widget: `{action: focus, id: view}`; `{action: resize, id: layout, sizes: [0 1fr]}` gives a split new sizes; `{action: context, value: {dir: /tmp}}` merges into the app's `tui context` record (`$state.context`), and with `id: view` into that widget's part of it, running the closures that see the change again; `{action: scroll, id: view, line: 40}` scrolls a preview's text (a negative line counts from the end); `{action: columns, id: files, columns: [name size]}` changes which columns a table shows; `{action: select, id: files, index: 3}` highlights a row. A list of such records applies each in turn, and `{action: data, value: [...]}` in such a list replaces the data as a returned list would, so new rows and a highlight can go together.\n\
         \n\
         `{action: suspend, hook: {|state| ... }}` lends the terminal to the hook: raw mode and mouse capture go off, the normal screen comes back, and a full-screen program (an editor, fzf) can run; what the hook returns is then applied as above, so a list replaces the data and nothing leaves things as they were. Add `area: <widget id>` to keep the TUI on screen round the hook's output instead: the cursor starts at that widget's top-left corner and `$state.area` holds its `{x, y, width, height}`, for a program that can draw in a region (fzf with `--height` and `--margin`).\n\
         \n\
         `--when <id>` scopes the bind to the time that widget is focused. If the hook then returns nothing, the key goes on to the widget as if the bind were not there, so a bind can claim a key for some rows and leave it for others.\n\
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
                "Chord like 'ctrl+s', a sequence like 'm s', or a {modifier, keycode} record.",
            )
            .required(
                "hook",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "Closure run with the state record.",
            )
            .named(
                "when",
                SyntaxShape::String,
                "Only while this widget id is focused; a hook returning nothing passes the key on.",
                None,
            )
            .named(
                "label",
                SyntaxShape::String,
                "What the key panel says this bind does, while a sequence awaits its next key.",
                None,
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
                description: "Page a PDF preview with PgDn only while the preview is focused",
                example: "ls | tui split [(tui table) (tui preview --id view --on-change {|| {page: 1} } {|row, ctx| pdftoppm -png -singlefile -f $ctx.page -l $ctx.page $row.name | complete | get stdout })] | tui bind --when view pagedown {|s| {action: context, id: view, value: {page: ($s.values.view.context.page + 1)}} } | tui run",
                result: None,
            },
            Example {
                description: "Choose the columns with a two-key sequence; `m` alone shows the choices",
                example: "ls | tui table --id files --columns [name size] | tui bind \"m s\" --label \"size\" {|| {action: columns, id: files, columns: [name size]} } | tui bind \"m t\" --label \"modified\" {|| {action: columns, id: files, columns: [name modified]} } | tui run",
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
        let when: Option<String> = call.get_flag(engine_state, stack, "when")?;
        let label: Option<String> = call.get_flag(engine_state, stack, "label")?;
        with_app(call, input, |app| {
            app.binds.retain(|b| !(b.chord == chord && b.when == when));
            app.binds.push(Bind {
                chord,
                closure,
                label,
                when,
            });
            Ok(())
        })
    }
}
