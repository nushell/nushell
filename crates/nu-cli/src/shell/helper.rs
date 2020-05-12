use crate::context::Context;
use ansi_term::{Color, Style};
use nu_parser::SignatureRegistry;
use nu_protocol::hir::FlatShape;
use nu_source::{Span, Spanned, Tag, Tagged};
use rustyline::completion::Completer;
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use std::borrow::Cow::{self, Owned};

pub(crate) struct Helper {
    context: Context,
    pub colored_prompt: String,
}

impl Helper {
    pub(crate) fn new(context: Context) -> Helper {
        Helper {
            context,
            colored_prompt: String::new(),
        }
    }
}

impl Completer for Helper {
    type Candidate = rustyline::completion::Pair;
    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<rustyline::completion::Pair>), ReadlineError> {
        self.context.shell_manager.complete(line, pos, ctx)
    }
}

impl Hinter for Helper {
    fn hint(&self, line: &str, pos: usize, ctx: &rustyline::Context<'_>) -> Option<String> {
        self.context.shell_manager.hint(line, pos, ctx)
    }
}

impl Highlighter for Helper {
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
        Painter::paint_string(line, &self.context.registry().clone_box())
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

    pub fn paint_string<'l>(line: &'l str, registry: &dyn SignatureRegistry) -> Cow<'l, str> {
        let lite_block = nu_parser::lite_parse(line, 0);

        match lite_block {
            Err(_) => Cow::Borrowed(line),
            Ok(lb) => {
                let classified = nu_parser::classify_block(&lb, registry);

                let shapes = nu_parser::shapes(&classified.block);
                let mut painter = Painter::new(line);

                for shape in shapes {
                    painter.paint_shape(&shape);
                }

                Cow::Owned(painter.into_string())
            }
        }
    }

    fn paint_shape(&mut self, shape: &Spanned<FlatShape>) {
        let style = match &shape.item {
            FlatShape::OpenDelimiter(_) => Color::White.normal(),
            FlatShape::CloseDelimiter(_) => Color::White.normal(),
            FlatShape::ItVariable | FlatShape::Keyword => Color::Purple.bold(),
            FlatShape::Variable | FlatShape::Identifier => Color::Purple.normal(),
            FlatShape::Type => Color::Blue.bold(),
            FlatShape::Operator => Color::Yellow.normal(),
            FlatShape::DotDot => Color::Yellow.bold(),
            FlatShape::Dot => Style::new().fg(Color::White),
            FlatShape::InternalCommand => Color::Cyan.bold(),
            FlatShape::ExternalCommand => Color::Cyan.normal(),
            FlatShape::ExternalWord => Color::Green.bold(),
            FlatShape::BareMember => Color::Yellow.bold(),
            FlatShape::StringMember => Color::Yellow.bold(),
            FlatShape::String => Color::Green.normal(),
            FlatShape::Path => Color::Cyan.normal(),
            FlatShape::GlobPattern => Color::Cyan.bold(),
            FlatShape::Word => Color::Green.normal(),
            FlatShape::Pipe => Color::Purple.bold(),
            FlatShape::Flag => Color::Blue.bold(),
            FlatShape::ShorthandFlag => Color::Blue.bold(),
            FlatShape::Int => Color::Purple.bold(),
            FlatShape::Decimal => Color::Purple.bold(),
            FlatShape::Whitespace | FlatShape::Separator => Color::White.normal(),
            FlatShape::Comment => Color::Green.bold(),
            FlatShape::Garbage => Style::new().fg(Color::White).on(Color::Red),
            FlatShape::Size { number, unit } => {
                self.paint(Color::Purple.bold(), number);
                self.paint(Color::Cyan.bold(), unit);
                return;
            }
        };

        self.paint(style, &shape.span);
    }

    fn paint(&mut self, style: Style, span: &Span) {
        for pos in span.start()..span.end() {
            self.styles[pos] = style;
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
