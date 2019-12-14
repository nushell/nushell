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
                let mut out = String::new();
                let pipeline = match v.as_pipeline() {
                    Err(_) => return Cow::Borrowed(line),
                    Ok(v) => v,
                };

                let text = Text::from(line);
                let expand_context = self.context.expand_context(&text);

                let tokens = vec![Token::Pipeline(pipeline.clone()).into_spanned(v.span())];
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
                    ptree::print_tree(&tokens.expand_tracer().clone().print(Text::from(line)))
                        .unwrap();
                    outln!("");
                }

                for shape in shapes {
                    let styled = paint_flat_shape(&shape, line);
                    out.push_str(&styled);
                }

                Cow::Owned(out)
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

fn paint_flat_shape(flat_shape: &ShapeResult, line: &str) -> String {
    let style = match &flat_shape {
        ShapeResult::Success(shape) => match shape.item {
            FlatShape::OpenDelimiter(_) | FlatShape::CloseDelimiter(_) => Color::White.normal(),
            FlatShape::ItVariable | FlatShape::Keyword => Color::Purple.bold(),
            FlatShape::Variable | FlatShape::Identifier => Color::Purple.normal(),
            FlatShape::Type => Color::Black.bold(),
            FlatShape::CompareOperator => Color::Yellow.normal(),
            FlatShape::DotDot => Color::Yellow.bold(),
            FlatShape::Dot => Color::White.normal(),
            FlatShape::InternalCommand => Color::Cyan.bold(),
            FlatShape::ExternalCommand => Color::Cyan.normal(),
            FlatShape::ExternalWord => Color::Black.bold(),
            FlatShape::BareMember | FlatShape::StringMember => Color::Yellow.bold(),
            FlatShape::String => Color::Green.normal(),
            FlatShape::Path => Color::Cyan.normal(),
            FlatShape::GlobPattern => Color::Purple.normal(),
            FlatShape::Word => Color::Green.normal(),
            FlatShape::Pipe => Color::Purple.bold(),
            FlatShape::Flag => Color::Black.bold(),
            FlatShape::ShorthandFlag => Color::Black.bold(),
            FlatShape::Int => Color::Purple.bold(),
            FlatShape::Decimal => Color::Purple.bold(),
            FlatShape::Whitespace | FlatShape::Separator => Color::White.normal(),
            FlatShape::Comment => Color::Black.bold(),
            FlatShape::Size { number, unit } => {
                let number = number.slice(line);
                let unit = unit.slice(line);
                return format!(
                    "{}{}",
                    Color::Purple.bold().paint(number),
                    Color::Cyan.bold().paint(unit)
                );
            }
        },
        ShapeResult::Fallback { .. } => Style::new().fg(Color::White).on(Color::Red),
    };

    let body = flat_shape.span().slice(line);
    style.paint(body).to_string()
}

impl rustyline::Helper for Helper {}
