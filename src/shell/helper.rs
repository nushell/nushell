use crate::shell::completer::NuCompleter;

use crate::parser::lexer::SpannedToken;
use crate::prelude::*;
use ansi_term::Color;
use log::trace;
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
    fn highlight_prompt<'b, 's: 'b, 'p:'b>(&'s self, prompt: &'p str, _: bool) -> Cow<'b, str> {
        Owned("\x1b[32m".to_owned() + &prompt[0..prompt.len() - 2] + "\x1b[m> ")
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
    }

    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        let tokens = crate::parser::lexer::Lexer::new(line, true);
        let tokens: Result<Vec<(usize, SpannedToken, usize)>, _> = tokens.collect();

        match tokens {
            Err(_) => Cow::Borrowed(line),
            Ok(v) => {
                let mut out = String::new();
                let mut iter = v.iter();

                let mut state = State::Command;

                loop {
                    match iter.next() {
                        None => return Cow::Owned(out),
                        Some((start, token, end)) => {
                            let (style, new_state) = token_style(&token, state);

                            trace!("token={:?}", token);
                            trace!("style={:?}", style);
                            trace!("new_state={:?}", new_state);

                            state = new_state;
                            let slice = &line[*start..*end];
                            let styled = style.paint(slice);
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

#[derive(Debug)]
enum State {
    Command,
    Flag,
    Var,
    Bare,
    None,
}

fn token_style(
    token: &crate::parser::lexer::SpannedToken,
    state: State,
) -> (ansi_term::Style, State) {
    use crate::parser::lexer::Token::*;

    match (state, &token.token) {
        (State::Command, Bare) => (Color::Cyan.bold(), State::None),
        (State::Command, Whitespace) => (Color::White.normal(), State::Command),

        (State::Flag, Bare) => (Color::Black.bold(), State::None),

        (State::Var, Variable) => (Color::Yellow.bold(), State::None),

        (State::Bare, PathDot) => (Color::Green.normal(), State::Bare),
        (State::Bare, Member) => (Color::Green.normal(), State::Bare),

        (_, Dash) | (_, DashDash) => (Color::Black.bold(), State::Flag),
        (_, Dollar) => (Color::Yellow.bold(), State::Var),
        (_, Bare) => (Color::Green.normal(), State::Bare),
        (_, Member) => (Color::Cyan.normal(), State::None),
        (_, Num) => (Color::Purple.bold(), State::None),
        (_, DQString) | (_, SQString) => (Color::Green.normal(), State::None),
        (_, Pipe) => (Color::White.normal(), State::Command),
        _ => (Color::White.normal(), State::None),
    }
}

impl rustyline::Helper for Helper {}
