use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct KeybindingsGet;

impl Command for KeybindingsGet {
    fn name(&self) -> &str {
        "keybindings get"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Platform)
            .named(
                "types",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "Listen for events of specified types only (can be one of: focus, key, mouse, paste, resize)",
                Some('t'),
            )
            .input_output_types(vec![(
                Type::Nothing,
                Type::Record(vec![
                    ("keycode".to_string(), Type::String),
                    ("modifiers".to_string(), Type::List(Box::new(Type::String))),
                ]),
            )])
    }

    fn usage(&self) -> &str {
        "Get keyboard event from user"
    }

    fn extra_usage(&self) -> &str {
        r#"
        There can be 5 different type of events: focus, key, mouse, paste, resize. Each will empit a
        corresponding record, distinguished by field type:
        {type: focus event: gained|lost}
        {type: resize columns: <int> rows: <int>}
        "#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let event_type_filter = call.get_flag::<Value>(engine_state, stack, "types")?;
        let event_type_filter = event_type_filter
            .map(|list| EventTypeFilter::from_value(list, head))
            .transpose()?
            .unwrap_or_else(|| EventTypeFilter::all());

        loop {
            let event = crossterm::event::read().map_err(|_| {
                ShellError::GenericError(
                    "Error with user input".to_string(),
                    "".to_string(),
                    Some(head),
                    None,
                    Vec::new(),
                )
            })?;
            let event = parse_event(head, &event, &event_type_filter);
            if let Some(event) = event {
                return Ok(event.into_pipeline_data());
            }
        }
    }
}

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
            for eventType in vals {
                if let Value::String { val, span } = eventType {
                    match val.as_str() {
                        "focus" => filter.listen_focus = true,
                        "key" => filter.listen_key = true,
                        "mouse" => filter.listen_mouse = true,
                        "paste" => filter.listen_paste = true,
                        "resize" => filter.listen_resize = true,
                        _ => return Err(Self::wrong_type_error(head, val.as_str(), span)),
                    }
                } else {
                    return Err(Self::bad_list_error(head, &eventType));
                }
            }
            Ok(filter)
        } else {
            Err(Self::bad_list_error(head, &value))
        }
    }

    fn wrong_type_error(head: Span, val: &str, val_span: Span) -> ShellError {
        ShellError::UnsupportedInput(
            format!("{} is not a valid event type", val),
            "value originates from here".into(),
            head,
            val_span,
        )
    }

    fn bad_list_error(head: Span, value: &Value) -> ShellError {
        ShellError::UnsupportedInput(
            "--types expects a list of strings".to_string(),
            "value originates from here".into(),
            head,
            value.span().unwrap_or_else(|_| head),
        )
    }
}

fn parse_event(
    head: Span,
    event: &crossterm::event::Event,
    filter: &EventTypeFilter,
) -> Option<Value> {
    match event {
        crossterm::event::Event::FocusGained => {
            create_focus_event(head, filter, FocusEventType::Gained)
        }
        crossterm::event::Event::FocusLost => {
            create_focus_event(head, filter, FocusEventType::Lost)
        }
        crossterm::event::Event::Key(event) => create_key_event(head, filter, event),
        crossterm::event::Event::Mouse(_) => None,
        crossterm::event::Event::Paste(_) => None,
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
        let cols = vec!["type".to_string(), "event".to_string()];
        let vals = vec![
            Value::string("focus", head),
            Value::string(event_type.string(), head),
        ];

        Some(Value::record(cols, vals, head))
    } else {
        None
    }
}

fn create_key_event(
    head: Span,
    filter: &EventTypeFilter,
    event: &crossterm::event::KeyEvent,
) -> Option<Value> {
    if filter.listen_key {
        let crossterm::event::KeyEvent {
            code,
            modifiers,
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

        let cols = vec!["type".to_string(), "key".to_string(), "modifiers".to_string()];

        let typ = Value::string("key".to_string(), head);
        let key = Value::string(get_keycode_name(code), head);
        let modifiers = parse_modifiers(head, modifiers);
        let vals = vec![typ, key, modifiers];

        Some(Value::record(cols, vals, head))
    } else {
        None
    }
}

fn get_keycode_name(code: &KeyCode) -> String {
    match code {
        KeyCode::F(n) => format!("f_{n}"),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Media(m) => format!("media_{m:?}").to_lowercase(),
        KeyCode::Modifier(m) => format!("modifier_{m:?}").to_lowercase(),
        _ => format!("{code:?}").to_lowercase(),
    }
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

    let parsed_modifiers = ALL_MODIFIERS.iter()
        .filter(|m| modifiers.contains(**m))
        .map(|m| format!("modifier_{m:?}").to_lowercase())
        .map(|string| Value::string(string, head))
        .collect();

    Value::list(parsed_modifiers, head)
}

fn create_resize_event(
    head: Span,
    filter: &EventTypeFilter,
    columns: u16,
    rows: u16,
) -> Option<Value> {
    if filter.listen_resize {
        let cols = vec![
            "type".to_string(),
            "columns".to_string(),
            "rows".to_string(),
        ];
        let vals = vec![
            Value::string("resize", head),
            Value::int(columns as i64, head),
            Value::int(rows as i64, head),
        ];

        Some(Value::record(cols, vals, head))
    } else {
        None
    }
}
