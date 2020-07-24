use crate::completion::{self, Completer};
use crate::context::Context;
use crate::shell::palette::{DefaultPalette, Palette};

use ansi_term::{Color, Style};
use nu_parser::SignatureRegistry;
use nu_protocol::hir::FlatShape;
use nu_source::{Spanned, Tag, Tagged};
use rustyline::hint::Hinter;
use std::borrow::Cow::{self, Owned};

pub struct Helper {
    completer: Box<dyn Completer>,
    context: Context,
    pub colored_prompt: String,
}

impl Helper {
    pub(crate) fn new(completer: Box<dyn Completer>, context: Context) -> Helper {
        Helper {
            completer,
            context,
            colored_prompt: String::new(),
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
        ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<Self::Candidate>), rustyline::error::ReadlineError> {
        let ctx = completion::Context::new(&self.context, ctx);
        self.completer
            .complete(line, pos, &ctx)
            .map_err(|_| rustyline::error::ReadlineError::Eof)
    }
}

impl Hinter for Helper {
    fn hint(&self, line: &str, pos: usize, ctx: &rustyline::Context<'_>) -> Option<String> {
        let ctx = completion::Context::new(&self.context, ctx);
        self.completer.hint(line, pos, &ctx)
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

// Use default validator for normal single line behaviour
// In the future we can implement this for custom multi-line support
impl rustyline::validate::Validator for Helper {}
