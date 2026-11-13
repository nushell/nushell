use crate::platform::RawModeGuard;
use crossterm::event::{
    DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
    EnableMouseCapture, KeyCode, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind,
};
use crossterm::execute;
use nu_engine::command_prelude::*;
use std::time::Duration;

use nu_utils::time::Instant;

use nu_protocol::shell_error::{generic::GenericError, io::IoError};
use num_traits::AsPrimitive;
use std::io::stdout;

#[derive(Clone)]
pub struct InputListen;

impl Command for InputListen {
    fn name(&self) -> &str {
        "input listen"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["prompt", "interactive", "keycode"]
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Platform)
            .named(
                "types",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "Listen for event of specified types only (can be one of: focus, key, mouse, paste, resize).",
                Some('t'),
            )
            .switch(
                "raw",
                "Add raw_code field with numeric value of keycode and raw_modifiers with bit mask flags.",
                Some('r'),
            )
            .named(
                "timeout",
                SyntaxShape::Duration,
                "How long to wait for input before returning.",
                Some('o')
            )
            // Each event type has its own fields (see `extra_description`), and `type` is the only
            // one they share. A key-only record type fails `let x = input listen` on other events.
            .input_output_types(vec![(
                Type::Nothing,
                Type::Record(vec![("type".to_string(), Type::String)].into()),
            )])
    }

    fn description(&self) -> &str {
        "Listen for user interface events."
    }

    fn extra_description(&self) -> &str {
        "There are 5 different type of events: focus, key, mouse, paste, resize. Each will produce a
corresponding record, distinguished by type field:
```
    { type: focus event: (gained|lost) }
    { type: key key_type: <key_type> code: <string> modifiers: [ <modifier> ... ] }
    { type: mouse col: <int> row: <int> kind: <string> modifiers: [ <modifier> ... ] }
    { type: paste content: <string> }
    { type: resize col: <int> row: <int> }
```
With --raw, key and mouse records also get `raw_modifiers: <int>` (the modifier bit mask),
and key records for char keys get `raw_code: <int>` (the character's code point).
There are 6 `modifier` variants: shift, control, alt, super, hyper, meta.
There are 5 `key_type` variants:
    f - f1, f2, f3 ... keys
    char - alphanumeric and special symbols (a, A, 1, $ ...)
    media - dedicated media keys (play, pause, tracknext ...)
    modifier - a modifier key on its own (leftshift, rightalt ...). Terminals send these only in
               the kitty keyboard protocol's report-all-keys mode, which `input listen` doesn't turn on
    other - keys not falling under previous categories (up, down, backspace, enter ...)
Some terminals send a focus event whenever focus reporting is turned on, so `input listen`
returns `{type: focus event: gained}` right away. To wait for other events instead, leave
focus out of --types, for example `input listen --types [key mouse paste resize]`."
    }
    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Listen for a keyboard shortcut and find out how nu receives it.",
            example: "input listen --types [key]",
            result: None,
        }]
    }
    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let event_type_filter = get_event_type_filter(engine_state, stack, call, head)?;
        let timeout: Option<Duration> = call.get_flag(engine_state, stack, "timeout")?;
        let add_raw = call.has_flag(engine_state, stack, "raw")?;
        let config = stack.get_config(engine_state);

        let _raw_mode = RawModeGuard::acquire(stack, head)?;
        // Turns off what is turned on below when dropped, on every return path. Declared after
        // `_raw_mode` so it drops first, while the terminal is still in raw mode.
        let mut console_state = DeferredConsoleRestore::default();

        if config.use_kitty_protocol {
            if let Ok(false) = crossterm::terminal::supports_keyboard_enhancement() {
                println!("WARN: The terminal doesn't support use_kitty_protocol config.\r");
            }

            // enable kitty protocol
            //
            // Note that, currently, only the following support this protocol:
            // * [kitty terminal](https://sw.kovidgoyal.net/kitty/)
            // * [foot terminal](https://codeberg.org/dnkl/foot/issues/319)
            // * [WezTerm terminal](https://wezfurlong.org/wezterm/config/lua/config/enable_kitty_keyboard.html)
            // * [notcurses library](https://github.com/dankamongmen/notcurses/issues/2131)
            // * [neovim text editor](https://github.com/neovim/neovim/pull/18181)
            // * [kakoune text editor](https://github.com/mawww/kakoune/issues/4103)
            // * [dte text editor](https://gitlab.com/craigbarnes/dte/-/issues/138)
            // * [ghostty terminal](https://github.com/ghostty-org/ghostty/pull/317)
            //
            // Refer to https://sw.kovidgoyal.net/kitty/keyboard-protocol/ if you're curious.
            let _ = execute!(
                stdout(),
                crossterm::event::PushKeyboardEnhancementFlags(
                    crossterm::event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                )
            );
            console_state.pop_keyboard_enhancement = true;
        }

        event_type_filter.enable_events(&mut console_state, head)?;
        let start = Instant::now();
        let mut remaining_time = timeout;

        loop {
            if let Some(t) = remaining_time
                && !crossterm::event::poll(t).map_err(|_| {
                    ShellError::Generic(GenericError::new("Error with user input", "", head))
                })?
            {
                return Err(ShellError::Generic(GenericError::new(
                    "Timed out while waiting for user input",
                    "no input was received within the timeout duration",
                    head,
                )));
            }
            let event = crossterm::event::read().map_err(|_| {
                ShellError::Generic(GenericError::new("Error with user input", "", head))
            })?;
            let event = parse_event(head, &event, &event_type_filter, add_raw);
            if let Some(event) = event {
                return Ok(event.into_pipeline_data());
            }

            remaining_time = timeout.map(|t| t.saturating_sub(start.elapsed()));
        }
    }
}

