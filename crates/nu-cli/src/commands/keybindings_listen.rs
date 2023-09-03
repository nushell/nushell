use crossterm::QueueableCommand;
use crossterm::{event::Event, event::KeyCode, event::KeyEvent, terminal};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    record, Category, Example, IntoPipelineData, ParsedKeybinding, PipelineData, Record,
    ShellError, Signature, Span, Type, Value,
};
use std::io::{stdout, Write};

use crate::reedline_config::key_combination_to_parsed_keybinding;

#[derive(Clone)]
pub struct KeybindingsListen;

enum OutputStyle {
    Raw,
    Config,
}

impl Command for KeybindingsListen {
    fn name(&self) -> &str {
        "keybindings listen"
    }

    fn usage(&self) -> &str {
        "Get input from the user."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Platform)
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .switch(
                "generate-config",
                "generate output capable of being pasted into `config nu` keybindings section",
                Some('g'),
            )
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        println!("Type any key combination to see key details. Press ESC to abort.");
        let style = match call.has_flag("generate-config") {
            true => OutputStyle::Config,
            false => OutputStyle::Raw,
        };

        match print_events(engine_state, style) {
            Ok(v) => Ok(v.into_pipeline_data()),
            Err(e) => {
                terminal::disable_raw_mode()?;
                Err(ShellError::GenericError(
                    "Error with input".to_string(),
                    "".to_string(),
                    None,
                    Some(e.to_string()),
                    Vec::new(),
                ))
            }
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Type and see key event codes",
                example: "keybindings listen",
                result: None,
            },
            Example {
                description: "Generate code for putting into `config nu`",
                example: "keybindings listen -g; config nu",
                result: None,
            },
        ]
    }
}

fn print_events(engine_state: &EngineState, style: OutputStyle) -> Result<Value, ShellError> {
    let config = engine_state.get_config();

    stdout().flush()?;
    terminal::enable_raw_mode()?;
    let mut stdout = std::io::BufWriter::new(std::io::stderr());

    loop {
        let event = crossterm::event::read()?;
        if event == Event::Key(KeyCode::Esc.into()) {
            break;
        }
        // stdout.queue(crossterm::style::Print(format!("event: {:?}", &event)))?;
        // stdout.queue(crossterm::style::Print("\r\n"))?;

        let o = match style {
            OutputStyle::Raw => {
                // Get a record
                let val = print_events_raw_helper(event)?;
                // Print out the record
                val.iter()
                    .map(|(x, y)| format!("{}: {}", x, y.into_string("", config)))
                    .collect::<Vec<String>>()
                    .join(" ")
            }
            OutputStyle::Config => {
                let val = print_events_config_helper(event)?;
                val.map(
                    |ParsedKeybinding {
                         modifier,
                         keycode,
                         ..
                     }| {
                        Value::record(
                            record! {
                                "modifier" => modifier.clone(),
                                "keycode" => keycode.clone(),
                                "mode" => Value::string(config.edit_mode.clone(), Span::unknown()),
                                "event" => Value::record(record!{
                                    "edit" => Value::string("InsertString", Span::unknown()),
                                    "value" => Value::string("Add your action here", Span::unknown()),
                                }, Span::unknown()),
                            },
                            Span::unknown(),
                        )
                        // FIXME: this outputs a string like:
                        // {modifier: 'control' keycode: 'char_a' mode: 'emacs' event: {edit: 'InsertString', value: 'Add your action here'}}
                        // This works, but is a bit ugly. Is there a nushell autoformatter that we can pipe this through to remove the quotes etc?
                        .into_string_parsable(" ", config)
                    },
                )
                .unwrap_or_default()
            }
        };
        stdout.queue(crossterm::style::Print(o))?;
        stdout.queue(crossterm::style::Print("\r\n"))?;
        stdout.flush()?;
    }
    terminal::disable_raw_mode()?;

    Ok(Value::nothing(Span::unknown()))
}

// this fn is totally ripped off from crossterm's examples
// it's really a diagnostic routine to see if crossterm is
// even seeing the events. if you press a key and no events
// are printed, it's a good chance your terminal is eating
// those events.
fn print_events_raw_helper(event: Event) -> Result<Record, ShellError> {
    if let Event::Key(KeyEvent {
        code,
        modifiers,
        kind,
        state,
    }) = event
    {
        match code {
            KeyCode::Char(c) => {
                let record = record! {
                    "char" => Value::string(format!("{c}"), Span::unknown()),
                    "code" => Value::string(format!("{:#08x}", u32::from(c)), Span::unknown()),
                    "modifier" => Value::string(format!("{modifiers:?}"), Span::unknown()),
                    "flags" => Value::string(format!("{modifiers:#08b}"), Span::unknown()),
                    "kind" => Value::string(format!("{kind:?}"), Span::unknown()),
                    "state" => Value::string(format!("{state:?}"), Span::unknown()),
                };
                Ok(record)
            }
            _ => {
                let record = record! {
                    "code" => Value::string(format!("{code:?}"), Span::unknown()),
                    "modifier" => Value::string(format!("{modifiers:?}"), Span::unknown()),
                    "flags" => Value::string(format!("{modifiers:#08b}"), Span::unknown()),
                    "kind" => Value::string(format!("{kind:?}"), Span::unknown()),
                    "state" => Value::string(format!("{state:?}"), Span::unknown()),
                };
                Ok(record)
            }
        }
    } else {
        let record = record! { "event" => Value::string(format!("{event:?}"), Span::unknown()) };
        Ok(record)
    }
}

fn print_events_config_helper(event: Event) -> Result<Option<ParsedKeybinding>, ShellError> {
    if let Event::Key(KeyEvent {
        code, modifiers, ..
    }) = event
    {
        Ok(Some(key_combination_to_parsed_keybinding(modifiers, code)?))
    } else {
        Ok(None)
    }
}
