use std::borrow::Cow::{self, Owned};

use ansi_term::{Color, Style};
use nu_parser::SignatureRegistry;
use nu_protocol::hir::FlatShape;
use nu_source::{Spanned, Tag, Tagged};

use crate::completion;
use crate::context::Context;
use crate::shell::completer::NuCompleter;
use crate::shell::palette::{DefaultPalette, Palette};

pub struct Helper {
    completer: NuCompleter,
    hinter: Option<rustyline::hint::HistoryHinter>,
    context: Context,
    pub colored_prompt: String,
    validator: NuValidator,
}

impl Helper {
    pub(crate) fn new(context: Context, hinter: Option<rustyline::hint::HistoryHinter>) -> Helper {
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
        let ctx = completion::Context::new(&self.context);
        Ok(self.completer.complete(line, pos, &ctx))
    }

    fn update(&self, line: &mut rustyline::line_buffer::LineBuffer, start: usize, elected: &str) {
        let end = (start + elected.len()).min(line.len());
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

pub struct Painter {
    original: Vec<u8>,
    styles: Vec<Style>,
}

impl Painter {
    fn new(original: &str) -> Painter {
        let bytes: Vec<u8> = original.bytes().collect();
        let bytes_count = bytes.len();
        Painter {
            original: bytes,
            styles: vec![Color::White.normal(); bytes_count],
        }
    }

    pub fn paint_string<'l, P: Palette>(
        line: &'l str,
        registry: &dyn SignatureRegistry,
        palette: &P,
    ) -> Cow<'l, str> {
        let lite_block = nu_parser::lite_parse(line, 0);

        match lite_block {
            Err(_) => Cow::Borrowed(line),
            Ok(lb) => {
                let classified = nu_parser::classify_block(&lb, registry);

                let shapes = nu_parser::shapes(&classified.block);
                let mut painter = Painter::new(line);

                for shape in shapes {
                    painter.paint_shape(&shape, palette);
                }

                Cow::Owned(painter.into_string())
            }
        }
    }

    fn paint_shape<P: Palette>(&mut self, shape: &Spanned<FlatShape>, palette: &P) {
        palette
            .styles_for_shape(shape)
            .iter()
            .for_each(|x| self.paint(x));
    }

    fn paint(&mut self, styled_span: &Spanned<Style>) {
        for pos in styled_span.span.start()..styled_span.span.end() {
            self.styles[pos] = styled_span.item;
        }
    }

    fn into_string(self) -> String {
        let mut idx_start = 0;
        let mut idx_end = 1;

        if self.original.is_empty() {
            String::new()
        } else {
            let mut builder = String::new();

            let mut current_style = self.styles[0];

            while idx_end < self.styles.len() {
                if self.styles[idx_end] != current_style {
                    // Emit, as we changed styles
                    let intermediate = String::from_utf8_lossy(&self.original[idx_start..idx_end]);

                    builder.push_str(&format!("{}", current_style.paint(intermediate)));

                    current_style = self.styles[idx_end];
                    idx_start = idx_end;
                    idx_end += 1;
                } else {
                    idx_end += 1;
                }
            }

            let intermediate = String::from_utf8_lossy(&self.original[idx_start..idx_end]);
            builder.push_str(&format!("{}", current_style.paint(intermediate)));

            builder
        }
    }
}

impl rustyline::Helper for Helper {}
