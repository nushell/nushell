use crate::context::Context;
use ansi_term::{Color, Style};
use log::log_enabled;
use nu_parser::{FlatShape, PipelineShape, ShapeResult, Token, TokensIterator};
use nu_protocol::{errln, outln};
use nu_source::{nom_input, HasSpan, Tag, Tagged, Text};
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
        let text = Text::from(line);
        self.context
            .shell_manager
            .hint(line, pos, ctx, self.context.expand_context(&text))
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
        let tokens = nu_parser::pipeline(nom_input(line));

        match tokens {
            Err(_) => Cow::Borrowed(line),
            Ok((_rest, v)) => {
                let pipeline = match v.as_pipeline() {
                    Err(_) => return Cow::Borrowed(line),
                    Ok(v) => v,
                };

                let text = Text::from(line);
                let expand_context = self.context.expand_context(&text);

                let tokens = vec![Token::Pipeline(pipeline).into_spanned(v.span())];
                let mut tokens = TokensIterator::new(&tokens[..], expand_context, v.span());

                let shapes = {
                    // We just constructed a token list that only contains a pipeline, so it can't fail
                    let result = tokens.expand_infallible(PipelineShape);

                    if let Some(failure) = result.failed {
                        errln!(
                            "BUG: PipelineShape didn't find a pipeline :: {:#?}",
                            failure
                        );
                    }

                    tokens.finish_tracer();

                    tokens.state().shapes()
                };

                if log_enabled!(target: "nu::expand_syntax", log::Level::Debug) {
                    outln!("");
                    let _ =
                        ptree::print_tree(&tokens.expand_tracer().clone().print(Text::from(line)));
                    outln!("");
                }

                let mut painter = Painter::new();

                for shape in shapes {
                    painter.paint_shape(&shape, line);
                }

                Cow::Owned(painter.into_string())
            }
        }
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

struct Painter {
    current: Style,
    buffer: String,
}

impl Painter {
    fn new() -> Painter {
        Painter {
            current: Style::default(),
            buffer: String::new(),
        }
    }

    fn into_string(self) -> String {
        self.buffer
    }

    fn paint_shape(&mut self, shape: &ShapeResult, line: &str) {
        let style = match &shape {
            ShapeResult::Success(shape) => match shape.item {
                FlatShape::OpenDelimiter(_) => Color::White.normal(),
                FlatShape::CloseDelimiter(_) => Color::White.normal(),
                FlatShape::ItVariable | FlatShape::Keyword => Color::Purple.bold(),
                FlatShape::Variable | FlatShape::Identifier => Color::Purple.normal(),
                FlatShape::Type => Color::Blue.bold(),
                FlatShape::CompareOperator => Color::Yellow.normal(),
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
                    let number = number.slice(line);
                    let unit = unit.slice(line);

                    self.paint(Color::Purple.bold(), number);
                    self.paint(Color::Cyan.bold(), unit);
                    return;
                }
            },
            ShapeResult::Fallback { shape, .. } => match shape.item {
                FlatShape::Whitespace | FlatShape::Separator => Color::White.normal(),
                _ => Style::new().fg(Color::White).on(Color::Red),
            },
        };

        self.paint(style, shape.span().slice(line));
    }

    fn paint(&mut self, style: Style, body: &str) {
        let infix = self.current.infix(style);
        self.current = style;
        self.buffer
            .push_str(&format!("{}{}", infix, style.paint(body)));
    }
}

impl rustyline::Helper for Helper {}

// Use default validator for normal single line behaviour
// In the future we can implement this for custom multi-line support
impl rustyline::validate::Validator for Helper {}
