use crossterm::QueueableCommand;
use crossterm::{event::Event, event::KeyCode, event::KeyEvent, terminal};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Type, Value,
};
use std::io::{stdout, Write};
use crate::commands::keybindings_get::{EventTypeFilter, parse_event};

#[derive(Clone)]
pub struct KeybindingsListen;

impl Command for KeybindingsListen {
    fn name(&self) -> &str {
        "keybindings listen"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Platform)
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .allow_variants_without_examples(true)
    }

    fn usage(&self) -> &str {
        "Inspect keyboard events from user"
    }

    fn extra_usage(&self) -> &str {
        r#"Prints keyboard events in stderr.
To get keyboard events as a record use "keybindings get""#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        println!("Type any key combination to see key details. Press ESC to abort.");

        match print_events(engine_state) {
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
        vec![Example {
            description: "Type and see key event codes",
            example: "keybindings listen",
            result: None,
        }]
    }
}

pub fn print_events(engine_state: &EngineState) -> Result<Value, ShellError> {
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

        // Get a record
        let v = parse_event(Span::unknown(), &event, &EventTypeFilter::all());
        // Print out the record
        if let Some(v) = v {
            let o = match v {
                Value::Record { cols, vals, .. } => cols
                    .iter()
                    .zip(vals.iter())
                    .map(|(x, y)| format!("{}: {}", x, y.into_string(" ", config)))
                    .collect::<Vec<String>>()
                    .join(", "),

                _ => "".to_string(),
            };
            stdout.queue(crossterm::style::Print(o))?;
            stdout.queue(crossterm::style::Print("\r\n"))?;
            stdout.flush()?;
        }
    }
    terminal::disable_raw_mode()?;

    Ok(Value::nothing(Span::unknown()))
}
