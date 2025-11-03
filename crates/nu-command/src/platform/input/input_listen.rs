use crossterm::event::{
    DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
    EnableMouseCapture, KeyCode, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind,
};
use crossterm::{execute, terminal};
use nu_engine::command_prelude::*;

use nu_protocol::shell_error::io::IoError;
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
                "Listen for event of specified types only (can be one of: focus, key, mouse, paste, resize)",
                Some('t'),
            )
            .switch(
                "raw",
                "Add raw_code field with numeric value of keycode and raw_flags with bit mask flags",
                Some('r'),
            )
            .input_output_types(vec![(
                Type::Nothing,
                Type::Record([
                    ("keycode".to_string(), Type::String),
                    ("modifiers".to_string(), Type::List(Box::new(Type::String))),
                ].into()),
            )])
    }

    fn description(&self) -> &str {
        "Listen for user interface event."
    }

    fn extra_description(&self) -> &str {
        r#"There are 5 different type of events: focus, key, mouse, paste, resize. Each will produce a
corresponding record, distinguished by type field:
```
    { type: focus event: (gained|lost) }
    { type: key key_type: <key_type> code: <string> modifiers: [ <modifier> ... ] }
    { type: mouse col: <int> row: <int> kind: <string> modifiers: [ <modifier> ... ] }
    { type: paste content: <string> }
    { type: resize col: <int> row: <int> }
```
There are 6 `modifier` variants: shift, control, alt, super, hyper, meta.
There are 4 `key_type` variants:
    f - f1, f2, f3 ... keys
    char - alphanumeric and special symbols (a, A, 1, $ ...)
    media - dedicated media keys (play, pause, tracknext ...)
    other - keys not falling under previous categories (up, down, backspace, enter ...)"#
    }
    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Listen for a keyboard shortcut and find out how nu receives it",
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
        let add_raw = call.has_flag(engine_state, stack, "raw")?;
        let config = engine_state.get_config();

        terminal::enable_raw_mode().map_err(|err| IoError::new(err, head, None))?;

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
        }

        let console_state = event_type_filter.enable_events(head)?;
        loop {
            let event = crossterm::event::read().map_err(|_| ShellError::GenericError {
                error: "Error with user input".into(),
                msg: "".into(),
                span: Some(head),
                help: None,
                inner: vec![],
            })?;
            let event = parse_event(head, &event, &event_type_filter, add_raw);
            if let Some(event) = event {
                terminal::disable_raw_mode().map_err(|err| IoError::new(err, head, None))?;
                if config.use_kitty_protocol {
                    let _ = execute!(
                        std::io::stdout(),
                        crossterm::event::PopKeyboardEnhancementFlags
                    );
                }

                console_state.restore();
                return Ok(event.into_pipeline_data());
            }
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

#[derive(Clone)]
struct EventTypeFilter {
    listen_focus: bool,
    listen_key: bool,
    listen_mouse: bool,
    listen_paste: bool,
    listen_resize: bool,
}

impl EventTypeFilter {
    fn none() -> EventTypeFilter {
        EventTypeFilter {
            listen_focus: false,
            listen_key: false,
            listen_mouse: false,
            listen_paste: false,
            listen_resize: false,
        }
    }

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
        if let Value::List { vals, .. } = value {
            let mut filter = Self::none();
            for event_type in vals {
                let span = event_type.span();
                if let Value::String { val, .. } = event_type {
                    match val.as_str() {
                        "focus" => filter.listen_focus = true,
                        "key" => filter.listen_key = true,
                        "mouse" => filter.listen_mouse = true,
                        "paste" => filter.listen_paste = true,
                        "resize" => filter.listen_resize = true,
                        _ => return Err(Self::wrong_type_error(head, val.as_str(), span)),
                    }
                } else {
                    return Err(Self::bad_list_error(head, &event_type));
                }
            }
            Ok(filter)
        } else {
            Err(Self::bad_list_error(head, &value))
        }
    }

    fn wrong_type_error(head: Span, val: &str, val_span: Span) -> ShellError {
        ShellError::UnsupportedInput {
            msg: format!("{val} is not a valid event type"),
            input: "value originates from here".into(),
            msg_span: head,
            input_span: val_span,
        }
    }

    fn bad_list_error(head: Span, value: &Value) -> ShellError {
        ShellError::UnsupportedInput {
            msg: "--types expects a list of strings".to_string(),
            input: "value originates from here".into(),
            msg_span: head,
            input_span: value.span(),
        }
    }

    /// Enable capturing of all events allowed by this filter.
    /// Call [`DeferredConsoleRestore::restore`] when done capturing events to restore
    /// console state
    fn enable_events(&self, span: Span) -> Result<DeferredConsoleRestore, ShellError> {
        if self.listen_mouse {
            crossterm::execute!(stdout(), EnableMouseCapture)
                .map_err(|err| IoError::new(err, span, None))?;
        }

        if self.listen_paste {
            crossterm::execute!(stdout(), EnableBracketedPaste)
                .map_err(|err| IoError::new(err, span, None))?;
        }

        if self.listen_focus {
            crossterm::execute!(stdout(), crossterm::event::EnableFocusChange)
                .map_err(|err| IoError::new(err, span, None))?;
        }

        Ok(DeferredConsoleRestore {
            setup_event_types: self.clone(),
        })
    }
}

/// Promise to disable all event capturing previously enabled by [`EventTypeFilter::enable_events`]
struct DeferredConsoleRestore {
    setup_event_types: EventTypeFilter,
}

impl DeferredConsoleRestore {
    /// Disable all event capturing flags set up by [`EventTypeFilter::enable_events`]
    fn restore(self) {
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