fn get_event_type_filter(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    head: Span,
) -> Result<EventTypeFilter, ShellError> {
    let event_type_filter = call.get_flag::<Value>(engine_state, stack, "types")?;
    let event_type_filter = event_type_filter
        .map(|list| EventTypeFilter::from_value(list, head))
        .transpose()?
        .unwrap_or_else(EventTypeFilter::all);
    Ok(event_type_filter)
}

/// Which event types to listen for. The default listens for none.
#[derive(Default)]
struct EventTypeFilter {
    listen_focus: bool,
    listen_key: bool,
    listen_mouse: bool,
    listen_paste: bool,
    listen_resize: bool,
}

impl EventTypeFilter {
    fn all() -> EventTypeFilter {
        EventTypeFilter {
            listen_focus: true,
            listen_key: true,
            listen_mouse: true,
            listen_paste: true,
            listen_resize: true,
        }
    }

    fn from_value(value: Value, head: Span) -> Result<EventTypeFilter, ShellError> {
        let list_span = value.span();
        let Value::List { vals, .. } = value else {
            return Err(Self::types_error(
                head,
                "--types expects a list of strings".into(),
                list_span,
            ));
        };
        // An empty list listens for nothing, so `input listen` could only end by timing out.
        if vals.is_empty() {
            return Err(Self::types_error(
                head,
                "--types expects at least one event type".into(),
                list_span,
            ));
        }

        let mut filter = Self::default();
        for event_type in vals {
            let span = event_type.span();
            let Value::String { val, .. } = event_type else {
                return Err(Self::types_error(
                    head,
                    "--types expects a list of strings".into(),
                    span,
                ));
            };
            match val.as_str() {
                "focus" => filter.listen_focus = true,
                "key" => filter.listen_key = true,
                "mouse" => filter.listen_mouse = true,
                "paste" => filter.listen_paste = true,
                "resize" => filter.listen_resize = true,
                _ => {
                    return Err(Self::types_error(
                        head,
                        format!("{val} is not a valid event type"),
                        span,
                    ));
                }
            }
        }
        Ok(filter)
    }

    /// An invalid `--types` argument, pointing at the offending value.
    fn types_error(head: Span, msg: String, input_span: Span) -> ShellError {
        ShellError::UnsupportedInput {
            msg,
            input: "value originates from here".into(),
            msg_span: head,
            input_span,
        }
    }

    /// Enable capturing of all events allowed by this filter. Each mode is recorded in
    /// `console_state` before its enable sequence is written, so a failure part way through,
    /// even one after the bytes reached the terminal, still turns it off. Turning off a mode
    /// that never came on is harmless.
    fn enable_events(
        &self,
        console_state: &mut DeferredConsoleRestore,
        span: Span,
    ) -> Result<(), ShellError> {
        if self.listen_mouse {
            console_state.setup_event_types.listen_mouse = true;
            crossterm::execute!(stdout(), EnableMouseCapture)
                .map_err(|err| IoError::new(err, span, None))?;
        }

        if self.listen_paste {
            console_state.setup_event_types.listen_paste = true;
            crossterm::execute!(stdout(), EnableBracketedPaste)
                .map_err(|err| IoError::new(err, span, None))?;
        }

        if self.listen_focus {
            console_state.setup_event_types.listen_focus = true;
            crossterm::execute!(stdout(), crossterm::event::EnableFocusChange)
                .map_err(|err| IoError::new(err, span, None))?;
        }

        Ok(())
    }
}

