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

pub trait Palette {
    fn styles_for_shape(&self, shape: &Spanned<FlatShape>) -> Vec<Spanned<Style>>;
}

pub struct DefaultPalette {}

impl Palette for DefaultPalette {
    fn styles_for_shape(&self, shape: &Spanned<FlatShape>) -> Vec<Spanned<Style>> {
        match &shape.item {
            FlatShape::OpenDelimiter(_) => single_style_span(Color::White.normal(), shape.span),
            FlatShape::CloseDelimiter(_) => single_style_span(Color::White.normal(), shape.span),
            FlatShape::ItVariable | FlatShape::Keyword => {
                single_style_span(Color::Purple.bold(), shape.span)
            }
            FlatShape::Variable | FlatShape::Identifier => {
                single_style_span(Color::Purple.normal(), shape.span)
            }
            FlatShape::Type => single_style_span(Color::Blue.bold(), shape.span),
            FlatShape::Operator => single_style_span(Color::Yellow.normal(), shape.span),
            FlatShape::DotDot => single_style_span(Color::Yellow.bold(), shape.span),
            FlatShape::Dot => single_style_span(Style::new().fg(Color::White), shape.span),
            FlatShape::InternalCommand => single_style_span(Color::Cyan.bold(), shape.span),
            FlatShape::ExternalCommand => single_style_span(Color::Cyan.normal(), shape.span),
            FlatShape::ExternalWord => single_style_span(Color::Green.bold(), shape.span),
            FlatShape::BareMember => single_style_span(Color::Yellow.bold(), shape.span),
            FlatShape::StringMember => single_style_span(Color::Yellow.bold(), shape.span),
            FlatShape::String => single_style_span(Color::Green.normal(), shape.span),
            FlatShape::Path => single_style_span(Color::Cyan.normal(), shape.span),
            FlatShape::GlobPattern => single_style_span(Color::Cyan.bold(), shape.span),
            FlatShape::Word => single_style_span(Color::Green.normal(), shape.span),
            FlatShape::Pipe => single_style_span(Color::Purple.bold(), shape.span),
            FlatShape::Flag => single_style_span(Color::Blue.bold(), shape.span),
            FlatShape::ShorthandFlag => single_style_span(Color::Blue.bold(), shape.span),
            FlatShape::Int => single_style_span(Color::Purple.bold(), shape.span),
            FlatShape::Decimal => single_style_span(Color::Purple.bold(), shape.span),
            FlatShape::Whitespace | FlatShape::Separator => {
                single_style_span(Color::White.normal(), shape.span)
            }
            FlatShape::Comment => single_style_span(Color::Green.bold(), shape.span),
            FlatShape::Garbage => {
                single_style_span(Style::new().fg(Color::White).on(Color::Red), shape.span)
            }
            FlatShape::Size { number, unit } => vec![
                Spanned::<Style> {
                    span: *number,
                    item: Color::Purple.bold(),
                },
                Spanned::<Style> {
                    span: *unit,
                    item: Color::Cyan.bold(),
                },
            ],
        }
    }
}

fn single_style_span(style: Style, span: Span) -> Vec<Spanned<Style>> {
    vec![Spanned::<Style> { span, item: style }]
}
