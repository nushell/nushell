use dialoguer::{
    console::{Style, Term},
    theme::ColorfulTheme,
    FuzzySelect,
};
use nu_ansi_term::Color;
use nu_completion::NuCompleter;
use nu_engine::{DefaultPalette, EvaluationContext, Painter};
use nu_source::{Tag, Tagged};
use std::borrow::Cow::{self, Owned};
use std::fmt::{self, Display, Formatter};

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

use nu_protocol::{SignatureRegistry, VariableRegistry};
struct CompletionContext<'a>(&'a EvaluationContext);

impl<'a> nu_completion::CompletionContext for CompletionContext<'a> {
    fn signature_registry(&self) -> &dyn SignatureRegistry {
        &self.0.scope
    }

    fn scope(&self) -> &dyn nu_parser::ParserScope {
        &self.0.scope
    }

    fn source(&self) -> &EvaluationContext {
        self.as_ref()
    }

    fn variable_registry(&self) -> &dyn VariableRegistry {
        self.0
    }
}

impl<'a> AsRef<EvaluationContext> for CompletionContext<'a> {
    fn as_ref(&self) -> &EvaluationContext {
        self.0
    }
}
#[derive(Clone)]
pub struct CompletionSuggestion(nu_completion::Suggestion);

impl Display for CompletionSuggestion {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.0.display)
    }
}

impl rustyline::completion::Candidate for CompletionSuggestion {
    fn display(&self) -> &str {
        &self.0.display
    }

    fn replacement(&self) -> &str {
        &self.0.replacement
    }
}

impl rustyline::completion::Completer for Helper {
    type Candidate = CompletionSuggestion;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<Self::Candidate>), rustyline::error::ReadlineError> {
        let ctx = CompletionContext(&self.context);
        let (position, suggestions) = self.completer.complete(line, pos, &ctx);
        let suggestions: Vec<CompletionSuggestion> =
            suggestions.into_iter().map(CompletionSuggestion).collect();

        match suggestions.len() {
            0 => return Ok((position, vec![])),
            1 => return Ok((position, suggestions)),
            _ => {}
        }

        //let _ = crossterm::terminal::disable_raw_mode();
        println!();
        let theme = ColorfulTheme {
            active_item_style: Style::new().for_stderr().on_green().black(),
            ..Default::default()
        };
        let result = FuzzySelect::with_theme(&theme)
            .default(0)
            .items(&suggestions[..])
            .interact_on_opt(&Term::stdout())
            .unwrap_or(None);
        //let _ = crossterm::terminal::enable_raw_mode();
        //
        match result {
            Some(index) => Ok((position, vec![suggestions[index].clone()])),
            _ => {
                // TODO seems dialog is not cleaned up correctly
                let term = &Term::stdout();
                term.clear_last_lines(1)?;
                term.flush()?;
                term.show_cursor()?;
                Ok((position, vec![]))
            }
        }
    }

    fn update(&self, line: &mut rustyline::line_buffer::LineBuffer, start: usize, elected: &str) {
        let end = line.pos();
        line.replace(start..end, elected)
    }
}

impl rustyline::hint::Hinter for Helper {
    type Hint = String;
    fn hint(&self, line: &str, pos: usize, ctx: &rustyline::Context<'_>) -> Option<String> {
        match &self.hinter {
            Some(the_hinter) => the_hinter.hint(line, pos, ctx),
            None => Some("".to_string()),
        }
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
        Owned(Color::DarkGray.prefix().to_string() + hint + nu_ansi_term::ansi::RESET)
    }

    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        let cfg = &self.context.configs().lock();
        if let Some(palette) = &cfg.syntax_config {
            Painter::paint_string(line, &self.context.scope, palette)
        } else {
            Painter::paint_string(line, &self.context.scope, &DefaultPalette {})
        }
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

        let (tokens, err) = nu_parser::lex(src, 0, nu_parser::NewlineMode::Normal);
        if let Some(err) = err {
            if let nu_errors::ParseErrorReason::Eof { .. } = err.reason() {
                return Ok(rustyline::validate::ValidationResult::Incomplete);
            }
        }

        let (_, err) = nu_parser::parse_block(tokens);

        if let Some(err) = err {
            if let nu_errors::ParseErrorReason::Eof { .. } = err.reason() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use nu_engine::EvaluationContext;
    use rustyline::completion::Completer;
    use rustyline::line_buffer::LineBuffer;

    #[ignore]
    #[test]
    fn closing_quote_should_replaced() {
        let text = "cd \"folder with spaces\\subdirectory\\\"";
        let replacement = "\"folder with spaces\\subdirectory\\subsubdirectory\\\"";

        let mut buffer = LineBuffer::with_capacity(256);
        buffer.insert_str(0, text);
        buffer.set_pos(text.len() - 1);

        let helper = Helper::new(EvaluationContext::basic(), None);

        helper.update(&mut buffer, "cd ".len(), replacement);

        assert_eq!(
            buffer.as_str(),
            "cd \"folder with spaces\\subdirectory\\subsubdirectory\\\""
        );
    }

    #[ignore]
    #[test]
    fn replacement_with_cursor_in_text() {
        let text = "cd \"folder with spaces\\subdirectory\\\"";
        let replacement = "\"folder with spaces\\subdirectory\\subsubdirectory\\\"";

        let mut buffer = LineBuffer::with_capacity(256);
        buffer.insert_str(0, text);
        buffer.set_pos(text.len() - 30);

        let helper = Helper::new(EvaluationContext::basic(), None);

        helper.update(&mut buffer, "cd ".len(), replacement);

        assert_eq!(
            buffer.as_str(),
            "cd \"folder with spaces\\subdirectory\\subsubdirectory\\\""
        );
    }
}