/// Guard that restores the console when dropped: it pops the kitty keyboard flags pushed in
/// [`InputListen::run`] and disables the event capturing turned on by
/// [`EventTypeFilter::enable_events`]. Dropping it covers every exit path, timeouts and
/// errors included.
#[derive(Default)]
struct DeferredConsoleRestore {
    /// The event types whose capturing may be on and has to be turned off.
    setup_event_types: EventTypeFilter,
    /// Set once the kitty keyboard enhancement flags were pushed.
    pop_keyboard_enhancement: bool,
}

impl Drop for DeferredConsoleRestore {
    fn drop(&mut self) {
        if self.pop_keyboard_enhancement {
            let _ = execute!(stdout(), crossterm::event::PopKeyboardEnhancementFlags);
        }

        if self.setup_event_types.listen_mouse {
            let _ = crossterm::execute!(stdout(), DisableMouseCapture);
        }

        if self.setup_event_types.listen_paste {
            let _ = crossterm::execute!(stdout(), DisableBracketedPaste);
        }

        if self.setup_event_types.listen_focus {
            let _ = crossterm::execute!(stdout(), DisableFocusChange);
        }
    }
}

fn parse_event(
    head: Span,
    event: &crossterm::event::Event,
    filter: &EventTypeFilter,
    add_raw: bool,
) -> Option<Value> {
    match event {
        crossterm::event::Event::FocusGained => {
            create_focus_event(head, filter, FocusEventType::Gained)
        }
        crossterm::event::Event::FocusLost => {
            create_focus_event(head, filter, FocusEventType::Lost)
        }
        crossterm::event::Event::Key(event) => create_key_event(head, filter, event, add_raw),
        crossterm::event::Event::Mouse(event) => create_mouse_event(head, filter, event, add_raw),
        crossterm::event::Event::Paste(content) => create_paste_event(head, filter, content),
        crossterm::event::Event::Resize(cols, rows) => {
            create_resize_event(head, filter, *cols, *rows)
        }
    }
}

enum FocusEventType {
    Gained,
    Lost,
}

impl FocusEventType {
    fn string(self) -> String {
        match self {
            FocusEventType::Gained => "gained".to_string(),
            FocusEventType::Lost => "lost".to_string(),
        }
    }
}

fn create_focus_event(
    head: Span,
    filter: &EventTypeFilter,
    event_type: FocusEventType,
) -> Option<Value> {
    if filter.listen_focus {
        Some(Value::record(
            record! {
                "type" => Value::string("focus", head),
                "event" => Value::string(event_type.string(), head)
            },
            head,
        ))
    } else {
        None
    }
}

fn create_key_event(
    head: Span,
    filter: &EventTypeFilter,
    event: &crossterm::event::KeyEvent,
    add_raw: bool,
) -> Option<Value> {
    if filter.listen_key {
        let crossterm::event::KeyEvent {
            code: raw_code,
            modifiers: raw_modifiers,
            kind,
            ..
        } = event;

        // Ignore release events on windows.
        // Refer to crossterm::event::PushKeyboardEnhancementFlags. According to the doc
        // KeyEventKind and KeyEventState work correctly only on windows and with kitty
        // keyboard protocol. Because of this `keybindings get` currently ignores anything
        // but KeyEventKind::Press
        if let KeyEventKind::Release | KeyEventKind::Repeat = kind {
            return None;
        }

        let (key, code) = get_keycode_name(head, raw_code);

        let mut record = record! {
            "type" => Value::string("key", head),
            "key_type" => key,
            "code" => code,
            "modifiers" => parse_modifiers(head, raw_modifiers),
        };

        if add_raw {
            if let KeyCode::Char(c) = raw_code {
                record.push("raw_code", Value::int(c.as_(), head));
            }
            record.push(
                "raw_modifiers",
                Value::int(raw_modifiers.bits() as i64, head),
            );
        }

        Some(Value::record(record, head))
    } else {
        None
    }
}

fn get_keycode_name(head: Span, code: &KeyCode) -> (Value, Value) {
    let (typ, code) = match code {
        KeyCode::F(n) => ("f", n.to_string()),
        KeyCode::Char(c) => ("char", c.to_string()),
        KeyCode::Media(m) => ("media", format!("{m:?}").to_ascii_lowercase()),
        KeyCode::Modifier(m) => ("modifier", format!("{m:?}").to_ascii_lowercase()),
        _ => ("other", format!("{code:?}").to_ascii_lowercase()),
    };
    (Value::string(typ, head), Value::string(code, head))
}

