use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, Signature, Span, Value,
};
use reedline::{
    get_reedline_edit_commands, get_reedline_keybinding_modifiers, get_reedline_keycodes,
    get_reedline_prompt_edit_modes, get_reedline_reedline_events,
};

#[derive(Clone)]
pub struct KeybindingsList;

impl Command for KeybindingsList {
    fn name(&self) -> &str {
        "keybindings list"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .switch("modifiers", "list of modifiers", Some('m'))
            .switch("keycodes", "list of keycodes", Some('k'))
            .switch("modes", "list of edit modes", Some('o'))
            .switch("events", "list of reedline event", Some('e'))
            .switch("edits", "list of edit commands", Some('d'))
            .category(Category::Platform)
    }

    fn usage(&self) -> &str {
        "List available options that can be used to create keybindings"
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get list of key modifiers",
                example: "keybindings list -m",
                result: None,
            },
            Example {
                description: "Get list of reedline events and edit commands",
                example: "keybindings list -e -d",
                result: None,
            },
            Example {
                description: "Get list with all the available options",
                example: "keybindings list",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let records = if call.named_len() == 0 {
            let all_options = vec!["modifiers", "keycodes", "edits", "modes", "events"];
            all_options
                .iter()
                .flat_map(|argument| get_records(argument, &call.head))
                .collect()
        } else {
            call.named_iter()
                .flat_map(|(argument, _, _)| get_records(argument.item.as_str(), &call.head))
                .collect()
        };

        Ok(Value::List {
            vals: records,
            span: call.head,
        }
        .into_pipeline_data())
    }
}

fn get_records(entry_type: &str, span: &Span) -> Vec<Value> {
    let values = match entry_type {
        "modifiers" => get_reedline_keybinding_modifiers().sorted(),
        "keycodes" => get_reedline_keycodes().sorted(),
        "edits" => get_reedline_edit_commands().sorted(),
        "modes" => get_reedline_prompt_edit_modes().sorted(),
        "events" => get_reedline_reedline_events().sorted(),
        _ => Vec::new(),
    };

    values
        .iter()
        .map(|edit| edit.split('\n'))
        .flat_map(|edit| edit.map(|edit| convert_to_record(edit, entry_type, span)))
        .collect()
}

fn convert_to_record(edit: &str, entry_type: &str, span: &Span) -> Value {
    let entry_type = Value::String {
        val: entry_type.to_string(),
        span: *span,
    };

    let name = Value::String {
        val: edit.to_string(),
        span: *span,
    };

    Value::Record {
        cols: vec!["type".to_string(), "name".to_string()],
        vals: vec![entry_type, name],
        span: *span,
    }
}

// Helper to sort a vec and return a vec
trait SortedImpl {
    fn sorted(self) -> Self;
}

impl<E> SortedImpl for Vec<E>
where
    E: std::cmp::Ord,
{
    fn sorted(mut self) -> Self {
        self.sort();
        self
    }
}
