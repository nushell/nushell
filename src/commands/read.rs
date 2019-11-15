use crate::commands::WholeStreamCommand;
use crate::data::Value;
use crate::errors::ShellError;
use crate::prelude::*;
use crossterm::{cursor, terminal, RawScreen};
use crossterm::{InputEvent, KeyEvent};
use std::io::{self, Write};

use crate::parser::registry::Signature;

#[derive(Deserialize)]
pub struct ReadArgs {}
pub struct Read;

impl WholeStreamCommand for Read {
    fn name(&self) -> &str {
        "read"
    }

    fn signature(&self) -> Signature {
        Signature::build("read")
    }

    fn usage(&self) -> &str {
        "Reads from standard input."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        Ok(args.process_raw(registry, read)?.run())
    }
}

fn read(
    ReadArgs {}: ReadArgs,
    context: RunnableContext,
    _raw: RawCommandArgs,
) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {

        let mut characters = String::new();

        if let Ok(_raw) = RawScreen::into_raw_mode() {
            let term = terminal();
            let cursor = cursor();

            println!("{}", ansi_term::Colour::Blue.paint("[Ctrl-C to end input]"));
            let (x, y) = cursor.pos();
            let _ = cursor.goto(0, y);

            let input = crossterm::input();
            let mut sync_stdin = input.read_sync();

            loop {
                if let Some(event) = sync_stdin.next() {
                    match event {
                        InputEvent::Keyboard(k) => match k {
                            KeyEvent::Ctrl(character) => {
                                if 'c' == character {
                                    println!("");
                                    let (x, y) = cursor.pos();
                                    let _ = cursor.goto(0, y);
                                    break;
                                }
                            }
                            KeyEvent::Backspace => {
                                characters.pop();

                                let (x, y) = cursor.pos();

                                let new_x = if x > 0 { x - 1 } else { x };
                                let _ = cursor.goto(new_x, y);
                                print!(" ");
                                let _ = cursor.goto(new_x, y);
                            }
                            KeyEvent::Char(character) => {
                                characters.push(character);

                                let (_, y) = cursor.pos();

                                if b"\n" == &[character as u8] {
                                    print!("{}", character);
                                    let _ = cursor.goto(0, y + 1);
                                } else {
                                    print!("{}", character);
                                }
                            }
                            _ => {}
                        },
                        _ => {}
                    }

                    let _ = io::stdout().flush();
                }
            }
            let _ = cursor.show();
        }

        yield Ok(ReturnSuccess::Value(Value::string(characters).tagged(&context.name)))
    };

    Ok(stream.to_output_stream())
}
