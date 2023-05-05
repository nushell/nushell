use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape,
    Type, Value,
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
        crossterm::event::Event::FocusGained => None,
        crossterm::event::Event::FocusLost => None,
        crossterm::event::Event::Key(_) => None,
        crossterm::event::Event::Mouse(_) => None,
        crossterm::event::Event::Paste(_) => None,
        crossterm::event::Event::Resize(_, _) => None,
    }
}
