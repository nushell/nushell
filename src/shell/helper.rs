use crate::shell::completer::NuCompleter;

use rustyline::completion::{self, Completer, FilenameCompleter};
use rustyline::error::ReadlineError;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::{Hinter, HistoryHinter};
use std::borrow::Cow::{self, Owned};

crate struct Helper {
    completer: NuCompleter,
    highlighter: MatchingBracketHighlighter,
    hinter: HistoryHinter,
}

impl Helper {
    crate fn new() -> Helper {
        Helper {
            completer: NuCompleter {
                file_completer: FilenameCompleter::new(),
            },
            highlighter: MatchingBracketHighlighter::new(),
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
    fn highlight_prompt<'p>(&self, prompt: &'p str) -> Cow<'p, str> {
        Owned("\x1b[32m".to_owned() + &prompt[0..prompt.len() - 2] + "\x1b[m> ")
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
    }

    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize) -> bool {
        self.highlighter.highlight_char(line, pos)
    }
}

impl rustyline::Helper for Helper {}
