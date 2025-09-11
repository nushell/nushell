use nu_engine::command_prelude::*;
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
            .input_output_types(vec![(Type::Nothing, Type::table())])
            .switch("modifiers", "list of modifiers", Some('m'))
            .switch("keycodes", "list of keycodes", Some('k'))
            .switch("modes", "list of edit modes", Some('o'))
            .switch("events", "list of reedline event", Some('e'))
            .switch("edits", "list of edit commands", Some('d'))
            .category(Category::Platform)
    }

    fn description(&self) -> &str {
        "List available options that can be used to create keybindings."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Get list of key modifiers",
                example: "keybindings list --modifiers",
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
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let all_options = ["modifiers", "keycodes", "edits", "modes", "events"];

        let presence = all_options
            .iter()
            .map(|option| call.has_flag(engine_state, stack, option))
            .collect::<Result<Vec<_>, ShellError>>()?;

        let no_option_specified = presence.iter().all(|present| !*present);

        let records = all_options
            .iter()
            .zip(presence)
            .filter(|(_, present)| no_option_specified || *present)
            .flat_map(|(option, _)| get_records(option, call.head))
            .collect();

        Ok(Value::list(records, call.head).into_pipeline_data())
    }
}

fn get_records(entry_type: &str, span: Span) -> Vec<Value> {
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

fn convert_to_record(edit: &str, entry_type: &str, span: Span) -> Value {
    Value::record(
        record! {
            "type" => Value::string(entry_type, span),
            "name" => Value::string(edit, span),
        },
        span,
    )
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
