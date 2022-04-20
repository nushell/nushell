use crossterm::QueueableCommand;
use crossterm::{event::Event, event::KeyCode, event::KeyEvent, terminal};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Value,
};
use std::io::{stdout, Write};

#[derive(Clone)]
pub struct KeybindingsListen;

impl Command for KeybindingsListen {
    fn name(&self) -> &str {
        "keybindings listen"
    }

    fn usage(&self) -> &str {
        "Get input from the user."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Platform)
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
        let v = print_events_helper(event)?;
        // Print out the record
        let o = match v {
            Value::Record { cols, vals, .. } => cols
                .iter()
                .zip(vals.iter())
                .map(|(x, y)| format!("{}: {}", x, y.into_string("", config)))
                .collect::<Vec<String>>()
                .join(", "),

            _ => "".to_string(),
        };
        stdout.queue(crossterm::style::Print(o))?;
        stdout.queue(crossterm::style::Print("\r\n"))?;
        stdout.flush()?;
    }
    terminal::disable_raw_mode()?;

    Ok(Value::nothing(Span::test_data()))
}

// this fn is totally ripped off from crossterm's examples
// it's really a diagnostic routine to see if crossterm is
// even seeing the events. if you press a key and no events
// are printed, it's a good chance your terminal is eating
// those events.
fn print_events_helper(event: Event) -> Result<Value, ShellError> {
    if let Event::Key(KeyEvent { code, modifiers }) = event {
        match code {
            KeyCode::Char(c) => {
                let record = Value::Record {
                    cols: vec![
                        "char".into(),
                        "code".into(),
                        "modifier".into(),
                        "flags".into(),
                    ],
                    vals: vec![
                        Value::string(format!("{}", c), Span::test_data()),
                        Value::string(format!("{:#08x}", u32::from(c)), Span::test_data()),
                        Value::string(format!("{:?}", modifiers), Span::test_data()),
                        Value::string(format!("{:#08b}", modifiers), Span::test_data()),
                    ],
                    span: Span::test_data(),
                };
                Ok(record)
            }
            _ => {
                let record = Value::Record {
                    cols: vec!["code".into(), "modifier".into(), "flags".into()],
                    vals: vec![
                        Value::string(format!("{:?}", code), Span::test_data()),
                        Value::string(format!("{:?}", modifiers), Span::test_data()),
                        Value::string(format!("{:#08b}", modifiers), Span::test_data()),
                    ],
                    span: Span::test_data(),
                };
                Ok(record)
            }
        }
    } else {
        let record = Value::Record {
            cols: vec!["event".into()],
            vals: vec![Value::string(format!("{:?}", event), Span::test_data())],
            span: Span::test_data(),
        };
        Ok(record)
    }
}

#[cfg(test)]
mod tests {
    use crate::KeybindingsListen;

    #[test]
    fn examples_work_as_expected() {
        use crate::test_examples;
        test_examples(KeybindingsListen {})
    }
}
