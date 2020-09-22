use std::borrow::Cow::{self, Owned};

use nu_parser::SignatureRegistry;
use nu_source::{Tag, Tagged};

use crate::completion;
use crate::evaluation_context::EvaluationContext;
use crate::shell::completer::NuCompleter;
use crate::shell::painter::Painter;
use crate::shell::palette::DefaultPalette;

pub struct Helper {
    completer: NuCompleter,
    hinter: Option<rustyline::hint::HistoryHinter>,
    context: EvaluationContext,
    pub colored_prompt: String,
    validator: NuValidator,
}

impl Helper {
    pub(crate) fn new(
        context: EvaluationContext,
        hinter: Option<rustyline::hint::HistoryHinter>,
    ) -> Helper {
        Helper {
            completer: NuCompleter {},
            hinter,
            context,
            colored_prompt: String::new(),
            validator: NuValidator {},
        }
    }
}

impl rustyline::completion::Candidate for completion::Suggestion {
    fn display(&self) -> &str {
        &self.display
    }

    fn replacement(&self) -> &str {
        &self.replacement
    }
}

impl rustyline::completion::Completer for Helper {
    type Candidate = completion::Suggestion;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<Self::Candidate>), rustyline::error::ReadlineError> {
        let ctx = completion::CompletionContext::new(&self.context);
        Ok(self.completer.complete(line, pos, &ctx))
    }

    fn update(&self, line: &mut rustyline::line_buffer::LineBuffer, start: usize, elected: &str) {
        let end = line.pos();
        line.replace(start..end, elected)
    }
}

impl rustyline::hint::Hinter for Helper {
    fn hint(&self, line: &str, pos: usize, ctx: &rustyline::Context<'_>) -> Option<String> {
        self.hinter.as_ref().and_then(|h| h.hint(line, pos, &ctx))
    }
}

impl rustyline::highlight::Highlighter for Helper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        use std::borrow::Cow::Borrowed;

        if default {
            Borrowed(&self.colored_prompt)
        } else {
            Borrowed(prompt)
        }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
    }

    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        Painter::paint_string(
            line,
            &self.context.registry().clone_box(),
            &DefaultPalette {},
        )
    }

    fn highlight_char(&self, _line: &str, _pos: usize) -> bool {
        true
    }
}

impl rustyline::validate::Validator for Helper {
    fn validate(
        &self,
        ctx: &mut rustyline::validate::ValidationContext,
    ) -> rustyline::Result<rustyline::validate::ValidationResult> {
        self.validator.validate(ctx)
    }

    fn validate_while_typing(&self) -> bool {
        self.validator.validate_while_typing()
    }
}

struct NuValidator {}

impl rustyline::validate::Validator for NuValidator {
    fn validate(
        &self,
        ctx: &mut rustyline::validate::ValidationContext,
    ) -> rustyline::Result<rustyline::validate::ValidationResult> {
        let src = ctx.input();

        let lite_result = nu_parser::lite_parse(src, 0);

        if let Err(err) = lite_result {
            if let nu_errors::ParseErrorReason::Eof { .. } = err.cause.reason() {
                return Ok(rustyline::validate::ValidationResult::Incomplete);
            }
        }

        Ok(rustyline::validate::ValidationResult::Valid(None))
    }
}

#[allow(unused)]
fn vec_tag<T>(input: Vec<Tagged<T>>) -> Option<Tag> {
    let mut iter = input.iter();
    let first = iter.next()?.tag.clone();
    let last = iter.last();

    Some(match last {
        None => first,
        Some(last) => first.until(&last.tag),
    })
}

impl rustyline::Helper for Helper {}