fn parse_modifiers(head: Span, modifiers: &KeyModifiers) -> Value {
    const ALL_MODIFIERS: [KeyModifiers; 6] = [
        KeyModifiers::SHIFT,
        KeyModifiers::CONTROL,
        KeyModifiers::ALT,
        KeyModifiers::SUPER,
        KeyModifiers::HYPER,
        KeyModifiers::META,
    ];

    let parsed_modifiers = ALL_MODIFIERS
        .iter()
        .filter(|m| modifiers.contains(**m))
        .map(|m| format!("{m:?}").to_ascii_lowercase())
        .map(|string| Value::string(string, head))
        .collect();

    Value::list(parsed_modifiers, head)
}

fn create_mouse_event(
    head: Span,
    filter: &EventTypeFilter,
    event: &MouseEvent,
    add_raw: bool,
) -> Option<Value> {
    if filter.listen_mouse {
        let kind = match event.kind {
            MouseEventKind::Down(btn) => format!("{btn:?}_down"),
            MouseEventKind::Up(btn) => format!("{btn:?}_up"),
            MouseEventKind::Drag(btn) => format!("{btn:?}_drag"),
            MouseEventKind::Moved => "moved".to_string(),
            MouseEventKind::ScrollDown => "scroll_down".to_string(),
            MouseEventKind::ScrollUp => "scroll_up".to_string(),
            MouseEventKind::ScrollLeft => "scroll_left".to_string(),
            MouseEventKind::ScrollRight => "scroll_right".to_string(),
        };

        let mut record = record! {
            "type" => Value::string("mouse", head),
            "col" => Value::int(event.column as i64, head),
            "row" => Value::int(event.row as i64, head),
            "kind" => Value::string(kind, head),
            "modifiers" => parse_modifiers(head, &event.modifiers),
        };

        if add_raw {
            record.push(
                "raw_modifiers",
                Value::int(event.modifiers.bits() as i64, head),
            );
        }

        Some(Value::record(record, head))
    } else {
        None
    }
}

fn create_paste_event(head: Span, filter: &EventTypeFilter, content: &str) -> Option<Value> {
    if filter.listen_paste {
        let record = record! {
            "type" => Value::string("paste", head),
            "content" => Value::string(content, head),
        };

        Some(Value::record(record, head))
    } else {
        None
    }
}

fn create_resize_event(
    head: Span,
    filter: &EventTypeFilter,
    columns: u16,
    rows: u16,
) -> Option<Value> {
    if filter.listen_resize {
        let record = record! {
            "type" => Value::string("resize", head),
            "col" => Value::int(columns as i64, head),
            "row" => Value::int(rows as i64, head),
        };

        Some(Value::record(record, head))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{Event, KeyEvent};
    use nu_protocol::CompareTypes;
    use rstest::rstest;

    /// `let x = input listen` checks the event against the declared output type at runtime,
    /// so every event kind must fit it, with and without `--raw`. The type requires a `type`
    /// string field, so this also checks that every record has one.
    #[rstest]
    #[case::focus_gained(Event::FocusGained)]
    #[case::focus_lost(Event::FocusLost)]
    #[case::key(Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL)))]
    #[case::mouse(Event::Mouse(MouseEvent {
        kind: MouseEventKind::Moved,
        column: 1,
        row: 2,
        modifiers: KeyModifiers::NONE,
    }))]
    #[case::paste(Event::Paste("pasted".into()))]
    #[case::resize(Event::Resize(80, 24))]
    fn output_type_accepts_event(#[case] event: Event, #[values(false, true)] add_raw: bool) {
        let value = parse_event(Span::test_data(), &event, &EventTypeFilter::all(), add_raw)
            .expect("every event type passes the `all` filter");
        for (_, output_ty) in &InputListen.signature().input_output_types {
            assert!(
                value.is_assignable_to(output_ty),
                "{value:?} is not a {output_ty}"
            );
        }
    }

    /// Every invalid `--types` value is an error before the terminal is touched. `[]` would
    /// otherwise listen for nothing and only return on `--timeout`.
    #[rstest]
    #[case::empty_list(Value::test_list(vec![]), "at least one event type")]
    #[case::not_a_list(Value::test_string("key"), "list of strings")]
    #[case::not_a_string(Value::test_list(vec![Value::test_int(1)]), "list of strings")]
    #[case::unknown_type(Value::test_list(vec![Value::test_string("bogus")]), "bogus is not a valid event type")]
    fn invalid_types_are_rejected(#[case] types: Value, #[case] expected: &str) {
        match EventTypeFilter::from_value(types, Span::test_data()) {
            Err(ShellError::UnsupportedInput { msg, .. }) => {
                assert!(msg.contains(expected), "{msg}")
            }
            Err(err) => panic!("unexpected error: {err:?}"),
            Ok(_) => panic!("invalid --types was accepted"),
        }
    }
}
