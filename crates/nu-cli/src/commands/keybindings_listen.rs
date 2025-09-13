use crossterm::{
    QueueableCommand, event::Event, event::KeyCode, event::KeyEvent, execute, terminal,
};
use nu_engine::command_prelude::*;
use nu_protocol::shell_error::io::IoError;
use std::io::{Write, stdout};

#[derive(Clone)]
pub struct KeybindingsListen;

impl Command for KeybindingsListen {
    fn name(&self) -> &str {
        "keybindings listen"
    }

    fn description(&self) -> &str {
        "Get input from the user."
    }

    fn extra_description(&self) -> &str {
        "This is an internal debugging tool. For better output, try `input listen --types [key]`"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Platform)
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .allow_variants_without_examples(true)
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
                terminal::disable_raw_mode().map_err(|err| {
                    IoError::new_internal(
                        err,
                        "Could not disable raw mode",
                        nu_protocol::location!(),
                    )
                })?;
                Err(ShellError::GenericError {
                    error: "Error with input".into(),
                    msg: "".into(),
                    span: None,
                    help: Some(e.to_string()),
                    inner: vec![],
                })
            }
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Type and see key event codes",
            example: "keybindings listen",
            result: None,
        }]
    }
}

pub fn print_events(engine_state: &EngineState) -> Result<Value, ShellError> {
    let config = engine_state.get_config();

    stdout().flush().map_err(|err| {
        IoError::new_internal(err, "Could not flush stdout", nu_protocol::location!())
    })?;
    terminal::enable_raw_mode().map_err(|err| {
        IoError::new_internal(err, "Could not enable raw mode", nu_protocol::location!())
    })?;

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
        //
        // Refer to https://sw.kovidgoyal.net/kitty/keyboard-protocol/ if you're curious.
        let _ = execute!(
            stdout(),
            crossterm::event::PushKeyboardEnhancementFlags(
                crossterm::event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
            )
        );
    }

    let mut stdout = std::io::BufWriter::new(std::io::stderr());

    loop {
        let event = crossterm::event::read().map_err(|err| {
            IoError::new_internal(err, "Could not read event", nu_protocol::location!())
        })?;
        if event == Event::Key(KeyCode::Esc.into()) {
            break;
        }
        // stdout.queue(crossterm::style::Print(format!("event: {:?}", &event)))?;
        // stdout.queue(crossterm::style::Print("\r\n"))?;

        // Get a record
        let v = print_events_helper(event)?;
        // Print out the record
        let o = match v {
            Value::Record { val, .. } => val
                .iter()
                .map(|(x, y)| format!("{}: {}", x, y.to_expanded_string("", config)))
                .collect::<Vec<String>>()
                .join(", "),

            _ => "".to_string(),
        };
        stdout.queue(crossterm::style::Print(o)).map_err(|err| {
            IoError::new_internal(
                err,
                "Could not print output record",
                nu_protocol::location!(),
            )
        })?;
        stdout
            .queue(crossterm::style::Print("\r\n"))
            .map_err(|err| {
                IoError::new_internal(err, "Could not print linebreak", nu_protocol::location!())
            })?;
        stdout.flush().map_err(|err| {
            IoError::new_internal(err, "Could not flush", nu_protocol::location!())
        })?;
    }

    if config.use_kitty_protocol {
        let _ = execute!(
            std::io::stdout(),
            crossterm::event::PopKeyboardEnhancementFlags
        );
    }

    terminal::disable_raw_mode().map_err(|err| {
        IoError::new_internal(err, "Could not disable raw mode", nu_protocol::location!())
    })?;

    Ok(Value::nothing(Span::unknown()))
}

// this fn is totally ripped off from crossterm's examples
// it's really a diagnostic routine to see if crossterm is
// even seeing the events. if you press a key and no events
// are printed, it's a good chance your terminal is eating
// those events.
fn print_events_helper(event: Event) -> Result<Value, ShellError> {
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
                Ok(Value::record(record, Span::unknown()))
            }
            _ => {
                let record = record! {
                    "code" => Value::string(format!("{code:?}"), Span::unknown()),
                    "modifier" => Value::string(format!("{modifiers:?}"), Span::unknown()),
                    "flags" => Value::string(format!("{modifiers:#08b}"), Span::unknown()),
                    "kind" => Value::string(format!("{kind:?}"), Span::unknown()),
                    "state" => Value::string(format!("{state:?}"), Span::unknown()),
                };
                Ok(Value::record(record, Span::unknown()))
            }
        }
    } else {
        let record = record! { "event" => Value::string(format!("{event:?}"), Span::unknown()) };
        Ok(Value::record(record, Span::unknown()))
    }
}
