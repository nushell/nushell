use crate::shell::completer::NuCompleter;

use crate::parser::nom_input;
use crate::prelude::*;
use ansi_term::Color;
use rustyline::completion::{self, Completer, FilenameCompleter};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::{Hinter, HistoryHinter};
use std::borrow::Cow::{self, Owned};

crate struct Helper {
    completer: NuCompleter,
    hinter: HistoryHinter,
}

impl Helper {
    crate fn new(commands: indexmap::IndexMap<String, Arc<dyn Command>>) -> Helper {
        Helper {
            completer: NuCompleter {
                file_completer: FilenameCompleter::new(),
                commands,
            },
            hinter: HistoryHinter {},
        }
    }
}

impl Completer for Helper {
    type Candidate = completion::Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<completion::Pair>), ReadlineError> {
        self.completer.complete(line, pos, ctx)
    }
}

impl Hinter for Helper {
    fn hint(&self, line: &str, pos: usize, ctx: &rustyline::Context<'_>) -> Option<String> {
        self.hinter.hint(line, pos, ctx)
    }
}

impl Highlighter for Helper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(&'s self, prompt: &'p str, _: bool) -> Cow<'b, str> {
        Owned("\x1b[32m".to_owned() + &prompt[0..prompt.len() - 2] + "\x1b[m> ")
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
    }

    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {

        let tokens = crate::parser::pipeline(nom_input(line));

        match tokens {
            Err(_) => Cow::Borrowed(line),
            Ok((_rest, v)) => {
                let mut out = String::new();
                let tokens = match v.as_pipeline() {
                    Err(_) => return Cow::Borrowed(line),
                    Ok(v) => v,
                };

                let mut iter = tokens.into_iter();

                match iter.next() {
                    None => return Cow::Owned(out),
                    Some(v) => out.push_str(v.span().slice(line)),
                };

                loop {
                    match iter.next() {
                        None => return Cow::Owned(out),
                        Some(token) => {
                            // let styled = token_style(&token, state);

                            // trace!("token={:?}", token);
                            // trace!("style={:?}", style);
                            // trace!("new_state={:?}", new_state);

                            // state = new_state;
                            // let slice = &line[*start..*end];
                            // let styled = style.paint(slice);
                            out.push_str("|");
                            let styled = Color::Black.bold().paint(token.span().slice(line));
                            out.push_str(&styled.to_string());
                        }
                    }
                }
            }
        }
    }

    fn highlight_char(&self, _line: &str, _pos: usize) -> bool {
        true
    }
}

impl rustyline::Helper for Helper {}
